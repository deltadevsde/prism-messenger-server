pub struct RegistrationChallenge(pub Vec<u8>);

impl AsRef<[u8]> for RegistrationChallenge {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
