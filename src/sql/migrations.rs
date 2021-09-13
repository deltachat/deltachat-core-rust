//! Migrations module.

use anyhow::Result;

use crate::config::Config;
use crate::constants::ShowEmails;
use crate::context::Context;
use crate::dc_tools::EmailAddress;
use crate::imap;
use crate::provider::get_provider_by_domain;
use crate::sql::Sql;

const DBVERSION: i32 = 68;
const VERSION_CFG: &str = "dbversion";
const TABLES: &str = include_str!("./tables.sql");

pub async fn run(context: &Context, sql: &Sql) -> Result<(bool, bool, bool, bool)> {
    let mut recalc_fingerprints = false;
    let mut exists_before_update = false;
    let mut dbversion_before_update = DBVERSION;

    if !sql.table_exists("config").await? {
        info!(context, "First time init: creating tables",);
        sql.transaction(move |transaction| {
            transaction.execute_batch(TABLES)?;

            // set raw config inside the transaction
            transaction.execute(
                "INSERT INTO config (keyname, value) VALUES (?, ?);",
                paramsv![VERSION_CFG, format!("{}", dbversion_before_update)],
            )?;
            Ok(())
        })
        .await?;
    } else {
        exists_before_update = true;
        dbversion_before_update = sql
            .get_raw_config_int(VERSION_CFG)
            .await?
            .unwrap_or_default();
    }

    let dbversion = dbversion_before_update;
    let mut update_icons = !exists_before_update;
    let mut disable_server_delete = false;
    let mut recode_avatar = false;

    if dbversion < 1 {
        info!(context, "[migration] v1");
        sql.execute_migration(
            r#"
CREATE TABLE leftgrps ( id INTEGER PRIMARY KEY, grpid TEXT DEFAULT '');
CREATE INDEX leftgrps_index1 ON leftgrps (grpid);"#,
            1,
        )
        .await?;
    }
    if dbversion < 2 {
        info!(context, "[migration] v2");
        sql.execute_migration(
            "ALTER TABLE contacts ADD COLUMN authname TEXT DEFAULT '';",
            2,
        )
        .await?;
    }
    if dbversion < 7 {
        info!(context, "[migration] v7");
        sql.execute_migration(
            "CREATE TABLE keypairs (\
                 id INTEGER PRIMARY KEY, \
                 addr TEXT DEFAULT '' COLLATE NOCASE, \
                 is_default INTEGER DEFAULT 0, \
                 private_key, \
                 public_key, \
                 created INTEGER DEFAULT 0);",
            7,
        )
        .await?;
    }
    if dbversion < 10 {
        info!(context, "[migration] v10");
        sql.execute_migration(
            "CREATE TABLE acpeerstates (\
                 id INTEGER PRIMARY KEY, \
                 addr TEXT DEFAULT '' COLLATE NOCASE, \
                 last_seen INTEGER DEFAULT 0, \
                 last_seen_autocrypt INTEGER DEFAULT 0, \
                 public_key, \
                 prefer_encrypted INTEGER DEFAULT 0); \
              CREATE INDEX acpeerstates_index1 ON acpeerstates (addr);",
            10,
        )
        .await?;
    }
    if dbversion < 12 {
        info!(context, "[migration] v12");
        sql.execute_migration(
            r#"
CREATE TABLE msgs_mdns ( msg_id INTEGER,  contact_id INTEGER);
CREATE INDEX msgs_mdns_index1 ON msgs_mdns (msg_id);"#,
            12,
        )
        .await?;
    }
    if dbversion < 17 {
        info!(context, "[migration] v17");
        sql.execute_migration(
            r#"
ALTER TABLE chats ADD COLUMN archived INTEGER DEFAULT 0;
CREATE INDEX chats_index2 ON chats (archived);
-- 'starred' column is not used currently
-- (dropping is not easily doable and stop adding it will make reusing it complicated)
ALTER TABLE msgs ADD COLUMN starred INTEGER DEFAULT 0;
CREATE INDEX msgs_index5 ON msgs (starred);"#,
            17,
        )
        .await?;
    }
    if dbversion < 18 {
        info!(context, "[migration] v18");
        sql.execute_migration(
            r#"
ALTER TABLE acpeerstates ADD COLUMN gossip_timestamp INTEGER DEFAULT 0;
ALTER TABLE acpeerstates ADD COLUMN gossip_key;"#,
            18,
        )
        .await?;
    }
    if dbversion < 27 {
        info!(context, "[migration] v27");
        // chat.id=1 and chat.id=2 are the old deaddrops,
        // the current ones are defined by chats.blocked=2
        sql.execute_migration(
            r#"
DELETE FROM msgs WHERE chat_id=1 OR chat_id=2;"
CREATE INDEX chats_contacts_index2 ON chats_contacts (contact_id);"
ALTER TABLE msgs ADD COLUMN timestamp_sent INTEGER DEFAULT 0;")
ALTER TABLE msgs ADD COLUMN timestamp_rcvd INTEGER DEFAULT 0;"#,
            27,
        )
        .await?;
    }
    if dbversion < 34 {
        info!(context, "[migration] v34");
        sql.execute_migration(
            r#"
ALTER TABLE msgs ADD COLUMN hidden INTEGER DEFAULT 0;
ALTER TABLE msgs_mdns ADD COLUMN timestamp_sent INTEGER DEFAULT 0;
ALTER TABLE acpeerstates ADD COLUMN public_key_fingerprint TEXT DEFAULT '';
ALTER TABLE acpeerstates ADD COLUMN gossip_key_fingerprint TEXT DEFAULT '';
CREATE INDEX acpeerstates_index3 ON acpeerstates (public_key_fingerprint);
CREATE INDEX acpeerstates_index4 ON acpeerstates (gossip_key_fingerprint);"#,
            34,
        )
        .await?;
        recalc_fingerprints = true;
    }
    if dbversion < 39 {
        info!(context, "[migration] v39");
        sql.execute_migration(
            r#"
CREATE TABLE tokens ( 
  id INTEGER PRIMARY KEY, 
  namespc INTEGER DEFAULT 0, 
  foreign_id INTEGER DEFAULT 0, 
  token TEXT DEFAULT '', 
  timestamp INTEGER DEFAULT 0
);
ALTER TABLE acpeerstates ADD COLUMN verified_key;
ALTER TABLE acpeerstates ADD COLUMN verified_key_fingerprint TEXT DEFAULT '';
CREATE INDEX acpeerstates_index5 ON acpeerstates (verified_key_fingerprint);"#,
            39,
        )
        .await?;
    }
    if dbversion < 40 {
        info!(context, "[migration] v40");
        sql.execute_migration("ALTER TABLE jobs ADD COLUMN thread INTEGER DEFAULT 0;", 40)
            .await?;
    }
    if dbversion < 44 {
        info!(context, "[migration] v44");
        sql.execute_migration("ALTER TABLE msgs ADD COLUMN mime_headers TEXT;", 44)
            .await?;
    }
    if dbversion < 46 {
        info!(context, "[migration] v46");
        sql.execute_migration(
            r#"
ALTER TABLE msgs ADD COLUMN mime_in_reply_to TEXT;
ALTER TABLE msgs ADD COLUMN mime_references TEXT;"#,
            46,
        )
        .await?;
    }
    if dbversion < 47 {
        info!(context, "[migration] v47");
        sql.execute_migration("ALTER TABLE jobs ADD COLUMN tries INTEGER DEFAULT 0;", 47)
            .await?;
    }
    if dbversion < 48 {
        info!(context, "[migration] v48");
        // NOTE: move_state is not used anymore
        sql.execute_migration(
            "ALTER TABLE msgs ADD COLUMN move_state INTEGER DEFAULT 1;",
            48,
        )
        .await?;
    }
    if dbversion < 49 {
        info!(context, "[migration] v49");
        sql.execute_migration(
            "ALTER TABLE chats ADD COLUMN gossiped_timestamp INTEGER DEFAULT 0;",
            49,
        )
        .await?;
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
        sql.set_db_version(50).await?;
    }
    if dbversion < 53 {
        info!(context, "[migration] v53");
        // the messages containing _only_ locations
        // are also added to the database as _hidden_.
        sql.execute_migration(
            r#"
CREATE TABLE locations ( 
  id INTEGER PRIMARY KEY AUTOINCREMENT, 
  latitude REAL DEFAULT 0.0, 
  longitude REAL DEFAULT 0.0, 
  accuracy REAL DEFAULT 0.0, 
  timestamp INTEGER DEFAULT 0, 
  chat_id INTEGER DEFAULT 0, 
  from_id INTEGER DEFAULT 0
);"
CREATE INDEX locations_index1 ON locations (from_id);
CREATE INDEX locations_index2 ON locations (timestamp);
ALTER TABLE chats ADD COLUMN locations_send_begin INTEGER DEFAULT 0;
ALTER TABLE chats ADD COLUMN locations_send_until INTEGER DEFAULT 0;
ALTER TABLE chats ADD COLUMN locations_last_sent INTEGER DEFAULT 0;
CREATE INDEX chats_index3 ON chats (locations_send_until);"#,
            53,
        )
        .await?;
    }
    if dbversion < 54 {
        info!(context, "[migration] v54");
        sql.execute_migration(
            r#"
ALTER TABLE msgs ADD COLUMN location_id INTEGER DEFAULT 0;
CREATE INDEX msgs_index6 ON msgs (location_id);"#,
            54,
        )
        .await?;
    }
    if dbversion < 55 {
        info!(context, "[migration] v55");
        sql.execute_migration(
            "ALTER TABLE locations ADD COLUMN independent INTEGER DEFAULT 0;",
            55,
        )
        .await?;
    }
    if dbversion < 59 {
        info!(context, "[migration] v59");
        // records in the devmsglabels are kept when the message is deleted.
        // so, msg_id may or may not exist.
        sql.execute_migration(
            r#"
CREATE TABLE devmsglabels (id INTEGER PRIMARY KEY AUTOINCREMENT, label TEXT, msg_id INTEGER DEFAULT 0);",
CREATE INDEX devmsglabels_index1 ON devmsglabels (label);"#, 59)
            .await?;
        if exists_before_update && sql.get_raw_config_int("bcc_self").await?.is_none() {
            sql.set_raw_config_int("bcc_self", 1).await?;
        }
    }

    if dbversion < 60 {
        info!(context, "[migration] v60");
        sql.execute_migration(
            "ALTER TABLE chats ADD COLUMN created_timestamp INTEGER DEFAULT 0;",
            60,
        )
        .await?;
    }
    if dbversion < 61 {
        info!(context, "[migration] v61");
        sql.execute_migration(
            "ALTER TABLE contacts ADD COLUMN selfavatar_sent INTEGER DEFAULT 0;",
            61,
        )
        .await?;
        update_icons = true;
    }
    if dbversion < 62 {
        info!(context, "[migration] v62");
        sql.execute_migration(
            "ALTER TABLE chats ADD COLUMN muted_until INTEGER DEFAULT 0;",
            62,
        )
        .await?;
    }
    if dbversion < 63 {
        info!(context, "[migration] v63");
        sql.execute_migration("UPDATE chats SET grpid='' WHERE type=100", 63)
            .await?;
    }
    if dbversion < 64 {
        info!(context, "[migration] v64");
        sql.execute_migration("ALTER TABLE msgs ADD COLUMN error TEXT DEFAULT '';", 64)
            .await?;
    }
    if dbversion < 65 {
        info!(context, "[migration] v65");
        sql.execute_migration(
            r#"
ALTER TABLE chats ADD COLUMN ephemeral_timer INTEGER;
ALTER TABLE msgs ADD COLUMN ephemeral_timer INTEGER DEFAULT 0;
ALTER TABLE msgs ADD COLUMN ephemeral_timestamp INTEGER DEFAULT 0;"#,
            65,
        )
        .await?;
    }
    if dbversion < 66 {
        info!(context, "[migration] v66");
        update_icons = true;
        sql.set_db_version(66).await?;
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
        sql.set_db_version(67).await?;
    }
    if dbversion < 68 {
        info!(context, "[migration] v68");
        // the index is used to speed up get_fresh_msg_cnt() (see comment there for more details) and marknoticed_chat()
        sql.execute_migration(
            "CREATE INDEX IF NOT EXISTS msgs_index7 ON msgs (state, hidden, chat_id);",
            68,
        )
        .await?;
    }
    if dbversion < 69 {
        info!(context, "[migration] v69");
        sql.execute_migration(
            r#"
ALTER TABLE chats ADD COLUMN protected INTEGER DEFAULT 0;
-- 120=group, 130=old verified group
UPDATE chats SET protected=1, type=120 WHERE type=130;"#,
            69,
        )
        .await?;
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

        sql.set_db_version(71).await?;
    }
    if dbversion < 72 {
        info!(context, "[migration] v72");
        if !sql.col_exists("msgs", "mime_modified").await? {
            sql.execute_migration(
                r#"
ALTER TABLE msgs ADD COLUMN mime_modified INTEGER DEFAULT 0;"#,
                72,
            )
            .await?;
        }
    }
    if dbversion < 73 {
        use Config::*;
        info!(context, "[migration] v73");
        sql.execute(
            r#"
CREATE TABLE imap_sync (folder TEXT PRIMARY KEY, uidvalidity INTEGER DEFAULT 0, uid_next INTEGER DEFAULT 0);"#,
paramsv![]
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
        sql.set_db_version(73).await?;
    }
    if dbversion < 74 {
        info!(context, "[migration] v74");
        sql.execute_migration("UPDATE contacts SET name='' WHERE name=authname", 74)
            .await?;
    }
    if dbversion < 75 {
        info!(context, "[migration] v75");
        sql.execute_migration(
            "ALTER TABLE contacts ADD COLUMN status TEXT DEFAULT '';",
            75,
        )
        .await?;
    }
    if dbversion < 76 {
        info!(context, "[migration] v76");
        sql.execute_migration("ALTER TABLE msgs ADD COLUMN subject TEXT DEFAULT '';", 76)
            .await?;
    }
    if dbversion < 77 {
        info!(context, "[migration] v77");
        recode_avatar = true;
        sql.set_db_version(77).await?;
    }
    if dbversion < 78 {
        // move requests to "Archived Chats",
        // this way, the app looks familiar after the contact request upgrade.
        info!(context, "[migration] v78");
        sql.execute_migration("UPDATE chats SET archived=1 WHERE blocked=2;", 78)
            .await?;
    }
    if dbversion < 79 {
        info!(context, "[migration] v79");
        sql.execute_migration(
            r#"
        ALTER TABLE msgs ADD COLUMN download_state INTEGER DEFAULT 0;
        "#,
            79,
        )
        .await?;
    }

    Ok((
        recalc_fingerprints,
        update_icons,
        disable_server_delete,
        recode_avatar,
    ))
}

impl Sql {
    async fn set_db_version(&self, version: i32) -> Result<()> {
        self.set_raw_config_int(VERSION_CFG, version).await?;
        Ok(())
    }

    async fn execute_migration(&self, query: &'static str, version: i32) -> Result<()> {
        self.transaction(move |transaction| {
            transaction.execute_batch(query)?;

            // set raw config inside the transaction
            transaction.execute(
                "UPDATE config SET value=? WHERE keyname=?;",
                paramsv![format!("{}", version), VERSION_CFG],
            )?;

            Ok(())
        })
        .await?;

        Ok(())
    }
}
