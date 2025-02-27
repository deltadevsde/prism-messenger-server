pub struct RegistrationChallenge(pub Vec<u8>);

impl RegistrationChallenge {
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl AsRef<[u8]> for RegistrationChallenge {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
