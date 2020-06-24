use super::{Error, Result, Sql};
use crate::constants::ShowEmails;
use crate::context::Context;

/// Executes all migrations required to get from the passed in `dbversion` to the latest.
pub async fn run(
    context: &Context,
    sql: &Sql,
    dbversion: i32,
    exists_before_update: bool,
) -> Result<()> {
    let migrate = |version: i32, stmt: &'static str| async move {
        if dbversion < version {
            info!(context, "[migration] v{}", version);

            sql.execute_batch(stmt).await?;
            sql.set_raw_config_int(context, "dbversion", version)
                .await?;
        }

        Ok::<_, Error>(())
    };

    migrate(
        0,
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
  param TEXT DEFAULT '');
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
  param TEXT DEFAULT '');
CREATE INDEX chats_index1 ON chats (grpid);
CREATE TABLE chats_contacts (chat_id INTEGER, contact_id INTEGER);
CREATE INDEX chats_contacts_index1 ON chats_contacts (chat_id);
INSERT INTO chats (id,type,name) VALUES
  (1,120,'deaddrop'), (2,120,'rsvd'), (3,120,'trash'),
  (4,120,'msgs_in_creation'), (5,120,'starred'), (6,120,'archivedlink'),
  (7,100,'rsvd'), (8,100,'rsvd'), (9,100,'rsvd');
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
  param TEXT DEFAULT '');
CREATE INDEX msgs_index1 ON msgs (rfc724_mid);
CREATE INDEX msgs_index2 ON msgs (chat_id);
CREATE INDEX msgs_index3 ON msgs (timestamp);
CREATE INDEX msgs_index4 ON msgs (state);
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
  param TEXT DEFAULT '');
CREATE INDEX jobs_index1 ON jobs (desired_timestamp);
"#,
    )
    .await?;

    migrate(
        1,
        r#"
CREATE TABLE leftgrps ( 
  id INTEGER PRIMARY KEY, 
  grpid TEXT DEFAULT '');
CREATE INDEX leftgrps_index1 ON leftgrps (grpid);
"#,
    )
    .await?;

    migrate(
        2,
        r#"
ALTER TABLE contacts ADD COLUMN authname TEXT DEFAULT '';
"#,
    )
    .await?;

    migrate(
        7,
        r#"
CREATE TABLE keypairs (
  id INTEGER PRIMARY KEY,
  addr TEXT DEFAULT '' COLLATE NOCASE,
  is_default INTEGER DEFAULT 0,
  private_key,
  public_key,
  created INTEGER DEFAULT 0);
"#,
    )
    .await?;

    migrate(
        10,
        r#"
CREATE TABLE acpeerstates (
  id INTEGER PRIMARY KEY,
  addr TEXT DEFAULT '' COLLATE NOCASE,
  last_seen INTEGER DEFAULT 0,
  last_seen_autocrypt INTEGER DEFAULT 0,
  public_key,
  prefer_encrypted INTEGER DEFAULT 0);
"#,
    )
    .await?;

    migrate(
        12,
        r#"
CREATE TABLE msgs_mdns (
  msg_id INTEGER, 
  contact_id INTEGER);
CREATE INDEX msgs_mdns_index1 ON msgs_mdns (msg_id);
"#,
    )
    .await?;

    migrate(
        17,
        r#"
ALTER TABLE chats ADD COLUMN archived INTEGER DEFAULT 0;
CREATE INDEX chats_index2 ON chats (archived);
ALTER TABLE msgs ADD COLUMN starred INTEGER DEFAULT 0;
CREATE INDEX msgs_index5 ON msgs (starred);
"#,
    )
    .await?;

    migrate(
        18,
        r#"
ALTER TABLE acpeerstates ADD COLUMN gossip_timestamp INTEGER DEFAULT 0;
ALTER TABLE acpeerstates ADD COLUMN gossip_key;
"#,
    )
    .await?;

    // chat.id=1 and chat.id=2 are the old deaddrops,
    // the current ones are defined by chats.blocked=2
    migrate(
        27,
        r#"
DELETE FROM msgs WHERE chat_id=1 OR chat_id=2;
CREATE INDEX chats_contacts_index2 ON chats_contacts (contact_id);
ALTER TABLE msgs ADD COLUMN timestamp_sent INTEGER DEFAULT 0;
ALTER TABLE msgs ADD COLUMN timestamp_rcvd INTEGER DEFAULT 0;
"#,
    )
    .await?;

    migrate(
        34,
        r#"
ALTER TABLE msgs ADD COLUMN hidden INTEGER DEFAULT 0;
ALTER TABLE msgs_mdns ADD COLUMN timestamp_sent INTEGER DEFAULT 0;
ALTER TABLE acpeerstates ADD COLUMN public_key_fingerprint TEXT DEFAULT '';
ALTER TABLE acpeerstates ADD COLUMN gossip_key_fingerprint TEXT DEFAULT '';
CREATE INDEX acpeerstates_index3 ON acpeerstates (public_key_fingerprint);
CREATE INDEX acpeerstates_index4 ON acpeerstates (gossip_key_fingerprint);
"#,
    )
    .await?;

    migrate(
        39,
        r#"
CREATE TABLE tokens ( 
  id INTEGER PRIMARY KEY, 
  namespc INTEGER DEFAULT 0, 
  foreign_id INTEGER DEFAULT 0, 
  token TEXT DEFAULT '', 
  timestamp INTEGER DEFAULT 0);
ALTER TABLE acpeerstates ADD COLUMN verified_key;
ALTER TABLE acpeerstates ADD COLUMN verified_key_fingerprint TEXT DEFAULT '';
CREATE INDEX acpeerstates_index5 ON acpeerstates (verified_key_fingerprint);
"#,
    )
    .await?;

    migrate(
        40,
        r#"
ALTER TABLE jobs ADD COLUMN thread INTEGER DEFAULT 0;
"#,
    )
    .await?;

    migrate(
        44,
        r#"
ALTER TABLE msgs ADD COLUMN mime_headers TEXT;
"#,
    )
    .await?;

    migrate(
        46,
        r#"
ALTER TABLE msgs ADD COLUMN mime_in_reply_to TEXT;
ALTER TABLE msgs ADD COLUMN mime_references TEXT;
"#,
    )
    .await?;

    migrate(
        47,
        r#"
ALTER TABLE jobs ADD COLUMN tries INTEGER DEFAULT 0;
"#,
    )
    .await?;

    // NOTE: move_state is not used anymore
    migrate(
        48,
        r#"
ALTER TABLE msgs ADD COLUMN move_state INTEGER DEFAULT 1;
"#,
    )
    .await?;

    migrate(
        49,
        r#"
ALTER TABLE chats ADD COLUMN gossiped_timestamp INTEGER DEFAULT 0;
"#,
    )
    .await?;

    if dbversion < 50 {
        info!(context, "[migration] v50");
        // installations <= 0.100.1 used DC_SHOW_EMAILS_ALL implicitly;
        // keep this default and use DC_SHOW_EMAILS_NO
        // only for new installations
        if exists_before_update {
            sql.set_raw_config_int(context, "show_emails", ShowEmails::All as i32)
                .await?;
        }

        sql.set_raw_config_int(context, "dbversion", 50).await?;
    }

    // the messages containing _only_ locations
    // are also added to the database as _hidden_.
    migrate(
        53,
        r#"
CREATE TABLE locations ( 
  id INTEGER PRIMARY KEY AUTOINCREMENT, 
  latitude REAL DEFAULT 0.0, 
  longitude REAL DEFAULT 0.0, 
  accuracy REAL DEFAULT 0.0, 
  timestamp INTEGER DEFAULT 0, 
  chat_id INTEGER DEFAULT 0, 
  from_id INTEGER DEFAULT 0);
CREATE INDEX locations_index1 ON locations (from_id);
CREATE INDEX locations_index2 ON locations (timestamp);
ALTER TABLE chats ADD COLUMN locations_send_begin INTEGER DEFAULT 0;
ALTER TABLE chats ADD COLUMN locations_send_until INTEGER DEFAULT 0;
ALTER TABLE chats ADD COLUMN locations_last_sent INTEGER DEFAULT 0;
CREATE INDEX chats_index3 ON chats (locations_send_until);
"#,
    )
    .await?;

    migrate(
        54,
        r#"
ALTER TABLE msgs ADD COLUMN location_id INTEGER DEFAULT 0;
CREATE INDEX msgs_index6 ON msgs (location_id);
"#,
    )
    .await?;

    migrate(
        55,
        r#"
ALTER TABLE locations ADD COLUMN independent INTEGER DEFAULT 0;
"#,
    )
    .await?;

    migrate(
        59,
        r#"
CREATE TABLE devmsglabels (
  id INTEGER PRIMARY KEY AUTOINCREMENT, 
  label TEXT, 
  msg_id INTEGER DEFAULT 0);
CREATE INDEX devmsglabels_index1 ON devmsglabels (label);
"#,
    )
    .await?;

    // records in the devmsglabels are kept when the message is deleted.
    // so, msg_id may or may not exist.
    if dbversion < 59 && exists_before_update && sql.get_raw_config_int("bcc_self").await.is_none()
    {
        sql.set_raw_config_int(context, "bcc_self", 1).await?;
    }

    migrate(
        60,
        r#"
ALTER TABLE chats ADD COLUMN created_timestamp INTEGER DEFAULT 0;
"#,
    )
    .await?;

    migrate(
        61,
        r#"
ALTER TABLE contacts ADD COLUMN selfavatar_sent INTEGER DEFAULT 0;
"#,
    )
    .await?;

    migrate(
        62,
        r#"
ALTER TABLE chats ADD COLUMN muted_until INTEGER DEFAULT 0;
"#,
    )
    .await?;

    migrate(
        63,
        r#"
UPDATE chats SET grpid='' WHERE type=100;
"#,
    )
    .await?;

    migrate(
        64,
        r#"
ALTER TABLE msgs ADD COLUMN error TEXT DEFAULT '';
"#,
    )
    .await?;

    Ok(())
}
