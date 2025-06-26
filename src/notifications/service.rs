use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

use crate::account::database::{AccountDatabase, AccountDatabaseError};
use crate::notifications::gateway::{NotificationError, NotificationGateway};

pub struct NotificationService<D: AccountDatabase, G: NotificationGateway> {
    account_db: Arc<D>,
    notification_gateway: Arc<G>,
}

impl<D: AccountDatabase, G: NotificationGateway> NotificationService<D, G> {
    pub fn new(account_db: Arc<D>, notification_gateway: Arc<G>) -> Self {
        Self {
            account_db,
            notification_gateway,
        }
    }

    #[instrument(skip(self))]
    pub async fn send_wakeup_notification(
        &self,
        account_id: Uuid,
    ) -> Result<(), NotificationError> {
        // Fetch the account from the database
        let account = self
            .account_db
            .fetch_account(account_id)
            .await
            .map_err(|err| match err {
                AccountDatabaseError::NotFound(msg) => {
                    NotificationError::SendFailure(format!("Account not found: {}", msg))
                }
                AccountDatabaseError::OperationFailed => {
                    NotificationError::SendFailure("Database operation failed".to_string())
                }
            })?
            .ok_or_else(|| {
                NotificationError::SendFailure(format!("Account not found: {}", account_id))
            })?;

        // Extract the APNS token
        let apns_token = account.apns_token.ok_or_else(|| {
            NotificationError::SendFailure(format!(
                "APNS token not found for account: {}",
                account_id
            ))
        })?;

        // Send the notification
        self.notification_gateway
            .send_silent_notification(&apns_token)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::database::MockAccountDatabase;
    use crate::account::entities::Account;
    use crate::crypto::salted_hash::SaltedHash;
    use crate::notifications::gateway::MockNotificationGateway;
    use mockall::predicate::eq;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_send_wakeup_notification_success() {
        let account_id = Uuid::new_v4();
        let apns_token = vec![1, 2, 3, 4, 5];
        let account = Account {
            id: account_id,
            auth_password_hash: SaltedHash::generate_from("auth_password"),
            apns_token: Some(apns_token.clone()),
            gcm_token: None,
        };

        let mut mock_db = MockAccountDatabase::new();
        mock_db
            .expect_fetch_account()
            .once()
            .with(eq(account_id))
            .returning(move |_| Ok(Some(account.clone())));

        let mut mock_gateway = MockNotificationGateway::new();
        mock_gateway
            .expect_send_silent_notification()
            .once()
            .with(eq(apns_token))
            .returning(|_| Ok(()));

        let service = NotificationService::new(Arc::new(mock_db), Arc::new(mock_gateway));
        let result = service.send_wakeup_notification(account_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_wakeup_notification_account_not_found() {
        let account_id = Uuid::new_v4();

        let mut mock_db = MockAccountDatabase::new();
        mock_db
            .expect_fetch_account()
            .once()
            .with(eq(account_id))
            .returning(|_| Ok(None));

        let mock_gateway = MockNotificationGateway::new();

        let service = NotificationService::new(Arc::new(mock_db), Arc::new(mock_gateway));
        let result = service.send_wakeup_notification(account_id).await;

        assert!(matches!(result, Err(NotificationError::SendFailure(_))));
    }

    #[tokio::test]
    async fn test_send_wakeup_notification_apns_token_not_found() {
        let account_id = Uuid::new_v4();
        let account = Account {
            id: account_id,
            auth_password_hash: SaltedHash::generate_from("auth_password"),
            apns_token: None,
            gcm_token: None,
        };

        let mut mock_db = MockAccountDatabase::new();
        mock_db
            .expect_fetch_account()
            .once()
            .with(eq(account_id))
            .returning(move |_| Ok(Some(account.clone())));

        let mock_gateway = MockNotificationGateway::new();

        let service = NotificationService::new(Arc::new(mock_db), Arc::new(mock_gateway));
        let result = service.send_wakeup_notification(account_id).await;

        assert!(matches!(result, Err(NotificationError::SendFailure(_))));
    }

    #[tokio::test]
    async fn test_send_notification_database_error() {
        let account_id = Uuid::new_v4();

        let mut mock_db = MockAccountDatabase::new();
        mock_db
            .expect_fetch_account()
            .once()
            .with(eq(account_id))
            .returning(|_| Err(AccountDatabaseError::OperationFailed));

        let mock_gateway = MockNotificationGateway::new();

        let service = NotificationService::new(Arc::new(mock_db), Arc::new(mock_gateway));
        let result = service.send_wakeup_notification(account_id).await;

        assert!(matches!(result, Err(NotificationError::SendFailure(_))));
    }

    #[tokio::test]
    async fn test_send_wakeup_notification_gateway_error() {
        let account_id = Uuid::new_v4();
        let apns_token = vec![1, 2, 3, 4, 5];
        let account = Account {
            id: account_id,
            auth_password_hash: SaltedHash::generate_from("auth_password"),
            apns_token: Some(apns_token.clone()),
            gcm_token: None,
        };

        let mut mock_db = MockAccountDatabase::new();
        mock_db
            .expect_fetch_account()
            .once()
            .with(eq(account_id))
            .returning(move |_| Ok(Some(account.clone())));

        let mut mock_gateway = MockNotificationGateway::new();
        mock_gateway
            .expect_send_silent_notification()
            .once()
            .with(eq(apns_token))
            .returning(|_| Err(NotificationError::SendFailure("Network error".to_string())));

        let service = NotificationService::new(Arc::new(mock_db), Arc::new(mock_gateway));
        let result = service.send_wakeup_notification(account_id).await;

        assert!(matches!(result, Err(NotificationError::SendFailure(_))));
    }
}
