use prism_client::SigningKey;
use prism_prover::Prover;
use std::sync::Arc;

use crate::{
    account::service::AccountService, database::inmemory::InMemoryDatabase,
    keys::service::KeyService, messages::service::MessagingService,
    registration::service::RegistrationService,
};

pub struct AppState {
    pub account_service: AccountService<Prover>,
    pub key_service: KeyService<Prover, InMemoryDatabase>,
    pub messaging_service: MessagingService<InMemoryDatabase>,
    pub registration_service: RegistrationService<Prover, InMemoryDatabase>,
}

impl AppState {
    pub fn new(prover: Arc<Prover>, signing_key: SigningKey) -> Self {
        let db = Arc::new(InMemoryDatabase::new());

        let account_service = AccountService::new(prover.clone());
        let registration_service =
            RegistrationService::new(prover.clone(), db.clone(), signing_key);
        let key_service = KeyService::new(prover.clone(), db.clone());
        let messaging_service = MessagingService::new(db.clone());

        Self {
            account_service,
            registration_service,
            key_service,
            messaging_service,
        }
    }
}
