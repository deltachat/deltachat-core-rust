use crate::context::Context;
use crate::dc_sqlite3::*;
use crate::dc_tools::*;
use crate::types::*;

// Token namespaces
pub type dc_tokennamespc_t = libc::c_uint;
pub const DC_TOKEN_AUTH: dc_tokennamespc_t = 110;
pub const DC_TOKEN_INVITENUMBER: dc_tokennamespc_t = 100;
// Functions to read/write token from/to the database. A token is any string associated with a key.
pub unsafe fn dc_token_save(
    context: &Context,
    namespc: dc_tokennamespc_t,
    foreign_id: uint32_t,
    token: *const libc::c_char,
) {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !token.is_null() {
        // foreign_id may be 0
        stmt = dc_sqlite3_prepare(
            context,
            &context.sql.clone().read().unwrap(),
            b"INSERT INTO tokens (namespc, foreign_id, token, timestamp) VALUES (?, ?, ?, ?);\x00"
                as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, namespc as libc::c_int);
        sqlite3_bind_int(stmt, 2i32, foreign_id as libc::c_int);
        sqlite3_bind_text(stmt, 3i32, token, -1i32, None);
        sqlite3_bind_int64(stmt, 4i32, time() as sqlite3_int64);
        sqlite3_step(stmt);
    }
    sqlite3_finalize(stmt);
}
pub unsafe fn dc_token_lookup(
    context: &Context,
    namespc: dc_tokennamespc_t,
    foreign_id: uint32_t,
) -> *mut libc::c_char {
    let token: *mut libc::c_char;
    let stmt: *mut sqlite3_stmt;
    stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"SELECT token FROM tokens WHERE namespc=? AND foreign_id=?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, namespc as libc::c_int);
    sqlite3_bind_int(stmt, 2i32, foreign_id as libc::c_int);
    sqlite3_step(stmt);
    token = dc_strdup_keep_null(sqlite3_column_text(stmt, 0i32) as *mut libc::c_char);

    sqlite3_finalize(stmt);
    token
}

pub unsafe fn dc_token_exists(
    context: &Context,
    namespc: dc_tokennamespc_t,
    token: *const libc::c_char,
) -> libc::c_int {
    let mut exists: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !token.is_null() {
        stmt = dc_sqlite3_prepare(
            context,
            &context.sql.clone().read().unwrap(),
            b"SELECT id FROM tokens WHERE namespc=? AND token=?;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, namespc as libc::c_int);
        sqlite3_bind_text(stmt, 2i32, token, -1i32, None);
        exists = (sqlite3_step(stmt) != 0i32) as libc::c_int
    }
    sqlite3_finalize(stmt);
    return exists;
}
