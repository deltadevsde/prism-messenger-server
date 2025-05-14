use anyhow::Result;
use async_trait::async_trait;
use prism_client::{Signature, VerifyingKey};
use sqlx::{Acquire, Row, SqlitePool};
use uuid::Uuid;

use crate::account::database::{AccountDatabase, AccountDatabaseError};
use crate::account::entities::Account;
use crate::crypto::salted_hash::SaltedHash;
use crate::keys::database::KeyDatabase;
use crate::keys::entities::{KeyBundle, Prekey};
use crate::keys::error::KeyError;
use crate::profiles::database::ProfileDatabase;
use crate::profiles::entities::Profile;
use crate::profiles::error::ProfileError;

pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Creates the necessary tables if they don't exist
    pub async fn init(&self) -> Result<(), sqlx::Error> {
        // Create accounts table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                auth_password_hash TEXT NOT NULL,
                apns_token BLOB,
                gcm_token BLOB
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create key_bundles table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS key_bundles (
                account_id BLOB PRIMARY KEY,
                identity_key BLOB NOT NULL,
                signed_prekey BLOB NOT NULL,
                signed_prekey_signature BLOB NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create prekeys table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS prekeys (
                account_id BLOB NOT NULL,
                key_idx INTEGER NOT NULL,
                key BLOB NOT NULL,
                PRIMARY KEY (account_id, key_idx),
                FOREIGN KEY (account_id) REFERENCES key_bundles(account_id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create profiles table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL UNIQUE,
                username TEXT NOT NULL UNIQUE,
                display_name TEXT,
                profile_picture_url TEXT,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

// ACCOUNTS

impl From<sqlx::Error> for AccountDatabaseError {
    fn from(_: sqlx::Error) -> Self {
        AccountDatabaseError::OperationFailed
    }
}

impl From<uuid::Error> for AccountDatabaseError {
    fn from(_: uuid::Error) -> Self {
        AccountDatabaseError::OperationFailed
    }
}

#[async_trait]
impl AccountDatabase for SqliteDatabase {
    async fn upsert_account(&self, account: Account) -> Result<(), AccountDatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO accounts (id, auth_password_hash, apns_token, gcm_token)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                auth_password_hash = excluded.auth_password_hash,
                apns_token = excluded.apns_token,
                gcm_token = excluded.gcm_token
            "#,
        )
        .bind(account.id.to_string())
        .bind(account.auth_password_hash.to_string())
        .bind(account.apns_token.as_deref())
        .bind(account.gcm_token.as_deref())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn fetch_account(&self, id: Uuid) -> Result<Option<Account>, AccountDatabaseError> {
        let row = sqlx::query(
            r#"
            SELECT id, auth_password_hash, apns_token, gcm_token
            FROM accounts
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| {
            let salted_hash = row.try_get("auth_password_hash").map(SaltedHash::new)?;

            let id_str: String = row
                .try_get("id")
                .map_err(|_| AccountDatabaseError::OperationFailed)?;
            let id = Uuid::parse_str(&id_str).map_err(|_| AccountDatabaseError::OperationFailed)?;

            Ok(Account {
                id,
                auth_password_hash: salted_hash,
                apns_token: row.try_get("apns_token")?,
                gcm_token: row.try_get("gcm_token")?,
            })
        })
        .transpose()
    }

    async fn remove_account(&self, id: Uuid) -> Result<(), AccountDatabaseError> {
        sqlx::query(
            r#"
            DELETE FROM accounts
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|_| AccountDatabaseError::OperationFailed)?;

        Ok(())
    }

    async fn update_apns_token(
        &self,
        id: Uuid,
        token: Vec<u8>,
    ) -> Result<(), AccountDatabaseError> {
        let result = sqlx::query(
            r#"
            UPDATE accounts
            SET apns_token = ?
            WHERE id = ?
            "#,
        )
        .bind(token)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|_| AccountDatabaseError::OperationFailed)?;

        if result.rows_affected() == 0 {
            return Err(AccountDatabaseError::NotFound(id.to_string()));
        }

        Ok(())
    }
}

#[async_trait]
impl KeyDatabase for SqliteDatabase {
    async fn insert_keybundle(
        &self,
        account_id: Uuid,
        key_bundle: KeyBundle,
    ) -> Result<(), KeyError> {
        let mut tx = self.pool.begin().await?;

        // First, delete any existing key bundle and prekeys for this user
        sqlx::query("DELETE FROM key_bundles WHERE account_id = ?")
            .bind(account_id)
            .execute(&mut *tx)
            .await?;

        // Serialize the keys to binary format
        let identity_key_bytes = key_bundle.identity_key.to_spki_der()?;
        let signed_prekey_bytes = key_bundle.signed_prekey.to_spki_der()?;
        let signature_bytes = key_bundle.signed_prekey_signature.to_prism_der()?;

        // Insert the new key bundle
        sqlx::query(
            r#"
            INSERT INTO key_bundles (account_id, identity_key, signed_prekey, signed_prekey_signature)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(account_id)
        .bind(identity_key_bytes)
        .bind(signed_prekey_bytes)
        .bind(signature_bytes)
        .execute(&mut *tx)
        .await?;

        // Insert the prekeys
        for prekey in &key_bundle.prekeys {
            let prekey_bytes = prekey.key.to_spki_der()?;

            sqlx::query(
                r#"
                INSERT INTO prekeys (account_id, key_idx, key)
                VALUES (?, ?, ?)
                "#,
            )
            .bind(account_id)
            .bind(prekey.key_idx as i64) // SQLite uses i64 for INTEGER
            .bind(prekey_bytes)
            .execute(&mut *tx)
            .await?;
        }

        // Commit the transaction
        tx.commit()
            .await
            .map_err(|e| KeyError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn get_keybundle(&self, account_id: Uuid) -> Result<Option<KeyBundle>, KeyError> {
        // First, check if the key bundle exists
        let key_bundle_row = sqlx::query(
            r#"
            SELECT identity_key, signed_prekey, signed_prekey_signature
            FROM key_bundles
            WHERE account_id = ?
            "#,
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = key_bundle_row {
            // Get the identity key
            let identity_key_bytes: Vec<u8> = row.get("identity_key");
            let identity_key = VerifyingKey::from_spki_der(&identity_key_bytes)?;

            // Get the signed prekey
            let signed_prekey_bytes: Vec<u8> = row.get("signed_prekey");
            let signed_prekey = VerifyingKey::from_spki_der(&signed_prekey_bytes)?;

            // Get the signature
            let signature_bytes: Vec<u8> = row.get("signed_prekey_signature");
            let signed_prekey_signature = Signature::from_prism_der(&signature_bytes)?;

            // Get the prekeys
            let prekeys_rows = sqlx::query(
                r#"
                SELECT key_idx, key
                FROM prekeys
                WHERE account_id = ?
                "#,
            )
            .bind(account_id)
            .fetch_all(&self.pool)
            .await?;

            let mut prekeys = Vec::new();
            for row in prekeys_rows {
                let key_idx: i64 = row.get("key_idx");
                let key_bytes: Vec<u8> = row.get("key");
                let key = VerifyingKey::from_spki_der(&key_bytes)?;

                prekeys.push(Prekey {
                    key_idx: key_idx as u64,
                    key,
                });
            }

            Ok(Some(KeyBundle {
                identity_key,
                signed_prekey,
                signed_prekey_signature,
                prekeys,
            }))
        } else {
            Ok(None)
        }
    }

    async fn add_prekeys(&self, account_id: Uuid, prekeys: Vec<Prekey>) -> Result<(), KeyError> {
        // Check if the key bundle exists
        let exists = sqlx::query("SELECT COUNT(*) as count FROM key_bundles WHERE account_id = ?")
            .bind(account_id)
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = exists.get("count");
        if count == 0 {
            return Err(KeyError::NotFound(account_id.to_string()));
        }

        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        // Insert the prekeys
        for prekey in &prekeys {
            let prekey_bytes = prekey.key.to_spki_der()?;

            sqlx::query(
                r#"
                INSERT INTO prekeys (account_id, key_idx, key)
                VALUES (?, ?, ?)
                ON CONFLICT(account_id, key_idx) DO UPDATE SET
                    key = excluded.key
                "#,
            )
            .bind(account_id)
            .bind(prekey.key_idx as i64)
            .bind(prekey_bytes)
            .execute(&mut *tx)
            .await?;
        }

        // Commit the transaction
        tx.commit().await?;
        Ok(())
    }
}

impl From<sqlx::Error> for KeyError {
    fn from(e: sqlx::Error) -> Self {
        KeyError::DatabaseError(e.to_string())
    }
}

#[async_trait]
impl ProfileDatabase for SqliteDatabase {
    async fn get_profile_by_id(&self, id: Uuid) -> Result<Option<Profile>, ProfileError> {
        let row = sqlx::query(
            r#"
            SELECT id, account_id, username, display_name, profile_picture_url, updated_at
            FROM profiles
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let id_str: String = row.try_get("id")?;
                let id =
                    Uuid::parse_str(&id_str).map_err(|e| ProfileError::Internal(e.to_string()))?;

                let account_id_str: String = row.try_get("account_id")?;
                let account_id = Uuid::parse_str(&account_id_str)
                    .map_err(|e| ProfileError::Internal(e.to_string()))?;

                Ok(Some(Profile {
                    id,
                    account_id,
                    username: row.try_get("username")?,
                    display_name: row.try_get("display_name")?,
                    profile_picture_url: row.try_get("profile_picture_url")?,
                    updated_at: row.try_get::<i64, _>("updated_at")? as u64,
                }))
            }
            None => Ok(None),
        }
    }

    async fn get_profile_by_account_id(
        &self,
        account_id: Uuid,
    ) -> Result<Option<Profile>, ProfileError> {
        let row = sqlx::query(
            r#"
            SELECT id, account_id, username, display_name, profile_picture_url, updated_at
            FROM profiles
            WHERE account_id = ?
            "#,
        )
        .bind(account_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let id_str: String = row.try_get("id")?;
                let id =
                    Uuid::parse_str(&id_str).map_err(|e| ProfileError::Internal(e.to_string()))?;

                let account_id_str: String = row.try_get("account_id")?;
                let account_id = Uuid::parse_str(&account_id_str)
                    .map_err(|e| ProfileError::Internal(e.to_string()))?;

                Ok(Some(Profile {
                    id,
                    account_id,
                    username: row.try_get("username")?,
                    display_name: row.try_get("display_name")?,
                    profile_picture_url: row.try_get("profile_picture_url")?,
                    updated_at: row.try_get::<i64, _>("updated_at")? as u64,
                }))
            }
            None => Ok(None),
        }
    }

    async fn upsert_profile(&self, profile: Profile) -> Result<(), ProfileError> {
        sqlx::query(
            r#"
            INSERT INTO profiles (id, account_id, username, display_name, profile_picture_url, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                account_id = excluded.account_id,
                username = excluded.username,
                display_name = excluded.display_name,
                profile_picture_url = excluded.profile_picture_url,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(profile.id.to_string())
        .bind(&profile.account_id.to_string())
        .bind(&profile.username)
        .bind(&profile.display_name)
        .bind(&profile.profile_picture_url)
        .bind(profile.updated_at as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_profile_by_username(
        &self,
        username: &str,
    ) -> Result<Option<Profile>, ProfileError> {
        let row = sqlx::query(
            r#"
            SELECT id, account_id, username, display_name, profile_picture_url, updated_at
            FROM profiles
            WHERE username = ?
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let id_str: String = row.try_get("id")?;
                let id =
                    Uuid::parse_str(&id_str).map_err(|e| ProfileError::Internal(e.to_string()))?;

                let account_id_str: String = row.try_get("account_id")?;
                let account_id = Uuid::parse_str(&account_id_str)
                    .map_err(|e| ProfileError::Internal(e.to_string()))?;

                Ok(Some(Profile {
                    id,
                    account_id,
                    username: row.try_get("username")?,
                    display_name: row.try_get("display_name")?,
                    profile_picture_url: row.try_get("profile_picture_url")?,
                    updated_at: row.try_get::<i64, _>("updated_at")? as u64,
                }))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_client::SigningKey;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn create_test_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .connect(":memory:")
            .await
            .expect("Failed to connect to in-memory SQLite")
    }

    #[tokio::test]
    async fn test_account_crud_operations() {
        let pool = create_test_pool().await;
        let db = SqliteDatabase::new(pool);
        db.init().await.expect("Failed to initialize database");

        // Create a test account
        let account = Account::new("password123", Some(b"test_apns_token".to_vec()), None);
        let account_id = account.id;

        // Test upsert_account
        db.upsert_account(account.clone())
            .await
            .expect("Failed to upsert account");

        // Test fetch_account
        let fetched = db
            .fetch_account(account_id)
            .await
            .expect("Failed to fetch account")
            .expect("Account should exist");
        assert_eq!(fetched.apns_token, Some(b"test_apns_token".to_vec()));

        // Test remove_account
        db.remove_account(account_id)
            .await
            .expect("Failed to remove account");

        // Verify the account was removed
        let result = db
            .fetch_account(account_id)
            .await
            .expect("Failed to fetch account");
        assert!(
            result.is_none(),
            "Account result should be None after deletion"
        );
    }

    #[tokio::test]
    async fn test_update_apns_token() {
        let pool = create_test_pool().await;
        let db = SqliteDatabase::new(pool);
        db.init().await.expect("Failed to initialize database");

        // Create a test account
        let account = Account::new("password123", None, None);
        let account_id = account.id;

        db.upsert_account(account)
            .await
            .expect("Failed to insert account");

        // Update the APNS token
        let new_token = vec![1, 2, 3, 4, 5];
        db.update_apns_token(account_id, new_token.clone())
            .await
            .expect("Failed to update APNS token");

        // Verify the token was updated
        let updated_account = db
            .fetch_account(account_id)
            .await
            .expect("Failed to fetch updated account")
            .expect("Account should exist");
        assert_eq!(updated_account.apns_token, Some(new_token));

        // Test updating non-existent account
        let non_existent_id = Uuid::new_v4();
        let result = db
            .update_apns_token(non_existent_id, vec![5, 6, 7, 8])
            .await;
        assert!(result.is_err());
        if let Err(AccountDatabaseError::NotFound(id)) = result {
            assert_eq!(id, non_existent_id.to_string());
        } else {
            panic!("Expected AccountDatabaseError::NotFound");
        }
    }

    #[tokio::test]
    async fn test_key_database_operations() {
        let pool = create_test_pool().await;
        let db = SqliteDatabase::new(pool);
        db.init().await.expect("Failed to initialize database");

        let account_id = Uuid::new_v4();

        // Create a test key bundle
        let identity_signing_key = SigningKey::new_ed25519();
        let identity_key = identity_signing_key.verifying_key();
        let signed_prekey = SigningKey::new_ed25519().verifying_key();
        let signed_prekey_signature = identity_signing_key
            .sign(signed_prekey.to_spki_der().unwrap())
            .unwrap();
        let prekey1 = Prekey {
            key_idx: 1,
            key: SigningKey::new_ed25519().verifying_key(),
        };
        let prekey2 = Prekey {
            key_idx: 2,
            key: SigningKey::new_ed25519().verifying_key(),
        };

        let key_bundle = KeyBundle {
            identity_key: identity_key.clone(),
            signed_prekey: signed_prekey.clone(),
            signed_prekey_signature: signed_prekey_signature.clone(),
            prekeys: vec![prekey1.clone(), prekey2.clone()],
        };

        // Test inserting a key bundle
        db.insert_keybundle(account_id, key_bundle)
            .await
            .expect("Failed to insert key bundle");

        // Test retrieving the key bundle
        let retrieved_bundle = db
            .get_keybundle(account_id)
            .await
            .expect("Failed to get key bundle")
            .expect("Key bundle should exist");

        // Directly compare the cryptographic types
        assert_eq!(&retrieved_bundle.identity_key, &identity_key);
        assert_eq!(retrieved_bundle.signed_prekey, signed_prekey);
        assert_eq!(
            retrieved_bundle.signed_prekey_signature,
            signed_prekey_signature
        );
        assert_eq!(retrieved_bundle.prekeys.len(), 2);
        assert_eq!(retrieved_bundle.prekeys[0], prekey1);
        assert_eq!(retrieved_bundle.prekeys[1], prekey2);

        // Test adding additional prekeys
        let prekey3 = Prekey {
            key_idx: 3,
            key: SigningKey::new_ed25519().verifying_key(),
        };
        let prekey4 = Prekey {
            key_idx: 4,
            key: SigningKey::new_ed25519().verifying_key(),
        };

        let additional_prekeys = vec![prekey3.clone(), prekey4.clone()];

        db.add_prekeys(account_id, additional_prekeys)
            .await
            .expect("Failed to add prekeys");

        // Verify the updated bundle has all prekeys
        let updated_bundle = db
            .get_keybundle(account_id)
            .await
            .expect("Failed to get updated key bundle")
            .expect("Updated key bundle should exist");

        // Check the number of prekeys
        assert_eq!(updated_bundle.prekeys.len(), 4);
        assert_eq!(updated_bundle.prekeys[2], prekey3);
        assert_eq!(updated_bundle.prekeys[3], prekey4);

        // Test adding prekeys for non-existent user
        let non_existent_account_id = Uuid::new_v4();
        let result = db
            .add_prekeys(
                non_existent_account_id,
                vec![Prekey {
                    key_idx: 1,
                    key: SigningKey::new_ed25519().verifying_key(),
                }],
            )
            .await;

        assert!(result.is_err());
        if let Err(KeyError::NotFound(user)) = result {
            assert_eq!(user, non_existent_account_id.to_string());
        } else {
            panic!("Expected KeyError::NotFound");
        }

        // Test getting a non-existent key bundle
        let non_existent_bundle = db
            .get_keybundle(non_existent_account_id)
            .await
            .expect("get_keybundle should not fail for non-existent user");

        assert!(non_existent_bundle.is_none(), "Bundle should not exist");
    }

    #[tokio::test]
    async fn test_profile_database_operations() {
        let pool = create_test_pool().await;
        let db = SqliteDatabase::new(pool);
        db.init().await.expect("Failed to initialize database");

        // First create a test account (needed due to foreign key constraint)
        let username = "profileuser";
        let account = Account::new("password123", None, None);
        let account_id = account.id;

        db.upsert_account(account)
            .await
            .expect("Failed to create account");

        // Create a test profile
        let profile = Profile::new(account_id, username.to_string());
        // Override with test values
        let profile = Profile {
            id: profile.id,
            account_id,
            username: username.to_string(),
            display_name: Some("Test User".to_string()),
            profile_picture_url: Some("https://example.com/image.jpg".to_string()),
            updated_at: 1234567890,
        };

        db.upsert_profile(profile.clone())
            .await
            .expect("Failed to insert profile");

        // Test get_profile_by_id
        let fetched_by_id = db
            .get_profile_by_id(profile.id)
            .await
            .expect("Failed to get profile by ID")
            .expect("Profile should exist");

        assert_eq!(fetched_by_id.id, profile.id);
        assert_eq!(fetched_by_id.account_id, profile.account_id);
        assert_eq!(fetched_by_id.display_name, profile.display_name);
        assert_eq!(
            fetched_by_id.profile_picture_url,
            profile.profile_picture_url
        );
        assert_eq!(fetched_by_id.updated_at, profile.updated_at);

        // Test get_profile_by_account_id
        let fetched_by_account_id = db
            .get_profile_by_account_id(account_id)
            .await
            .expect("Failed to get profile by account_id")
            .expect("Profile should exist");

        assert_eq!(fetched_by_account_id.id, profile.id);

        // Test profile update
        let mut updated_profile = Profile::new(account_id, username.to_string());
        updated_profile.id = profile.id; // Use the same ID for update
        updated_profile.display_name = Some("Updated Name".to_string());
        updated_profile.profile_picture_url = None;
        updated_profile.updated_at = 9876543210;

        db.upsert_profile(updated_profile.clone())
            .await
            .expect("Failed to update profile");

        let fetched_updated = db
            .get_profile_by_id(profile.id)
            .await
            .expect("Failed to get updated profile")
            .expect("Updated profile should exist");

        assert_eq!(
            fetched_updated.display_name,
            Some("Updated Name".to_string())
        );
        assert_eq!(fetched_updated.profile_picture_url, None);
        assert_eq!(fetched_updated.updated_at, 9876543210);
        assert_eq!(
            fetched_updated.account_id, account_id,
            "account_id should match the original account"
        );

        // Test get non-existent profile
        let non_existent_id = Uuid::new_v4();
        let non_existent_result = db
            .get_profile_by_id(non_existent_id)
            .await
            .expect("get_profile_by_id should not fail for non-existent profile");

        assert!(non_existent_result.is_none(), "Profile should not exist");

        let non_existent_account_id = Uuid::new_v4();
        let non_existent_account_result = db
            .get_profile_by_account_id(non_existent_account_id)
            .await
            .expect("get_profile_by_account_id should not fail for non-existent profile");

        assert!(
            non_existent_account_result.is_none(),
            "Profile should not exist"
        );

        // Test cascade delete - check that deleting an account removes the profile
        db.remove_account(account_id)
            .await
            .expect("Failed to remove account");

        let profile_after_account_deletion = db
            .get_profile_by_id(account_id)
            .await
            .expect("get_profile_by_id should not fail after account deletion");

        assert!(
            profile_after_account_deletion.is_none(),
            "Profile should be deleted when account is deleted (due to CASCADE constraint)"
        );
    }
}
