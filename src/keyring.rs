use std::borrow::Cow;

use crate::constants::*;
use crate::context::Context;
use crate::key::*;
use crate::sql::Sql;

#[derive(Default, Clone, Debug)]
pub struct Keyring<'a> {
    keys: Vec<Cow<'a, Key>>,
}

impl<'a> Keyring<'a> {
    pub fn add_owned(&mut self, key: Key) {
        self.add(Cow::Owned(key))
    }

    pub fn add_ref(&mut self, key: &'a Key) {
        self.add(Cow::Borrowed(key))
    }

    fn add(&mut self, key: Cow<'a, Key>) {
        self.keys.push(key);
    }

    pub fn keys(&self) -> &[Cow<'a, Key>] {
        &self.keys
    }

    pub fn load_self_private_for_decrypting(
        &mut self,
        context: &Context,
        self_addr: impl AsRef<str>,
        sql: &Sql,
    ) -> bool {
        sql.query_get_value(
            context,
            "SELECT private_key FROM keypairs ORDER BY addr=? DESC, is_default DESC;",
            &[self_addr.as_ref()],
        )
        .and_then(|blob: Vec<u8>| Key::from_slice(&blob, KeyType::Private))
        .map(|key| self.add_owned(key))
        .is_some()
    }
}
