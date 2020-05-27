use anyhow::Result;

use crate::constants::KeyType;
use crate::context::Context;
use crate::key::Key;

#[derive(Default, Clone, Debug)]
pub struct Keyring {
    keys: Vec<Key>,
}

impl Keyring {
    pub fn add(&mut self, key: Key) {
        self.keys.push(key)
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn keys(&self) -> &[Key] {
        &self.keys
    }

    pub async fn load_self_private_for_decrypting(
        context: &Context,
        self_addr: impl AsRef<str>,
    ) -> Result<Self> {
        let blob: Vec<u8> = context
            .sql
            .query_get_value_result(
                "SELECT private_key FROM keypairs ORDER BY addr=? DESC, is_default DESC;",
                paramsv![self_addr.as_ref().to_string()],
            )
            .await?
            .unwrap_or_default();

        let key = async_std::task::spawn_blocking(move || Key::from_slice(&blob, KeyType::Private))
            .await?;

        Ok(Self { keys: vec![key] })
    }
}
