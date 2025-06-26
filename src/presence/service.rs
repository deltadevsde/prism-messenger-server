use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

use super::{database::PresenceDatabase, entities::PresenceStatus, error::PresenceError};

#[derive(Clone)]
pub struct PresenceService<D: PresenceDatabase> {
    presence_db: Arc<D>,
}

impl<D: PresenceDatabase> PresenceService<D> {
    pub fn new(presence_db: Arc<D>) -> Self {
        Self { presence_db }
    }
}

impl<D: PresenceDatabase> PresenceService<D> {
    #[instrument(skip(self))]
    pub async fn get_presence_status(
        &self,
        account_id: &Uuid,
    ) -> Result<PresenceStatus, PresenceError> {
        let is_present = self.presence_db.is_present(account_id).await?;
        Ok(PresenceStatus::from(is_present))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presence::database::MockPresenceDatabase;

    #[tokio::test]
    async fn test_get_presence_status_online() {
        let account_id = Uuid::new_v4();
        let mut mock_db = MockPresenceDatabase::new();

        mock_db
            .expect_is_present()
            .with(mockall::predicate::eq(account_id))
            .times(1)
            .returning(|_| Ok(true));

        let service = PresenceService::new(Arc::new(mock_db));
        let result = service.get_presence_status(&account_id).await;

        assert!(result.is_ok());
        matches!(result.unwrap(), PresenceStatus::Online);
    }

    #[tokio::test]
    async fn test_get_presence_status_offline() {
        let account_id = Uuid::new_v4();
        let mut mock_db = MockPresenceDatabase::new();

        mock_db
            .expect_is_present()
            .with(mockall::predicate::eq(account_id))
            .times(1)
            .returning(|_| Ok(false));

        let service = PresenceService::new(Arc::new(mock_db));
        let result = service.get_presence_status(&account_id).await;

        assert!(result.is_ok());
        matches!(result.unwrap(), PresenceStatus::Offline);
    }

    #[tokio::test]
    async fn test_get_presence_status_database_error() {
        let account_id = Uuid::new_v4();
        let mut mock_db = MockPresenceDatabase::new();

        mock_db
            .expect_is_present()
            .with(mockall::predicate::eq(account_id))
            .times(1)
            .returning(|_| Err(PresenceError::Database("Connection failed".to_string())));

        let service = PresenceService::new(Arc::new(mock_db));
        let result = service.get_presence_status(&account_id).await;

        assert!(result.is_err());
        matches!(result.unwrap_err(), PresenceError::Database(_));
    }
}
