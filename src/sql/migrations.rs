use super::{Result, Sql};
use crate::config::Config;
use crate::constants::ShowEmails;
use crate::context::Context;
use crate::dc_tools::EmailAddress;
use crate::imap;
use crate::provider::get_provider_by_domain;

const DBVERSION: i32 = 68;

pub async fn run(context: &Context, sql: &Sql) -> Result<(bool, bool, bool)> {
    let mut exists_before_update = false;
    // Init tables to dbversion=68
    let mut dbversion_before_update: i32 = DBVERSION;
    if !sql.table_exists("config").await? {
        info!(context, "First time init: creating tables",);
        sql.with_conn(move |mut conn| {
            let tx = conn.transaction()?;
            tx.execute_batch(
                r#"
CREATE TABLE config (id INTEGER PRIMARY KEY, keyname TEXT, value TEXT);
CREATE INDEX config_index1 ON config (keyname);
CREATE TABLE contacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT DEFAULT '',
    addr TEXT DEFAULT '' COLLATE NOCASE,
    origin INTEGER DEFAULT 0,
    blocked INTEGER DEFAULT 0,
    last_seen INTEGER DEFAULT 0,
    param TEXT DEFAULT '',
    authname TEXT DEFAULT '',
    selfavatar_sent INTEGER DEFAULT 0
);
CREATE INDEX contacts_index1 ON contacts (name COLLATE NOCASE);
CREATE INDEX contacts_index2 ON contacts (addr COLLATE NOCASE);
INSERT INTO contacts (id,name,origin) VALUES
(1,'self',262144), (2,'info',262144), (3,'rsvd',262144),
(4,'rsvd',262144), (5,'device',262144), (6,'rsvd',262144),
(7,'rsvd',262144), (8,'rsvd',262144), (9,'rsvd',262144);

CREATE TABLE chats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    type INTEGER DEFAULT 0,
    name TEXT DEFAULT '',
    draft_timestamp INTEGER DEFAULT 0,
    draft_txt TEXT DEFAULT '',
    blocked INTEGER DEFAULT 0,
    grpid TEXT DEFAULT '',
    param TEXT DEFAULT '',
    archived INTEGER DEFAULT 0,
    gossiped_timestamp INTEGER DEFAULT 0,
    locations_send_begin INTEGER DEFAULT 0,
    locations_send_until INTEGER DEFAULT 0,
    locations_last_sent INTEGER DEFAULT 0,
    created_timestamp INTEGER DEFAULT 0,
    muted_until INTEGER DEFAULT 0,
    ephemeral_timer INTEGER
);
CREATE INDEX chats_index1 ON chats (grpid);
CREATE INDEX chats_index2 ON chats (archived);
CREATE INDEX chats_index3 ON chats (locations_send_until);
INSERT INTO chats (id,type,name) VALUES
(1,120,'deaddrop'), (2,120,'rsvd'), (3,120,'trash'),
(4,120,'msgs_in_creation'), (5,120,'starred'), (6,120,'archivedlink'),
(7,100,'rsvd'), (8,100,'rsvd'), (9,100,'rsvd');

CREATE TABLE chats_contacts (chat_id INTEGER, contact_id INTEGER);
CREATE INDEX chats_contacts_index1 ON chats_contacts (chat_id);
CREATE INDEX chats_contacts_index2 ON chats_contacts (contact_id);

CREATE TABLE msgs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    rfc724_mid TEXT DEFAULT '',
    server_folder TEXT DEFAULT '',
    server_uid INTEGER DEFAULT 0,
    chat_id INTEGER DEFAULT 0,
    from_id INTEGER DEFAULT 0,
    to_id INTEGER DEFAULT 0,
    timestamp INTEGER DEFAULT 0,
    type INTEGER DEFAULT 0,
    state INTEGER DEFAULT 0,
    msgrmsg INTEGER DEFAULT 1,
    bytes INTEGER DEFAULT 0,
    txt TEXT DEFAULT '',
    txt_raw TEXT DEFAULT '',
    param TEXT DEFAULT '',
    starred INTEGER DEFAULT 0,
    timestamp_sent INTEGER DEFAULT 0,
    timestamp_rcvd INTEGER DEFAULT 0,
    hidden INTEGER DEFAULT 0,
    mime_headers TEXT,
    mime_in_reply_to TEXT,
    mime_references TEXT,
    move_state INTEGER DEFAULT 1,
    location_id INTEGER DEFAULT 0,
    error TEXT DEFAULT '',

-- Timer value in seconds. For incoming messages this
-- timer starts when message is read, so we want to have
-- the value stored here until the timer starts.
    ephemeral_timer INTEGER DEFAULT 0,

-- Timestamp indicating when the message should be
-- deleted. It is convenient to store it here because UI
-- needs this value to display how much time is left until
-- the message is deleted.
    ephemeral_timestamp INTEGER DEFAULT 0
);

CREATE INDEX msgs_index1 ON msgs (rfc724_mid);
CREATE INDEX msgs_index2 ON msgs (chat_id);
CREATE INDEX msgs_index3 ON msgs (timestamp);
CREATE INDEX msgs_index4 ON msgs (state);
CREATE INDEX msgs_index5 ON msgs (starred);
CREATE INDEX msgs_index6 ON msgs (location_id);
CREATE INDEX msgs_index7 ON msgs (state, hidden, chat_id);
INSERT INTO msgs (id,msgrmsg,txt) VALUES
(1,0,'marker1'), (2,0,'rsvd'), (3,0,'rsvd'),
(4,0,'rsvd'), (5,0,'rsvd'), (6,0,'rsvd'), (7,0,'rsvd'),
(8,0,'rsvd'), (9,0,'daymarker');

CREATE TABLE jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    added_timestamp INTEGER,
    desired_timestamp INTEGER DEFAULT 0,
    action INTEGER,
    foreign_id INTEGER,
    param TEXT DEFAULT '',
    thread INTEGER DEFAULT 0,
    tries INTEGER DEFAULT 0
);
CREATE INDEX jobs_index1 ON jobs (desired_timestamp);

CREATE TABLE leftgrps (
    id INTEGER PRIMARY KEY,
    grpid TEXT DEFAULT ''
);
CREATE INDEX leftgrps_index1 ON leftgrps (grpid);

CREATE TABLE keypairs (
    id INTEGER PRIMARY KEY,
    addr TEXT DEFAULT '' COLLATE NOCASE,
    is_default INTEGER DEFAULT 0,
    private_key,
    public_key,
    created INTEGER DEFAULT 0
);

CREATE TABLE acpeerstates (
    id INTEGER PRIMARY KEY,
    addr TEXT DEFAULT '' COLLATE NOCASE,
    last_seen INTEGER DEFAULT 0,
    last_seen_autocrypt INTEGER DEFAULT 0,
    public_key,
    prefer_encrypted INTEGER DEFAULT 0,
    gossip_timestamp INTEGER DEFAULT 0,
    gossip_key,
    public_key_fingerprint TEXT DEFAULT '',
    gossip_key_fingerprint TEXT DEFAULT '',
    verified_key,
    verified_key_fingerprint TEXT DEFAULT ''
);
CREATE INDEX acpeerstates_index1 ON acpeerstates (addr);
CREATE INDEX acpeerstates_index3 ON acpeerstates (public_key_fingerprint);
CREATE INDEX acpeerstates_index4 ON acpeerstates (gossip_key_fingerprint);
CREATE INDEX acpeerstates_index5 ON acpeerstates (verified_key_fingerprint);

CREATE TABLE msgs_mdns (
    msg_id INTEGER,
    contact_id INTEGER,
    timestamp_sent INTEGER DEFAULT 0
);
CREATE INDEX msgs_mdns_index1 ON msgs_mdns (msg_id);

CREATE TABLE tokens (
    id INTEGER PRIMARY KEY,
    namespc INTEGER DEFAULT 0,
    foreign_id INTEGER DEFAULT 0,
    token TEXT DEFAULT '',
    timestamp INTEGER DEFAULT 0
);

CREATE TABLE locations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    latitude REAL DEFAULT 0.0,
    longitude REAL DEFAULT 0.0,
    accuracy REAL DEFAULT 0.0,
    timestamp INTEGER DEFAULT 0,
    chat_id INTEGER DEFAULT 0,
    from_id INTEGER DEFAULT 0,
    independent INTEGER DEFAULT 0
);
CREATE INDEX locations_index1 ON locations (from_id);
CREATE INDEX locations_index2 ON locations (timestamp);

CREATE TABLE devmsglabels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    label TEXT,
    msg_id INTEGER DEFAULT 0
);
CREATE INDEX devmsglabels_index1 ON devmsglabels (label);
"#,
            )?;
            tx.commit()?;
            Ok(())
        })
        .await?;

        sql.set_raw_config_int("dbversion", dbversion_before_update)
            .await?;
    } else {
        exists_before_update = true;
        dbversion_before_update = sql
            .get_raw_config_int("dbversion")
            .await?
            .unwrap_or_default();
    }

    let mut dbversion = dbversion_before_update;
    let mut recalc_fingerprints = false;
    let mut update_icons = !exists_before_update;
    let mut disable_server_delete = false;

    if dbversion < 1 {
        info!(context, "[migration] v1");
        sql.execute("CREATE TABLE leftgrps ( id INTEGER PRIMARY KEY, grpid TEXT DEFAULT '');")
            .await?;
        sql.execute("CREATE INDEX leftgrps_index1 ON leftgrps (grpid);")
            .await?;
        sql.set_raw_config_int("dbversion", 1).await?;
    }
    if dbversion < 2 {
        info!(context, "[migration] v2");
        sql.execute("ALTER TABLE contacts ADD COLUMN authname TEXT DEFAULT '';")
            .await?;
        sql.set_raw_config_int("dbversion", 2).await?;
    }
    if dbversion < 7 {
        info!(context, "[migration] v7");
        sql.execute(
            "CREATE TABLE keypairs (\
                 id INTEGER PRIMARY KEY, \
                 addr TEXT DEFAULT '' COLLATE NOCASE, \
                 is_default INTEGER DEFAULT 0, \
                 private_key, \
                 public_key, \
                 created INTEGER DEFAULT 0);",
        )
        .await?;
        sql.set_raw_config_int("dbversion", 7).await?;
    }
    if dbversion < 10 {
        info!(context, "[migration] v10");
        sql.execute(
            "CREATE TABLE acpeerstates (\
                 id INTEGER PRIMARY KEY, \
                 addr TEXT DEFAULT '' COLLATE NOCASE, \
                 last_seen INTEGER DEFAULT 0, \
                 last_seen_autocrypt INTEGER DEFAULT 0, \
                 public_key, \
                 prefer_encrypted INTEGER DEFAULT 0);",
        )
        .await?;
        sql.execute("CREATE INDEX acpeerstates_index1 ON acpeerstates (addr);")
            .await?;
        sql.set_raw_config_int("dbversion", 10).await?;
    }
    if dbversion < 12 {
        info!(context, "[migration] v12");
        sql.execute("CREATE TABLE msgs_mdns ( msg_id INTEGER,  contact_id INTEGER);")
            .await?;
        sql.execute("CREATE INDEX msgs_mdns_index1 ON msgs_mdns (msg_id);")
            .await?;
        sql.set_raw_config_int("dbversion", 12).await?;
    }
    if dbversion < 17 {
        info!(context, "[migration] v17");
        sql.execute("ALTER TABLE chats ADD COLUMN archived INTEGER DEFAULT 0;")
            .await?;
        sql.execute("CREATE INDEX chats_index2 ON chats (archived);")
            .await?;
        // 'starred' column is not used currently
        // (dropping is not easily doable and stop adding it will make reusing it complicated)
        sql.execute("ALTER TABLE msgs ADD COLUMN starred INTEGER DEFAULT 0;")
            .await?;
        sql.execute("CREATE INDEX msgs_index5 ON msgs (starred);")
            .await?;
        sql.set_raw_config_int("dbversion", 17).await?;
    }
    if dbversion < 18 {
        info!(context, "[migration] v18");
        sql.execute("ALTER TABLE acpeerstates ADD COLUMN gossip_timestamp INTEGER DEFAULT 0;")
            .await?;
        sql.execute("ALTER TABLE acpeerstates ADD COLUMN gossip_key;")
            .await?;
        sql.set_raw_config_int("dbversion", 18).await?;
    }
    if dbversion < 27 {
        info!(context, "[migration] v27");
        // chat.id=1 and chat.id=2 are the old deaddrops,
        // the current ones are defined by chats.blocked=2
        sql.execute("DELETE FROM msgs WHERE chat_id=1 OR chat_id=2;")
            .await?;
        sql.execute("CREATE INDEX chats_contacts_index2 ON chats_contacts (contact_id);")
            .await?;
        sql.execute("ALTER TABLE msgs ADD COLUMN timestamp_sent INTEGER DEFAULT 0;")
            .await?;
        sql.execute("ALTER TABLE msgs ADD COLUMN timestamp_rcvd INTEGER DEFAULT 0;")
            .await?;
        sql.set_raw_config_int("dbversion", 27).await?;
    }
    if dbversion < 34 {
        info!(context, "[migration] v34");
        sql.execute("ALTER TABLE msgs ADD COLUMN hidden INTEGER DEFAULT 0;")
            .await?;
        sql.execute("ALTER TABLE msgs_mdns ADD COLUMN timestamp_sent INTEGER DEFAULT 0;")
            .await?;
        sql.execute("ALTER TABLE acpeerstates ADD COLUMN public_key_fingerprint TEXT DEFAULT '';")
            .await?;
        sql.execute("ALTER TABLE acpeerstates ADD COLUMN gossip_key_fingerprint TEXT DEFAULT '';")
            .await?;
        sql.execute("CREATE INDEX acpeerstates_index3 ON acpeerstates (public_key_fingerprint);")
            .await?;
        sql.execute("CREATE INDEX acpeerstates_index4 ON acpeerstates (gossip_key_fingerprint);")
            .await?;
        recalc_fingerprints = true;
        sql.set_raw_config_int("dbversion", 34).await?;
    }
    if dbversion < 39 {
        info!(context, "[migration] v39");
        sql.execute(
                "CREATE TABLE tokens ( id INTEGER PRIMARY KEY, namespc INTEGER DEFAULT 0, foreign_id INTEGER DEFAULT 0, token TEXT DEFAULT '', timestamp INTEGER DEFAULT 0);",
                
            ).await?;
        sql.execute("ALTER TABLE acpeerstates ADD COLUMN verified_key;")
            .await?;
        sql.execute(
            "ALTER TABLE acpeerstates ADD COLUMN verified_key_fingerprint TEXT DEFAULT '';",
        )
        .await?;
        sql.execute("CREATE INDEX acpeerstates_index5 ON acpeerstates (verified_key_fingerprint);")
            .await?;
        sql.set_raw_config_int("dbversion", 39).await?;
    }
    if dbversion < 40 {
        info!(context, "[migration] v40");
        sql.execute("ALTER TABLE jobs ADD COLUMN thread INTEGER DEFAULT 0;")
            .await?;
        sql.set_raw_config_int("dbversion", 40).await?;
    }
    if dbversion < 44 {
        info!(context, "[migration] v44");
        sql.execute("ALTER TABLE msgs ADD COLUMN mime_headers TEXT;")
            .await?;
        sql.set_raw_config_int("dbversion", 44).await?;
    }
    if dbversion < 46 {
        info!(context, "[migration] v46");
        sql.execute("ALTER TABLE msgs ADD COLUMN mime_in_reply_to TEXT;")
            .await?;
        sql.execute("ALTER TABLE msgs ADD COLUMN mime_references TEXT;")
            .await?;
        dbversion = 46;
        sql.set_raw_config_int("dbversion", 46).await?;
    }
    if dbversion < 47 {
        info!(context, "[migration] v47");
        sql.execute("ALTER TABLE jobs ADD COLUMN tries INTEGER DEFAULT 0;")
            .await?;
        sql.set_raw_config_int("dbversion", 47).await?;
    }
    if dbversion < 48 {
        info!(context, "[migration] v48");
        // NOTE: move_state is not used anymore
        sql.execute("ALTER TABLE msgs ADD COLUMN move_state INTEGER DEFAULT 1;")
            .await?;
        sql.set_raw_config_int("dbversion", 48).await?;
    }
    if dbversion < 49 {
        info!(context, "[migration] v49");
        sql.execute("ALTER TABLE chats ADD COLUMN gossiped_timestamp INTEGER DEFAULT 0;")
            .await?;
        sql.set_raw_config_int("dbversion", 49).await?;
    }
    if dbversion < 50 {
        info!(context, "[migration] v50");
        // installations <= 0.100.1 used DC_SHOW_EMAILS_ALL implicitly;
        // keep this default and use DC_SHOW_EMAILS_NO
        // only for new installations
        if exists_before_update {
            sql.set_raw_config_int("show_emails", ShowEmails::All as i32)
                .await?;
        }
        sql.set_raw_config_int("dbversion", 50).await?;
    }
    if dbversion < 53 {
        info!(context, "[migration] v53");
        // the messages containing _only_ locations
        // are also added to the database as _hidden_.
        sql.execute(
                "CREATE TABLE locations ( id INTEGER PRIMARY KEY AUTOINCREMENT, latitude REAL DEFAULT 0.0, longitude REAL DEFAULT 0.0, accuracy REAL DEFAULT 0.0, timestamp INTEGER DEFAULT 0, chat_id INTEGER DEFAULT 0, from_id INTEGER DEFAULT 0);",
                
            ).await?;
        sql.execute("CREATE INDEX locations_index1 ON locations (from_id);")
            .await?;
        sql.execute("CREATE INDEX locations_index2 ON locations (timestamp);")
            .await?;
        sql.execute("ALTER TABLE chats ADD COLUMN locations_send_begin INTEGER DEFAULT 0;")
            .await?;
        sql.execute("ALTER TABLE chats ADD COLUMN locations_send_until INTEGER DEFAULT 0;")
            .await?;
        sql.execute("ALTER TABLE chats ADD COLUMN locations_last_sent INTEGER DEFAULT 0;")
            .await?;
        sql.execute("CREATE INDEX chats_index3 ON chats (locations_send_until);")
            .await?;
        sql.set_raw_config_int("dbversion", 53).await?;
    }
    if dbversion < 54 {
        info!(context, "[migration] v54");
        sql.execute("ALTER TABLE msgs ADD COLUMN location_id INTEGER DEFAULT 0;")
            .await?;
        sql.execute("CREATE INDEX msgs_index6 ON msgs (location_id);")
            .await?;
        sql.set_raw_config_int("dbversion", 54).await?;
    }
    if dbversion < 55 {
        info!(context, "[migration] v55");
        sql.execute("ALTER TABLE locations ADD COLUMN independent INTEGER DEFAULT 0;")
            .await?;
        sql.set_raw_config_int("dbversion", 55).await?;
    }
    if dbversion < 59 {
        info!(context, "[migration] v59");
        // records in the devmsglabels are kept when the message is deleted.
        // so, msg_id may or may not exist.
        sql.execute(
                "CREATE TABLE devmsglabels (id INTEGER PRIMARY KEY AUTOINCREMENT, label TEXT, msg_id INTEGER DEFAULT 0);",
                
            ).await?;
        sql.execute("CREATE INDEX devmsglabels_index1 ON devmsglabels (label);")
            .await?;
        if exists_before_update && sql.get_raw_config_int("bcc_self").await?.is_none() {
            sql.set_raw_config_int("bcc_self", 1).await?;
        }
        sql.set_raw_config_int("dbversion", 59).await?;
    }
    if dbversion < 60 {
        info!(context, "[migration] v60");
        sql.execute("ALTER TABLE chats ADD COLUMN created_timestamp INTEGER DEFAULT 0;")
            .await?;
        sql.set_raw_config_int("dbversion", 60).await?;
    }
    if dbversion < 61 {
        info!(context, "[migration] v61");
        sql.execute("ALTER TABLE contacts ADD COLUMN selfavatar_sent INTEGER DEFAULT 0;")
            .await?;
        update_icons = true;
        sql.set_raw_config_int("dbversion", 61).await?;
    }
    if dbversion < 62 {
        info!(context, "[migration] v62");
        sql.execute("ALTER TABLE chats ADD COLUMN muted_until INTEGER DEFAULT 0;")
            .await?;
        sql.set_raw_config_int("dbversion", 62).await?;
    }
    if dbversion < 63 {
        info!(context, "[migration] v63");
        sql.execute("UPDATE chats SET grpid='' WHERE type=100")
            .await?;
        sql.set_raw_config_int("dbversion", 63).await?;
    }
    if dbversion < 64 {
        info!(context, "[migration] v64");
        sql.execute("ALTER TABLE msgs ADD COLUMN error TEXT DEFAULT '';")
            .await?;
        sql.set_raw_config_int("dbversion", 64).await?;
    }
    if dbversion < 65 {
        info!(context, "[migration] v65");
        sql.execute("ALTER TABLE chats ADD COLUMN ephemeral_timer INTEGER")
            .await?;
        sql.execute("ALTER TABLE msgs ADD COLUMN ephemeral_timer INTEGER DEFAULT 0")
            .await?;
        sql.execute("ALTER TABLE msgs ADD COLUMN ephemeral_timestamp INTEGER DEFAULT 0")
            .await?;
        sql.set_raw_config_int("dbversion", 65).await?;
    }
    if dbversion < 66 {
        info!(context, "[migration] v66");
        update_icons = true;
        sql.set_raw_config_int("dbversion", 66).await?;
    }
    if dbversion < 67 {
        info!(context, "[migration] v67");
        for prefix in &["", "configured_"] {
            if let Some(server_flags) = sql
                .get_raw_config_int(format!("{}server_flags", prefix))
                .await?
            {
                let imap_socket_flags = server_flags & 0x700;
                let key = format!("{}mail_security", prefix);
                match imap_socket_flags {
                    0x100 => sql.set_raw_config_int(key, 2).await?, // STARTTLS
                    0x200 => sql.set_raw_config_int(key, 1).await?, // SSL/TLS
                    0x400 => sql.set_raw_config_int(key, 3).await?, // Plain
                    _ => sql.set_raw_config_int(key, 0).await?,
                }
                let smtp_socket_flags = server_flags & 0x70000;
                let key = format!("{}send_security", prefix);
                match smtp_socket_flags {
                    0x10000 => sql.set_raw_config_int(key, 2).await?, // STARTTLS
                    0x20000 => sql.set_raw_config_int(key, 1).await?, // SSL/TLS
                    0x40000 => sql.set_raw_config_int(key, 3).await?, // Plain
                    _ => sql.set_raw_config_int(key, 0).await?,
                }
            }
        }
        sql.set_raw_config_int("dbversion", 67).await?;
    }
    if dbversion < 68 {
        info!(context, "[migration] v68");
        // the index is used to speed up get_fresh_msg_cnt() (see comment there for more details) and marknoticed_chat()
        sql.execute("CREATE INDEX IF NOT EXISTS msgs_index7 ON msgs (state, hidden, chat_id);")
            .await?;
        sql.set_raw_config_int("dbversion", 68).await?;
    }
    if dbversion < 69 {
        info!(context, "[migration] v69");
        sql.execute("ALTER TABLE chats ADD COLUMN protected INTEGER DEFAULT 0;")
            .await?;
        sql.execute(
            "UPDATE chats SET protected=1, type=120 WHERE type=130;", // 120=group, 130=old verified group
        )
        .await?;
        sql.set_raw_config_int("dbversion", 69).await?;
    }

    if dbversion < 69 {
        info!(context, "[migration] v69");
        sql.execute(
            "ALTER TABLE chats ADD COLUMN protected INTEGER DEFAULT 0;"
        )
        .await?;
        sql.execute(
            "UPDATE chats SET protected=1, type=120 WHERE type=130;" // 120=group, 130=old verified group
        )
        .await?;
        sql.set_raw_config_int("dbversion", 69).await?;
    }
    if dbversion < 71 {
        info!(context, "[migration] v71");
        if let Some(addr) = context.get_config(Config::ConfiguredAddr).await? {
            if let Ok(domain) = addr.parse::<EmailAddress>().map(|email| email.domain) {
                context
                    .set_config(
                        Config::ConfiguredProvider,
                        get_provider_by_domain(&domain).map(|provider| provider.id),
                    )
                    .await?;
            } else {
                warn!(context, "Can't parse configured address: {:?}", addr);
            }
        }

        sql.set_raw_config_int("dbversion", 71).await?;
    }
    if dbversion < 72 {
        info!(context, "[migration] v72");
        if !sql.col_exists("msgs", "mime_modified").await? {
            sql.execute(
                "ALTER TABLE msgs ADD COLUMN mime_modified INTEGER DEFAULT 0;"
            )
            .await?;
        }
        sql.set_raw_config_int("dbversion", 72).await?;
    }
    if dbversion < 73 {
        use Config::*;
        info!(context, "[migration] v73");
        sql.execute(
                "CREATE TABLE imap_sync (folder TEXT PRIMARY KEY, uidvalidity INTEGER DEFAULT 0, uid_next INTEGER DEFAULT 0);"
            )
            .await?;
        for c in &[
            ConfiguredInboxFolder,
            ConfiguredSentboxFolder,
            ConfiguredMvboxFolder,
        ] {
            if let Some(folder) = context.get_config(*c).await? {
                let (uid_validity, last_seen_uid) =
                    imap::get_config_last_seen_uid(context, &folder).await?;
                if last_seen_uid > 0 {
                    imap::set_uid_next(context, &folder, last_seen_uid + 1).await?;
                    imap::set_uidvalidity(context, &folder, uid_validity).await?;
                }
            }
        }
        if exists_before_update {
            disable_server_delete = true;

            // Don't disable server delete if it was on by default (Nauta):
            if let Some(provider) = context.get_configured_provider().await? {
                if let Some(defaults) = &provider.config_defaults {
                    if defaults.iter().any(|d| d.key == Config::DeleteServerAfter) {
                        disable_server_delete = false;
                    }
                }
            }
        }
        sql.set_raw_config_int("dbversion", 73).await?;
    }
    if dbversion < 74 {
        info!(context, "[migration] v74");
        sql.execute(
            "UPDATE contacts SET name='' WHERE name=authname"
        )
        .await?;
        sql.set_raw_config_int("dbversion", 74).await?;
    }
    if dbversion < 75 {
        info!(context, "[migration] v75");
        sql.execute(
            "ALTER TABLE contacts ADD COLUMN status TEXT DEFAULT '';"
        )
        .await?;
        sql.set_raw_config_int("dbversion", 75).await?;
    }
    if dbversion < 76 {
        info!(context, "[migration] v76");
        sql.execute(
            "ALTER TABLE msgs ADD COLUMN subject TEXT DEFAULT '';"
        )
        .await?;
        sql.set_raw_config_int("dbversion", 76).await?;
    }

    Ok((recalc_fingerprints, update_icons, disable_server_delete))
}
