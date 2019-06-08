use std::borrow::Cow;

use crate::constants::*;
use crate::context::Context;
use crate::dc_sqlite3::*;
use crate::dc_tools::as_str;
use crate::key::*;

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
        self_addr: *const libc::c_char,
        sql: &dc_sqlite3_t,
    ) -> bool {
        // Can we prevent keyring and self_addr to be null?
        if self_addr.is_null() {
            return false;
        }

        dc_sqlite3_query_row(
            context,
            sql,
            "SELECT private_key FROM keypairs ORDER BY addr=? DESC, is_default DESC;",
            &[as_str(self_addr)],
            0,
        )
        .and_then(|blob| Key::from_slice(blob, KeyType::Private))
        .map(|key| self.add_owned(key))
        .is_some()
    }
}
