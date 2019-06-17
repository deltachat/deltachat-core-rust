use std::borrow::Cow;

use crate::constants::*;
use crate::context::Context;
use crate::dc_sqlite3::*;
use crate::key::*;
use crate::types::*;

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
        sql: &SQLite,
    ) -> bool {
        // Can we prevent keyring and self_addr to be null?
        if self_addr.is_null() {
            return false;
        }
        let stmt = unsafe {
            dc_sqlite3_prepare(
                context,
                sql,
                b"SELECT private_key FROM keypairs ORDER BY addr=? DESC, is_default DESC;\x00"
                    as *const u8 as *const libc::c_char,
            )
        };
        unsafe { sqlite3_bind_text(stmt, 1, self_addr, -1, None) };
        while unsafe { sqlite3_step(stmt) == 100 } {
            if let Some(key) = Key::from_stmt(stmt, 0, KeyType::Private) {
                self.add_owned(key);
            }
        }
        unsafe { sqlite3_finalize(stmt) };

        true
    }
}
