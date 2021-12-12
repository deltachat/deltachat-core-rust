//! Keyring to perform rpgp operations with.

use anyhow::Result;

use crate::context::Context;
use crate::key::DcKey;

/// An in-memory keyring.
///
/// Instances are usually constructed just for the rpgp operation and
/// short-lived.
#[derive(Clone, Debug, Default)]
pub struct Keyring<T>
where
    T: DcKey,
{
    keys: Vec<T>,
}

impl<T> Keyring<T>
where
    T: DcKey<KeyType = T>,
{
    /// New empty keyring.
    pub fn new() -> Keyring<T> {
        Keyring { keys: Vec::new() }
    }

    /// Create a new keyring with the the user's secret key loaded.
    pub async fn new_self(context: &Context) -> Result<Keyring<T>> {
        let mut keyring: Keyring<T> = Keyring::new();
        keyring.load_self(context).await?;
        Ok(keyring)
    }

    /// Load the user's key into the keyring.
    pub async fn load_self(&mut self, context: &Context) -> Result<()> {
        self.add(T::load_self(context).await?);
        Ok(())
    }

    /// Add a key to the keyring.
    pub fn add(&mut self, key: T) {
        self.keys.push(key);
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// A vector with reference to all the keys in the keyring.
    pub fn keys(&self) -> &[T] {
        &self.keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::{SignedPublicKey, SignedSecretKey};
    use crate::test_utils::{alice_keypair, TestContext};

    #[test]
    fn test_keyring_add_keys() {
        let alice = alice_keypair();
        let mut pub_ring: Keyring<SignedPublicKey> = Keyring::new();
        pub_ring.add(alice.public.clone());
        assert_eq!(pub_ring.keys(), [alice.public]);

        let mut sec_ring: Keyring<SignedSecretKey> = Keyring::new();
        sec_ring.add(alice.secret.clone());
        assert_eq!(sec_ring.keys(), [alice.secret]);
    }

    #[async_std::test]
    async fn test_keyring_load_self() {
        // new_self() implies load_self()
        let t = TestContext::new_alice().await;
        let alice = alice_keypair();

        let pub_ring: Keyring<SignedPublicKey> = Keyring::new_self(&t).await.unwrap();
        assert_eq!(pub_ring.keys(), [alice.public]);

        let sec_ring: Keyring<SignedSecretKey> = Keyring::new_self(&t).await.unwrap();
        assert_eq!(sec_ring.keys(), [alice.secret]);
    }
}
