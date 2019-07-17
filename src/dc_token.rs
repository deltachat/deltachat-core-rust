use crate::context::Context;
use crate::dc_tools::*;
use crate::sql;
use crate::x::strdup;

// Token namespaces
pub type dc_tokennamespc_t = usize;
pub const DC_TOKEN_AUTH: dc_tokennamespc_t = 110;
pub const DC_TOKEN_INVITENUMBER: dc_tokennamespc_t = 100;

// Functions to read/write token from/to the database. A token is any string associated with a key.

pub fn dc_token_save(
    context: &Context,
    namespc: dc_tokennamespc_t,
    foreign_id: u32,
    token: *const libc::c_char,
) -> bool {
    if token.is_null() {
        return false;
    }
    // foreign_id may be 0
    sql::execute(
        context,
        &context.sql,
        "INSERT INTO tokens (namespc, foreign_id, token, timestamp) VALUES (?, ?, ?, ?);",
        params![namespc as i32, foreign_id as i32, as_str(token), time()],
    )
    .is_ok()
}

pub fn dc_token_lookup(
    context: &Context,
    namespc: dc_tokennamespc_t,
    foreign_id: u32,
) -> *mut libc::c_char {
    if let Some(token) = context.sql.query_row_col::<_, String>(
        context,
        "SELECT token FROM tokens WHERE namespc=? AND foreign_id=?;",
        params![namespc as i32, foreign_id as i32],
        0,
    ) {
        unsafe { strdup(to_cstring(token).as_ptr()) }
    } else {
        std::ptr::null_mut()
    }
}

pub fn dc_token_exists(
    context: &Context,
    namespc: dc_tokennamespc_t,
    token: *const libc::c_char,
) -> bool {
    if token.is_null() {
        return false;
    }

    context
        .sql
        .exists(
            "SELECT id FROM tokens WHERE namespc=? AND token=?;",
            params![namespc as i32, as_str(token)],
        )
        .unwrap_or_default()
}
