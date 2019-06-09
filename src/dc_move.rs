use crate::constants::*;
use crate::context::*;
use crate::dc_job::*;
use crate::dc_msg::*;
use crate::dc_sqlite3::*;
use crate::types::*;

pub unsafe fn dc_do_heuristics_moves(context: &Context, folder: &str, msg_id: u32) {
    // for already seen messages, folder may be different from msg->folder
    let mut msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"mvbox_move\x00" as *const u8 as *const libc::c_char,
        1i32,
    ) == 0i32)
    {
        if !(0 == dc_is_inbox(context, folder) && 0 == dc_is_sentbox(context, folder)) {
            msg = dc_msg_new_load(context, msg_id);
            if !(0 != dc_msg_is_setupmessage(msg)) {
                // do not move setup messages;
                // there may be a non-delta device that wants to handle it
                if 0 != dc_is_mvbox(context, folder) {
                    dc_update_msg_move_state(context, (*msg).rfc724_mid, DC_MOVE_STATE_STAY);
                } else if 0 != (*msg).is_dc_message {
                    dc_job_add(
                        context,
                        200i32,
                        (*msg).id as libc::c_int,
                        0 as *const libc::c_char,
                        0i32,
                    );
                    dc_update_msg_move_state(context, (*msg).rfc724_mid, DC_MOVE_STATE_MOVING);
                }
            }
        }
    }
    sqlite3_finalize(stmt);
    dc_msg_unref(msg);
}
