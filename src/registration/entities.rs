pub struct RegistrationChallenge(pub Vec<u8>);

impl RegistrationChallenge {
    pub fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }
}

impl AsRef<[u8]> for RegistrationChallenge {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
