//! Database deserialization module.

use std::str::FromStr;

use anyhow::{anyhow, bail, Context as _, Result};
use bstr::BString;
use rusqlite::Transaction;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader};

use super::Sql;

/// Token of bencoding.
#[derive(Debug)]
enum BencodeToken {
    /// End "e".
    End,

    /// Length-prefixed bytestring.
    ByteString(BString),

    /// Integer like "i1000e".
    Integer(i64),

    /// Beginning of the list "l".
    List,

    /// Beginning of the dictionary "d".
    Dictionary,
}

/// Tokenizer for bencoded stream.
struct BencodeTokenizer<R: AsyncRead + Unpin> {
    r: BufReader<R>,

    peeked_token: Option<BencodeToken>,
}

impl<R: AsyncRead + Unpin> BencodeTokenizer<R> {
    fn new(r: R) -> Self {
        let r = BufReader::new(r);
        Self {
            r,
            peeked_token: None,
        }
    }

    async fn peek_token(&mut self) -> Result<Option<&BencodeToken>> {
        if self.peeked_token.is_none() {
            self.peeked_token = self.next_token().await?;
        }
        Ok(self.peeked_token.as_ref())
    }

    async fn next_token(&mut self) -> Result<Option<BencodeToken>> {
        if let Some(token) = self.peeked_token.take() {
            return Ok(Some(token));
        }

        let buf = self.r.fill_buf().await?;
        match buf.first() {
            None => Ok(None),
            Some(b'e') => {
                self.r.consume(1);
                Ok(Some(BencodeToken::End))
            }
            Some(b'l') => {
                self.r.consume(1);
                Ok(Some(BencodeToken::List))
            }
            Some(b'd') => {
                self.r.consume(1);
                Ok(Some(BencodeToken::Dictionary))
            }
            Some(b'i') => {
                let mut ibuf = Vec::new();
                let n = self.r.read_until(b'e', &mut ibuf).await?;
                if n == 0 {
                    Err(anyhow!("unexpected end of file while reading integer"))
                } else {
                    let num_bytes = ibuf.get(1..n - 1).context("out of bounds")?;
                    let num_str = std::str::from_utf8(num_bytes).context("invalid utf8 number")?;
                    let num =
                        i64::from_str(num_str).context("cannot parse the number {num_str:?}")?;
                    Ok(Some(BencodeToken::Integer(num)))
                }
            }
            Some(&x) => {
                if x.is_ascii_digit() {
                    let mut size_buf = Vec::new();
                    let n = self.r.read_until(b':', &mut size_buf).await?;
                    if n == 0 {
                        return Err(anyhow!("unexpected end of file while reading string"));
                    } else {
                        let size_bytes = size_buf.get(0..n - 1).context("out of bounds")?;
                        let size_str =
                            std::str::from_utf8(size_bytes).context("invalid utf8 number")?;
                        let size = usize::from_str(size_str)
                            .with_context(|| format!("cannot parse length prefix {size_str:?}"))?;
                        let mut str_buf = vec![0; size];
                        self.r.read_exact(&mut str_buf).await.with_context(|| {
                            format!("error while reading a string of {size} bytes")
                        })?;
                        return Ok(Some(BencodeToken::ByteString(BString::new(str_buf))));
                    }
                }
                Err(anyhow!("unexpected byte {x:?}"))
            }
        }
    }
}

struct Decoder<R: AsyncRead + Unpin> {
    tokenizer: BencodeTokenizer<R>,
}

impl<R: AsyncRead + Unpin> Decoder<R> {
    fn new(r: R) -> Self {
        let tokenizer = BencodeTokenizer::new(r);
        Self { tokenizer }
    }

    /// Expects a token.
    ///
    /// Returns an error on unexpected EOF.
    async fn expect_token(&mut self) -> Result<BencodeToken> {
        let token = self
            .tokenizer
            .next_token()
            .await?
            .context("unexpected end of file")?;
        Ok(token)
    }

    /// Expects a token without consuming it.
    async fn peek_token(&mut self) -> Result<&BencodeToken> {
        let token = self
            .tokenizer
            .peek_token()
            .await?
            .context("unexpected end of file")?;
        Ok(token)
    }

    async fn expect_end(&mut self) -> Result<()> {
        match self.expect_token().await? {
            BencodeToken::End => Ok(()),
            token => Err(anyhow!("unexpected token {token:?}, expected end")),
        }
    }

    /// Tries to read a dictionary token.
    ///
    /// Returns an error on EOF or unexpected token.
    async fn expect_dictionary(&mut self) -> Result<()> {
        match self.expect_token().await? {
            BencodeToken::Dictionary => Ok(()),
            token => Err(anyhow!("unexpected token {token:?}, expected dictionary")),
        }
    }

    /// Tries to read a dictionary or end token.
    ///
    /// Returns true if the dictionary starts and false if the end is detected.
    /// Returns an error on EOF or unexpected token.
    async fn expect_dictionary_opt(&mut self) -> Result<bool> {
        match self.expect_token().await? {
            BencodeToken::Dictionary => Ok(true),
            BencodeToken::End => Ok(false),
            token => Err(anyhow!(
                "unexpected token {token:?}, expected dictionary or end"
            )),
        }
    }

    /// Tries to read a list token.
    ///
    /// Returns an error on EOF or unexpected token.
    async fn expect_list(&mut self) -> Result<()> {
        match self.expect_token().await? {
            BencodeToken::List => Ok(()),
            token => Err(anyhow!("unexpected token {token:?}, expected list")),
        }
    }

    /// Tries to read a bytestring.
    ///
    /// Returns an error on EOF or unexpected token.
    async fn expect_bstring(&mut self) -> Result<BString> {
        match self.expect_token().await? {
            BencodeToken::ByteString(s) => Ok(s),
            token => Err(anyhow!("unexpected token {token:?}, expected bytestring")),
        }
    }

    /// Tries to read a bytestring or end token.
    ///
    /// Returns None if end token is is detected.
    /// Returns an error on EOF or unexpected token.
    async fn expect_bstring_opt(&mut self) -> Result<Option<BString>> {
        match self.expect_token().await? {
            BencodeToken::ByteString(s) => Ok(Some(s)),
            BencodeToken::End => Ok(None),
            token => Err(anyhow!(
                "unexpected token {token:?}, expected bytestring or end"
            )),
        }
    }

    /// Tries to read an UTF-8 string.
    async fn expect_string(&mut self) -> Result<String> {
        let s = self.expect_bstring().await?.try_into()?;
        Ok(s)
    }

    /// Tries to read an UTF-8 string or end token.
    async fn expect_string_opt(&mut self) -> Result<Option<String>> {
        if let Some(bstr) = self.expect_bstring_opt().await? {
            Ok(Some(bstr.try_into()?))
        } else {
            Ok(None)
        }
    }
    /// Tries to read a binary blob.
    async fn expect_blob(&mut self) -> Result<Vec<u8>> {
        let s = self.expect_bstring().await?;
        Ok(s.into())
    }

    /// Tries to read a string dictionary key.
    ///
    /// Returns `None` if the end of dictionary is reached.
    async fn expect_key(&mut self, expected_key: &str) -> Result<()> {
        match self.expect_token().await? {
            BencodeToken::ByteString(key) => {
                if key.as_slice() == expected_key.as_bytes() {
                    Ok(())
                } else {
                    Err(anyhow!("unexpected key {key}, expected key {expected_key}"))
                }
            }
            token => Err(anyhow!("unexpected token {token:?}, expected string")),
        }
    }

    async fn expect_key_opt(&mut self, expected_key: &str) -> Result<bool> {
        match self.peek_token().await? {
            BencodeToken::ByteString(key) => {
                if key.as_slice() == expected_key.as_bytes() {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            BencodeToken::End => Ok(false),
            token => Err(anyhow!("unexpected token {token:?}, expected string")),
        }
    }

    async fn expect_i64(&mut self) -> Result<i64> {
        let token = self.expect_token().await?;
        match token {
            BencodeToken::Integer(i) => Ok(i),
            t => Err(anyhow!("unexpected token {t:?}, expected integer")),
        }
    }

    async fn expect_u32(&mut self) -> Result<u32> {
        let i = u32::try_from(self.expect_i64().await?).context("failed to convert to u32")?;
        Ok(i)
    }

    async fn expect_f64(&mut self) -> Result<f64> {
        let buffer = self.expect_blob().await?;
        Ok(f64::from_be_bytes(
            buffer
                .try_into()
                .map_err(|_| anyhow!("unexpected end of file"))?,
        ))
    }

    async fn expect_bool(&mut self) -> Result<bool> {
        let i = self.expect_u32().await?;
        Ok(i != 0)
    }

    async fn deserialize_config(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut dbversion_found = false;

        let mut stmt = tx.prepare("INSERT INTO config (keyname, value) VALUES (?, ?)")?;

        self.expect_dictionary().await?;
        loop {
            let token = self.expect_token().await?;
            match token {
                BencodeToken::ByteString(key) => {
                    let value = self.expect_string().await?;
                    println!("{key:?}={value:?}");

                    if key.as_slice() == b"dbversion" {
                        if dbversion_found {
                            bail!("dbversion key found twice in the config");
                        } else {
                            dbversion_found = true;
                        }

                        if value != "99" {
                            bail!("unsupported serialized database version {value:?}, expected 99");
                        }
                    }

                    stmt.execute([key.as_slice(), value.as_bytes()])?;
                }
                BencodeToken::End => break,
                t => return Err(anyhow!("unexpected token {t:?}, expected config key")),
            }
        }

        if !dbversion_found {
            bail!("no dbversion found in the config");
        }
        Ok(())
    }

    async fn deserialize_acpeerstates(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut stmt = tx.prepare(
            "
INSERT INTO
acpeerstates (addr,
              gossip_key, gossip_key_fingerprint, gossip_timestamp,
              last_seen, last_seen_autocrypt,
              prefer_encrypted,
              public_key, public_key_fingerprint,
              verified_key, verified_key_fingerprint)
VALUES       (:addr,
              :gossip_key, :gossip_key_fingerprint, :gossip_timestamp,
              :last_seen, :last_seen_autocrypt,
              :prefer_encrypted,
              :public_key, :public_key_fingerprint,
              :verified_key, :verified_key_fingerprint)",
        )?;

        self.expect_list().await?;
        while self.expect_dictionary_opt().await? {
            self.expect_key("addr").await?;
            let addr = self.expect_string().await?;

            let gossip_key = if self.expect_key_opt("gossip_key").await? {
                Some(self.expect_blob().await?)
            } else {
                None
            };

            let gossip_key_fingerprint = if self.expect_key_opt("gossip_key_fingerprint").await? {
                Some(self.expect_string().await?)
            } else {
                None
            };

            self.expect_key("gossip_timestamp").await?;
            let gossip_timestamp = self.expect_i64().await?;

            self.expect_key("last_seen").await?;
            let last_seen = self.expect_i64().await?;

            self.expect_key("last_seen_autocrypt").await?;
            let last_seen_autocrypt = self.expect_i64().await?;

            self.expect_key("prefer_encrypted").await?;
            let prefer_encrypted = self.expect_i64().await?;

            let public_key = if self.expect_key_opt("public_key").await? {
                Some(self.expect_blob().await?)
            } else {
                None
            };

            let public_key_fingerprint = if self.expect_key_opt("public_key_fingerprint").await? {
                Some(self.expect_string().await?)
            } else {
                None
            };

            let verified_key = if self.expect_key_opt("verified_key").await? {
                Some(self.expect_blob().await?)
            } else {
                None
            };

            let verified_key_fingerprint =
                if self.expect_key_opt("verified_key_fingerprint").await? {
                    Some(self.expect_string().await?)
                } else {
                    None
                };

            self.expect_end().await?;

            stmt.execute(named_params! {
            ":addr": addr,
            ":gossip_key": gossip_key,
            ":gossip_key_fingerprint": gossip_key_fingerprint,
            ":gossip_timestamp": gossip_timestamp,
            ":last_seen": last_seen,
            ":last_seen_autocrypt": last_seen_autocrypt,
            ":prefer_encrypted": prefer_encrypted,
            ":public_key": public_key,
            ":public_key_fingerprint": public_key_fingerprint,
            ":verified_key": verified_key,
            ":verified_key_fingerprint": verified_key_fingerprint
            })?;
        }
        Ok(())
    }

    async fn deserialize_chats(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut stmt = tx.prepare(
            "
INSERT INTO
chats (id,
       type,
       name,
       blocked,
       grpid,
       param,
       archived,
       gossiped_timestamp,
       locations_send_begin,
       locations_send_until,
       locations_last_sent,
       created_timestamp,
       muted_until,
       ephemeral_timer,
       protected)
VALUES (:id,
        :type,
        :name,
        :blocked,
        :grpid,
        :param,
        :archived,
        :gossiped_timestamp,
        :locations_send_begin,
        :locations_send_until,
        :locations_last_sent,
        :created_timestamp,
        :muted_until,
        :ephemeral_timer,
        :protected)",
        )?;

        self.expect_list().await?;
        while self.expect_dictionary_opt().await? {
            self.expect_key("archived").await?;
            let archived = self.expect_bool().await?;

            self.expect_key("blocked").await?;
            let blocked = self.expect_u32().await?;

            self.expect_key("created_timestamp").await?;
            let created_timestamp = self.expect_i64().await?;

            self.expect_key("ephemeral_timer").await?;
            let ephemeral_timer = self.expect_i64().await?;

            self.expect_key("gossiped_timestamp").await?;
            let gossiped_timestamp = self.expect_i64().await?;

            self.expect_key("grpid").await?;
            let grpid = self.expect_string().await?;

            self.expect_key("id").await?;
            let id = self.expect_u32().await?;

            self.expect_key("locations_last_sent").await?;
            let locations_last_sent = self.expect_i64().await?;

            self.expect_key("locations_send_begin").await?;
            let locations_send_begin = self.expect_i64().await?;

            self.expect_key("locations_send_until").await?;
            let locations_send_until = self.expect_i64().await?;

            self.expect_key("muted_until").await?;
            let muted_until = self.expect_i64().await?;

            self.expect_key("name").await?;
            let name = self.expect_string().await?;

            self.expect_key("param").await?;
            let param = self.expect_string().await?;

            self.expect_key("protected").await?;
            let protected = self.expect_u32().await?;

            self.expect_key("type").await?;
            let typ = self.expect_u32().await?;

            stmt.execute(named_params! {
            ":id": id,
            ":type": typ,
            ":name": name,
            ":blocked": blocked,
            ":grpid": grpid,
            ":param": param,
            ":archived": archived,
            ":gossiped_timestamp": gossiped_timestamp,
            ":locations_send_begin": locations_send_begin,
            ":locations_send_until": locations_send_until,
            ":locations_last_sent": locations_last_sent,
            ":created_timestamp": created_timestamp,
            ":muted_until": muted_until,
            ":ephemeral_timer": ephemeral_timer,
            ":protected": protected
            })?;

            self.expect_end().await?;
        }
        Ok(())
    }

    async fn deserialize_chats_contacts(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut stmt = tx.prepare(
            "
INSERT INTO
chats_contacts (chat_id, contact_id)
VALUES (:chat_id, :contact_id)",
        )?;

        self.expect_list().await?;

        while self.expect_dictionary_opt().await? {
            self.expect_key("chat_id").await?;
            let chat_id = self.expect_u32().await?;

            self.expect_key("contact_id").await?;
            let contact_id = self.expect_u32().await?;

            self.expect_end().await?;

            stmt.execute(named_params! {
                ":chat_id": chat_id,
                ":contact_id": contact_id
            })?;
        }
        Ok(())
    }

    async fn deserialize_contacts(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut stmt = tx.prepare(
            "
INSERT INTO
contacts (id,
          name,
          addr,
          origin,
          blocked,
          last_seen,
          param,
          authname,
          selfavatar_sent,
          status)
VALUES (:id,
        :name,
        :addr,
        :origin,
        :blocked,
        :last_seen,
        :param,
        :authname,
        :selfavatar_sent,
        :status)",
        )?;

        self.expect_list().await?;

        while self.expect_dictionary_opt().await? {
            self.expect_key("addr").await?;
            let addr = self.expect_string().await?;

            self.expect_key("authname").await?;
            let authname = self.expect_string().await?;

            self.expect_key("blocked").await?;
            let blocked = self.expect_bool().await?;

            self.expect_key("id").await?;
            let id = self.expect_u32().await?;

            self.expect_key("last_seen").await?;
            let last_seen = self.expect_i64().await?;

            self.expect_key("name").await?;
            let name = self.expect_string().await?;

            self.expect_key("origin").await?;
            let origin = self.expect_u32().await?;

            self.expect_key("param").await?;
            let param = self.expect_string().await?;

            self.expect_key("selfavatar_sent").await?;
            let selfavatar_sent = self.expect_i64().await?;

            let status = if self.expect_key_opt("status").await? {
                self.expect_string().await?
            } else {
                "".to_string()
            };

            self.expect_end().await?;

            stmt.execute(named_params! {
                ":id": id,
                ":name": name,
                ":addr": addr,
                ":origin": origin,
                ":blocked": blocked,
                ":last_seen": last_seen,
                ":param": param,
                ":authname": authname,
                ":selfavatar_sent": selfavatar_sent,
                ":status": status
            })?;
        }
        Ok(())
    }

    async fn deserialize_dns_cache(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut stmt = tx.prepare(
            "
INSERT INTO
contacts (hostname,
          address,
          timestamp)
VALUES (:hostname,
        :address,
        :timestamp)",
        )?;
        self.expect_list().await?;
        self.skip_until_end().await?;

        while self.expect_dictionary_opt().await? {
            self.expect_key("address").await?;
            let address = self.expect_string().await?;

            self.expect_key("hostname").await?;
            let hostname = self.expect_string().await?;

            self.expect_key("timestamp").await?;
            let timestamp = self.expect_string().await?;

            self.expect_end().await?;

            stmt.execute(named_params! {
                ":hostname": hostname,
                ":address": address,
                ":timestamp": timestamp
            })?;
        }
        Ok(())
    }

    async fn deserialize_imap(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut stmt = tx.prepare(
            "
INSERT INTO
imap (id,
      rfc724_mid,
      folder,
      target,
      uid,
      uidvalidity)
VALUES (:id,
        :rfc724_mid,
        :folder,
        :target,
        :uid,
        :uidvalidity)",
        )?;

        self.expect_list().await?;

        while self.expect_dictionary_opt().await? {
            self.expect_key("folder").await?;
            let folder = self.expect_string().await?;

            self.expect_key("id").await?;
            let id = self.expect_string().await?;

            self.expect_key("rfc724_mid").await?;
            let rfc724_mid = self.expect_string().await?;

            self.expect_key("target").await?;
            let target = self.expect_string().await?;

            self.expect_key("uid").await?;
            let uid = self.expect_i64().await?;

            self.expect_key("uidvalidity").await?;
            let uidvalidity = self.expect_i64().await?;

            self.expect_end().await?;

            stmt.execute(named_params! {
                ":id": id,
                ":rfc724_mid": rfc724_mid,
                ":folder": folder,
                ":target": target,
                ":uid": uid,
                ":uidvalidity": uidvalidity
            })?;
        }

        Ok(())
    }

    async fn deserialize_imap_sync(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut stmt = tx.prepare(
            "
INSERT INTO
imap_sync (folder,
           uidvalidity,
           uid_next,
           modseq)
VALUES (:folder,
        :uidvalidity,
        :uid_next,
        :modseq)",
        )?;

        self.expect_list().await?;

        while self.expect_dictionary_opt().await? {
            self.expect_key("folder").await?;
            let folder = self.expect_string().await?;

            self.expect_key("modseq").await?;
            let modseq = self.expect_i64().await?;

            self.expect_key("uidnext").await?;
            let uidnext = self.expect_i64().await?;

            self.expect_key("uidvalidity").await?;
            let uidvalidity = self.expect_i64().await?;

            self.expect_end().await?;

            stmt.execute(named_params! {
                ":folder": folder,
                ":uidvalidity": uidvalidity,
                ":uid_next": uidnext,
                ":modseq": modseq
            })?;
        }

        Ok(())
    }

    async fn deserialize_keypairs(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut stmt = tx.prepare(
            "
INSERT INTO
keypairs (id,
          addr,
          is_default,
          private_key,
          public_key,
          created)
VALUES (:id,
        :addr,
        :is_default,
        :private_key,
        :public_key,
        :created)",
        )?;

        self.expect_list().await?;

        while self.expect_dictionary_opt().await? {
            self.expect_key("addr").await?;
            let addr = self.expect_string().await?;

            self.expect_key("created").await?;
            let created = self.expect_i64().await?;

            self.expect_key("id").await?;
            let id = self.expect_u32().await?;

            self.expect_key("is_default").await?;
            let is_default = self.expect_bool().await?;

            self.expect_key("private_key").await?;
            let private_key = self.expect_blob().await?;

            self.expect_key("public_key").await?;
            let public_key = self.expect_blob().await?;

            self.expect_end().await?;

            stmt.execute(named_params! {
                ":id": id,
                ":addr": addr,
                ":is_default": is_default,
                ":private_key": private_key,
                ":public_key": public_key,
                ":created": created,
            })?;
        }

        Ok(())
    }

    async fn deserialize_leftgroups(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        self.expect_list().await?;

        while let Some(grpid) = self.expect_string_opt().await? {
            tx.execute("INSERT INTO leftgrps (grpid) VALUES (?)", (grpid,))?;
        }

        Ok(())
    }

    async fn deserialize_locations(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut stmt = tx.prepare(
            "
INSERT INTO
locations (id,
           latitude,
           longitude,
           accuracy,
           timestamp,
           chat_id,
           from_id,
           independent)
VALUES (:id,
        :latitude,
        :longitude,
        :accuracy,
        :timestamp,
        :chat_id,
        :from_id,
        :independent)",
        )?;

        self.expect_list().await?;

        while self.expect_dictionary_opt().await? {
            self.expect_key("accuracy").await?;
            let accuracy = self.expect_f64().await?;

            self.expect_key("chat_id").await?;
            let chat_id = self.expect_f64().await?;

            self.expect_key("from_id").await?;
            let from_id = self.expect_u32().await?;

            self.expect_key("id").await?;
            let id = self.expect_i64().await?;

            self.expect_key("independent").await?;
            let independent = self.expect_u32().await?;

            self.expect_key("latitude").await?;
            let latitude = self.expect_f64().await?;

            self.expect_key("longitude").await?;
            let longitude = self.expect_f64().await?;

            self.expect_key("timestamp").await?;
            let timestamp = self.expect_i64().await?;

            self.expect_end().await?;

            stmt.execute(named_params! {
                ":id": id,
                ":latitude": latitude,
                ":longitude": longitude,
                ":accuracy": accuracy,
                ":timestamp": timestamp,
                ":chat_id": chat_id,
                ":from_id": from_id,
                ":independent": independent
            })?;
        }

        Ok(())
    }

    async fn deserialize_mdns(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut stmt = tx.prepare(
            "
INSERT INTO
msgs_mdns (msg_id,
           contact_id,
           timestamp_sent)
VALUES (:msg_id,
        :contact_id,
        :timestamp_sent)",
        )?;

        self.expect_list().await?;

        while self.expect_dictionary_opt().await? {
            self.expect_key("contact_id").await?;
            let contact_id = self.expect_u32().await?;

            self.expect_key("msg_id").await?;
            let msg_id = self.expect_u32().await?;

            self.expect_key("timestamp_sent").await?;
            let timestamp_sent = self.expect_i64().await?;

            self.expect_end().await?;

            stmt.execute(named_params! {
                ":msg_id": msg_id,
                ":contact_id": contact_id,
                ":timestamp_sent": timestamp_sent
            })?;
        }

        Ok(())
    }

    async fn deserialize_messages(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut stmt = tx.prepare(
            "
INSERT INTO
msgs (id,
      rfc724_mid,
      chat_id,
      from_id, to_id,
      timestamp,
      type,
      state,
      msgrmsg,
      bytes,
      txt,
      txt_raw,
      param,
      timestamp_sent,
      timestamp_rcvd,
      hidden,
      mime_headers,
      mime_in_reply_to,
      mime_references,
      location_id)
VALUES (:id,
        :rfc724_mid,
        :chat_id,
        :from_id, :to_id,
        :timestamp,
        :type,
        :state,
        :msgrmsg,
        :bytes,
        :txt,
        :txt_raw,
        :param,
        :timestamp_sent,
        :timestamp_rcvd,
        :hidden,
        :mime_headers,
        :mime_in_reply_to,
        :mime_references,
        :location_id)",
        )?;

        self.expect_list().await?;

        while self.expect_dictionary_opt().await? {
            self.expect_key("bytes").await?;
            let bytes = self.expect_i64().await?;

            self.expect_key("chat_id").await?;
            let chat_id = self.expect_i64().await?;

            self.expect_key("from_id").await?;
            let from_id = self.expect_i64().await?;

            self.expect_key("hidden").await?;
            let hidden = self.expect_i64().await?;

            self.expect_key("id").await?;
            let id = self.expect_i64().await?;

            self.expect_key("location_id").await?;
            let location_id = self.expect_i64().await?;

            self.expect_key("mime_headers").await?;
            let mime_headers = self.expect_blob().await?;

            let mime_in_reply_to = if self.expect_key_opt("mime_in_reply_to").await? {
                Some(self.expect_string().await?)
            } else {
                None
            };

            let mime_references = if self.expect_key_opt("mime_references").await? {
                Some(self.expect_string().await?)
            } else {
                None
            };

            self.expect_key("msgrmsg").await?;
            let msgrmsg = self.expect_i64().await?;

            self.expect_key("param").await?;
            let param = self.expect_string().await?;

            self.expect_key("rfc724_mid").await?;
            let rfc724_mid = self.expect_string().await?;

            self.expect_key("state").await?;
            let state = self.expect_i64().await?;

            self.expect_key("timestamp").await?;
            let timestamp = self.expect_i64().await?;

            self.expect_key("timestamp_rcvd").await?;
            let timestamp_rcvd = self.expect_i64().await?;

            self.expect_key("timestamp_sent").await?;
            let timestamp_sent = self.expect_i64().await?;

            self.expect_key("to_id").await?;
            let to_id = self.expect_i64().await?;

            self.expect_key("txt").await?;
            let txt = self.expect_string().await?;

            self.expect_key("txt_raw").await?;
            let txt_raw = self.expect_string().await?;

            self.expect_key("type").await?;
            let typ = self.expect_i64().await?;

            self.expect_end().await?;

            stmt.execute(named_params! {
                ":id": id,
                ":rfc724_mid": rfc724_mid,
                ":chat_id": chat_id,
                ":from_id": from_id,
                ":to_id": to_id,
                ":timestamp": timestamp,
                ":type": typ,
                ":state": state,
                ":msgrmsg": msgrmsg,
                ":bytes": bytes,
                ":txt": txt,
                ":txt_raw": txt_raw,
                ":param": param,
                ":timestamp_sent": timestamp_sent,
                ":timestamp_rcvd": timestamp_rcvd,
                ":hidden": hidden,
                ":mime_headers": mime_headers,
                ":mime_in_reply_to": mime_in_reply_to,
                ":mime_references": mime_references,
                ":location_id": location_id
            })?;
        }

        Ok(())
    }

    async fn deserialize_msgs_status_updates(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut stmt = tx.prepare(
            "
INSERT INTO
msgs_status_updates (id,
                     msg_id,
                     update_item)
VALUES (:id,
        :msg_id,
        :update_item)",
        )?;

        self.expect_list().await?;

        while self.expect_dictionary_opt().await? {
            self.expect_key("id").await?;
            let id = self.expect_i64().await?;

            self.expect_key("msg_id").await?;
            let msg_id = self.expect_i64().await?;

            self.expect_key("update_item").await?;
            let update_item = self.expect_u32().await?;

            self.expect_end().await?;

            stmt.execute(named_params! {
                ":id": id,
                ":msg_id": msg_id,
                ":update_item": update_item,
            })?;
        }

        Ok(())
    }

    async fn deserialize_reactions(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut stmt = tx.prepare(
            "
INSERT INTO
reactions (msg_id,
           contact_id,
           reaction)
VALUES (:msg_id,
        :contact_id,
        :reaction)",
        )?;

        self.expect_list().await?;

        while self.expect_dictionary_opt().await? {
            self.expect_key("msg_id").await?;
            let msg_id = self.expect_u32().await?;

            self.expect_key("contact_id").await?;
            let contact_id = self.expect_u32().await?;

            self.expect_key("reaction").await?;
            let reaction = self.expect_string().await?;

            self.expect_end().await?;

            stmt.execute(named_params! {
                ":msg_id": msg_id,
                ":contact_id": contact_id,
                ":reaction": reaction,
            })?;
        }

        Ok(())
    }

    async fn deserialize_sending_domains(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut stmt = tx.prepare(
            "
INSERT INTO sending_domains
        (domain,  dkim_works)
VALUES (:domain, :dkim_works)",
        )?;

        self.expect_list().await?;

        while self.expect_dictionary_opt().await? {
            self.expect_key("domain").await?;
            let domain = self.expect_string().await?;

            self.expect_key("dkim_works").await?;
            let dkim_works = self.expect_i64().await?;

            self.expect_end().await?;

            stmt.execute(named_params! {
                ":domain": domain,
                ":dkim_works": dkim_works,
            })?;
        }

        Ok(())
    }

    async fn deserialize_tokens(&mut self, tx: &mut Transaction<'_>) -> Result<()> {
        let mut stmt = tx.prepare(
            "
INSERT INTO tokens
        (id,  namespc,  foreign_id,  token,  timestamp)
VALUES (:id, :namespc, :foreign_id, :token, :timestamp)",
        )?;

        self.expect_list().await?;

        while self.expect_dictionary_opt().await? {
            self.expect_key("foreign_id").await?;
            let foreign_id = self.expect_u32().await?;

            self.expect_key("id").await?;
            let id = self.expect_i64().await?;

            self.expect_key("namespace").await?;
            let namespace = self.expect_u32().await?;

            self.expect_key("timestamp").await?;
            let timestamp = self.expect_i64().await?;

            self.expect_key("token").await?;
            let token = self.expect_string().await?;

            self.expect_end().await?;

            stmt.execute(named_params! {
                ":id": id,
                ":namespc": namespace,
                ":foreign_id": foreign_id,
                ":token": token,
                ":timestamp": timestamp,
            })?;
        }

        Ok(())
    }

    async fn skip_until_end(&mut self) -> Result<()> {
        let mut level: usize = 0;
        loop {
            let token = self.expect_token().await?;
            match token {
                BencodeToken::End => {
                    if level == 0 {
                        return Ok(());
                    } else {
                        level -= 1;
                    }
                }
                BencodeToken::ByteString(_) | BencodeToken::Integer(_) => {}
                BencodeToken::List | BencodeToken::Dictionary => level += 1,
            }
        }
    }

    async fn deserialize(mut self, mut tx: Transaction<'_>) -> Result<()> {
        self.expect_dictionary().await?;

        self.expect_key("_config").await?;
        self.deserialize_config(&mut tx)
            .await
            .context("deserialize_config")?;

        self.expect_key("acpeerstates").await?;
        self.deserialize_acpeerstates(&mut tx)
            .await
            .context("deserialize_acpeerstates")?;

        self.expect_key("chats").await?;
        self.deserialize_chats(&mut tx)
            .await
            .context("deserialize_chats")?;

        self.expect_key("chats_contacts").await?;
        self.deserialize_chats_contacts(&mut tx)
            .await
            .context("deserialize_chats_contacts")?;

        self.expect_key("contacts").await?;
        self.deserialize_contacts(&mut tx)
            .await
            .context("deserialize_contacts")?;

        self.expect_key("dns_cache").await?;
        self.deserialize_dns_cache(&mut tx)
            .await
            .context("deserialize_dns_cache")?;

        self.expect_key("imap").await?;
        self.deserialize_imap(&mut tx)
            .await
            .context("deserialize_imap")?;

        self.expect_key("imap_sync").await?;
        self.deserialize_imap_sync(&mut tx)
            .await
            .context("deserialize_imap_sync")?;

        self.expect_key("keypairs").await?;
        self.deserialize_keypairs(&mut tx)
            .await
            .context("deserialize_keypairs")?;

        self.expect_key("leftgroups").await?;
        self.deserialize_leftgroups(&mut tx)
            .await
            .context("deserialize_leftgroups")?;

        self.expect_key("locations").await?;
        self.deserialize_locations(&mut tx)
            .await
            .context("deserialize_locations")?;

        self.expect_key("mdns").await?;
        self.deserialize_mdns(&mut tx)
            .await
            .context("deserialize_mdns")?;

        self.expect_key("messages").await?;
        self.deserialize_messages(&mut tx)
            .await
            .context("deserialize_messages")?;

        self.expect_key("msgs_status_updates").await?;
        self.deserialize_msgs_status_updates(&mut tx)
            .await
            .context("deserialize_msgs_status_updates")?;

        self.expect_key("reactions").await?;
        self.deserialize_reactions(&mut tx)
            .await
            .context("deserialize_reactions")?;

        self.expect_key("sending_domains").await?;
        self.deserialize_sending_domains(&mut tx)
            .await
            .context("deserialize_sending_domains")?;

        self.expect_key("tokens").await?;
        self.deserialize_tokens(&mut tx)
            .await
            .context("deserialize_tokens")?;

        self.expect_end().await?;

        tx.commit()?;
        Ok(())
    }
}

impl Sql {
    /// Deserializes the database from a bytestream.
    pub async fn deserialize(&self, r: impl AsyncRead + Unpin) -> Result<()> {
        let mut conn = self.get_connection().await?;

        // Start a write transaction to take a database snapshot.
        let transaction = conn.transaction()?;

        let decoder = Decoder::new(r);
        decoder.deserialize(transaction).await?;

        Ok(())
    }
}
