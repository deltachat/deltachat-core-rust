use crate::context::Context;
use crate::dc_tools::*;
use crate::sql;

// Token namespaces
#[allow(non_camel_case_types)]
type dc_tokennamespc_t = usize;
pub const DC_TOKEN_AUTH: dc_tokennamespc_t = 110;
pub const DC_TOKEN_INVITENUMBER: dc_tokennamespc_t = 100;

// Functions to read/write token from/to the database. A token is any string associated with a key.

pub fn dc_token_save(
    context: &Context,
    namespc: dc_tokennamespc_t,
    foreign_id: u32,
    token: &str,
) -> bool {
    // foreign_id may be 0
    sql::execute(
        context,
        &context.sql,
        "INSERT INTO tokens (namespc, foreign_id, token, timestamp) VALUES (?, ?, ?, ?);",
        params![namespc as i32, foreign_id as i32, token, time()],
    )
    .is_ok()
}

pub fn dc_token_lookup(
    context: &Context,
    namespc: dc_tokennamespc_t,
    foreign_id: u32,
) -> Option<String> {
    context.sql.query_row_col::<_, String>(
        context,
        "SELECT token FROM tokens WHERE namespc=? AND foreign_id=?;",
        params![namespc as i32, foreign_id as i32],
        0,
    )
}

pub fn dc_token_exists(context: &Context, namespc: dc_tokennamespc_t, token: &str) -> bool {
    context
        .sql
        .exists(
            "SELECT id FROM tokens WHERE namespc=? AND token=?;",
            params![namespc as i32, token],
        )
        .unwrap_or_default()
}
