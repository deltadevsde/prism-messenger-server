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
                username TEXT NOT NULL UNIQUE,
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
                username TEXT PRIMARY KEY,
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
                username TEXT NOT NULL,
                key_idx INTEGER NOT NULL,
                key BLOB NOT NULL,
                PRIMARY KEY (username, key_idx),
                FOREIGN KEY (username) REFERENCES key_bundles(username) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl AccountDatabase for SqliteDatabase {
    async fn upsert_account(&self, account: Account) -> Result<(), AccountDatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO accounts (id, username, auth_password_hash, apns_token, gcm_token)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                username = excluded.username,
                auth_password_hash = excluded.auth_password_hash,
                apns_token = excluded.apns_token,
                gcm_token = excluded.gcm_token
            "#,
        )
        .bind(account.id.to_string())
        .bind(&account.username)
        .bind(account.auth_password_hash.to_string())
        .bind(account.apns_token.as_deref())
        .bind(account.gcm_token.as_deref())
        .execute(&self.pool)
        .await
        .map_err(|_| AccountDatabaseError::OperationFailed)?;

        Ok(())
    }

    async fn fetch_account(&self, id: Uuid) -> Result<Account, AccountDatabaseError> {
        let row = sqlx::query(
            r#"
            SELECT id, username, auth_password_hash, apns_token, gcm_token
            FROM accounts
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AccountDatabaseError::OperationFailed)?;

        match row {
            Some(row) => {
                let salted_hash = row
                    .try_get("auth_password_hash")
                    .map_err(|_| AccountDatabaseError::OperationFailed)
                    .map(SaltedHash::new)?;

                let id_str: String = row
                    .try_get("id")
                    .map_err(|_| AccountDatabaseError::OperationFailed)?;
                let id =
                    Uuid::parse_str(&id_str).map_err(|_| AccountDatabaseError::OperationFailed)?;

                Ok(Account {
                    id,
                    username: row
                        .try_get("username")
                        .map_err(|_| AccountDatabaseError::OperationFailed)?,
                    auth_password_hash: salted_hash,
                    apns_token: row
                        .try_get("apns_token")
                        .map_err(|_| AccountDatabaseError::OperationFailed)?,
                    gcm_token: row
                        .try_get("gcm_token")
                        .map_err(|_| AccountDatabaseError::OperationFailed)?,
                })
            }
            None => Err(AccountDatabaseError::NotFound(id.to_string())),
        }
    }

    async fn fetch_account_by_username(
        &self,
        username: &str,
    ) -> Result<Account, AccountDatabaseError> {
        let row = sqlx::query(
            r#"
            SELECT id, username, auth_password_hash, apns_token, gcm_token
            FROM accounts
            WHERE username = ?
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AccountDatabaseError::OperationFailed)?;

        match row {
            Some(row) => {
                let salted_hash = row
                    .try_get("auth_password_hash")
                    .map_err(|_| AccountDatabaseError::OperationFailed)
                    .map(SaltedHash::new)?;

                let id_str: String = row
                    .try_get("id")
                    .map_err(|_| AccountDatabaseError::OperationFailed)?;
                let id =
                    Uuid::parse_str(&id_str).map_err(|_| AccountDatabaseError::OperationFailed)?;

                Ok(Account {
                    id,
                    username: row
                        .try_get("username")
                        .map_err(|_| AccountDatabaseError::OperationFailed)?,
                    auth_password_hash: salted_hash,
                    apns_token: row
                        .try_get("apns_token")
                        .map_err(|_| AccountDatabaseError::OperationFailed)?,
                    gcm_token: row
                        .try_get("gcm_token")
                        .map_err(|_| AccountDatabaseError::OperationFailed)?,
                })
            }
            None => Err(AccountDatabaseError::NotFound(username.to_string())),
        }
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
    async fn insert_keybundle(&self, user_id: &str, key_bundle: KeyBundle) -> Result<(), KeyError> {
        let mut tx = self.pool.begin().await?;

        // First, delete any existing key bundle and prekeys for this user
        sqlx::query("DELETE FROM key_bundles WHERE username = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // Serialize the keys to binary format
        let identity_key_bytes = key_bundle.identity_key.to_spki_der()?;
        let signed_prekey_bytes = key_bundle.signed_prekey.to_spki_der()?;
        let signature_bytes = key_bundle.signed_prekey_signature.to_prism_der()?;

        // Insert the new key bundle
        sqlx::query(
            r#"
            INSERT INTO key_bundles (username, identity_key, signed_prekey, signed_prekey_signature)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(user_id)
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
                INSERT INTO prekeys (username, key_idx, key)
                VALUES (?, ?, ?)
                "#,
            )
            .bind(user_id)
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

    async fn get_keybundle(&self, user_id: &str) -> Result<Option<KeyBundle>, KeyError> {
        // First, check if the key bundle exists
        let key_bundle_row = sqlx::query(
            r#"
            SELECT identity_key, signed_prekey, signed_prekey_signature
            FROM key_bundles
            WHERE username = ?
            "#,
        )
        .bind(user_id)
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
                WHERE username = ?
                "#,
            )
            .bind(user_id)
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

    async fn add_prekeys(&self, user_id: &str, prekeys: Vec<Prekey>) -> Result<(), KeyError> {
        // Check if the key bundle exists
        let exists = sqlx::query("SELECT COUNT(*) as count FROM key_bundles WHERE username = ?")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = exists.get("count");
        if count == 0 {
            return Err(KeyError::NotFound(user_id.to_string()));
        }

        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        // Insert the prekeys
        for prekey in &prekeys {
            let prekey_bytes = prekey.key.to_spki_der()?;

            sqlx::query(
                r#"
                INSERT INTO prekeys (username, key_idx, key)
                VALUES (?, ?, ?)
                ON CONFLICT(username, key_idx) DO UPDATE SET
                    key = excluded.key
                "#,
            )
            .bind(user_id)
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

#[cfg(test)]
mod tests {
    use super::*;
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
        let account = Account::new("testuser".to_string(), "password123", None, None);
        let account_id = account.id;

        // Test upsert_account
        db.upsert_account(account.clone())
            .await
            .expect("Failed to upsert account");

        // Test fetch_account
        let fetched = db
            .fetch_account(account_id)
            .await
            .expect("Failed to fetch account");
        assert_eq!(fetched.username, "testuser");

        // Test fetch_account_by_username
        let fetched_by_username = db
            .fetch_account_by_username("testuser")
            .await
            .expect("Failed to fetch account by username");
        assert_eq!(fetched_by_username.id, account_id);

        // Test remove_account
        db.remove_account(account_id)
            .await
            .expect("Failed to remove account");

        // Verify the account was removed
        let result = db.fetch_account(account_id).await;
        assert!(result.is_err());
        if let Err(AccountDatabaseError::NotFound(id)) = result {
            assert_eq!(id, account_id.to_string());
        } else {
            panic!("Expected AccountDatabaseError::NotFound");
        }
    }

    #[tokio::test]
    async fn test_update_apns_token() {
        let pool = create_test_pool().await;
        let db = SqliteDatabase::new(pool);
        db.init().await.expect("Failed to initialize database");

        // Create a test account
        let account = Account::new("apnsuser".to_string(), "password123", None, None);
        let account_id = account.id;

        // Insert the account
        db.upsert_account(account)
            .await
            .expect("Failed to upsert account");

        // Update the APNS token
        let new_token = vec![1, 2, 3, 4, 5];
        db.update_apns_token(account_id, new_token.clone())
            .await
            .expect("Failed to update APNS token");

        // Verify the token was updated
        let updated_account = db
            .fetch_account(account_id)
            .await
            .expect("Failed to fetch updated account");
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

    // Skipping key database test as it requires mocking prism_client types
    #[tokio::test]
    #[ignore]
    async fn test_key_database_operations() {
        // This test requires actual implementation details for the prism_client types
        // which would need to be mocked for proper testing
        // Test has been disabled with #[ignore]
    }
}
