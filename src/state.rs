use std::sync::Arc;

use prism_keys::SigningKey;
use prism_prover::Prover;

use crate::{account::service::AccountService, registration::service::RegistrationService};

pub struct AppState {
    pub account_service: AccountService,
    pub registration_service: RegistrationService<Prover>,
}

impl AppState {
    pub fn new(prover: Arc<Prover>, signing_key: SigningKey) -> Self {
        let account_service = AccountService::new(prover.clone());
        let registration_service = RegistrationService::new(prover.clone(), signing_key);

        Self {
            account_service,
            registration_service,
        }
    }
}
