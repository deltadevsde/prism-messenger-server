use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::account::database::{AccountDatabase, AccountDatabaseError};
use crate::account::entities::Account;
use crate::crypto::salted_hash::SaltedHash;

pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Creates the accounts table if it doesn't exist
    pub async fn init(&self) -> Result<(), sqlx::Error> {
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
        .await
        .map(|_| ())
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
}
