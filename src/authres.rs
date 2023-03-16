//! Parsing and handling of the Authentication-Results header.
//! See the comment on [`handle_authres`] for more.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt;

use anyhow::Result;
use mailparse::MailHeaderMap;
use mailparse::ParsedMail;
use once_cell::sync::Lazy;

use crate::config::Config;
use crate::context::Context;
use crate::headerdef::HeaderDef;
use crate::tools::time;
use crate::tools::EmailAddress;

/// `authres` is short for the Authentication-Results header, defined in
/// <https://datatracker.ietf.org/doc/html/rfc8601>, which contains info
/// about whether DKIM and SPF passed.
///
/// To mitigate From forgery, we remember for each sending domain whether it is known
/// to have valid DKIM. If an email from such a domain comes with invalid DKIM,
/// we don't allow changing the autocrypt key.
///
/// See <https://github.com/deltachat/deltachat-core-rust/issues/3507>.
pub(crate) async fn handle_authres(
    context: &Context,
    mail: &ParsedMail<'_>,
    from: &str,
    message_time: i64,
) -> Result<DkimResults> {
    let from_domain = match EmailAddress::new(from) {
        Ok(email) => email.domain,
        Err(e) => {
            // This email is invalid, but don't return an error, we still want to
            // add a stub to the database so that it's not downloaded again
            return Err(anyhow::format_err!("invalid email {}: {:#}", from, e));
        }
    };

    let authres = parse_authres_headers(&mail.get_headers(), &from_domain);
    update_authservid_candidates(context, &authres).await?;
    compute_dkim_results(context, authres, &from_domain, message_time).await
}

#[derive(Debug)]
pub(crate) struct DkimResults {
    /// Whether DKIM passed for this particular e-mail.
    pub dkim_passed: bool,
    /// Whether DKIM is known to work for e-mails coming from the sender's domain,
    /// i.e. whether we expect DKIM to work.
    pub dkim_should_work: bool,
    /// Whether changing the public Autocrypt key should be allowed.
    /// This is false if we expected DKIM to work (dkim_works=true),
    /// but it failed now (dkim_passed=false).
    pub allow_keychange: bool,
}

impl fmt::Display for DkimResults {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "DKIM Results: Passed={}, Works={}, Allow_Keychange={}",
            self.dkim_passed, self.dkim_should_work, self.allow_keychange
        )?;
        if !self.allow_keychange {
            write!(fmt, " KEYCHANGES NOT ALLOWED!!!!")?;
        }
        Ok(())
    }
}

type AuthservId = String;

#[derive(Debug, PartialEq)]
enum DkimResult {
    /// The header explicitly said that DKIM passed
    Passed,
    /// The header explicitly said that DKIM failed
    Failed,
    /// The header didn't say anything about DKIM; this might mean that it wasn't
    /// checked, but it might also mean that it failed. This is because some providers
    /// (e.g. ik.me, mail.ru, posteo.de) don't add `dkim=none` to their
    /// Authentication-Results if there was no DKIM.
    Nothing,
}

type ParsedAuthresHeaders = Vec<(AuthservId, DkimResult)>;

fn parse_authres_headers(
    headers: &mailparse::headers::Headers<'_>,
    from_domain: &str,
) -> ParsedAuthresHeaders {
    let mut res = Vec::new();
    for header_value in headers.get_all_values(HeaderDef::AuthenticationResults.into()) {
        let header_value = remove_comments(&header_value);

        if let Some(mut authserv_id) = header_value.split(';').next() {
            if authserv_id.contains(char::is_whitespace) || authserv_id.is_empty() {
                // Outlook violates the RFC by not adding an authserv-id at all, which we notice
                // because there is whitespace in the first identifier before the ';'.
                // Authentication-Results-parsing still works securely because they remove incoming
                // Authentication-Results headers.
                // We just use an arbitrary authserv-id, it will work for Outlook, and in general,
                // with providers not implementing the RFC correctly, someone can trick us
                // into thinking that an incoming email is DKIM-correct, anyway.
                // The most important thing here is that we have some valid `authserv_id`.
                authserv_id = "invalidAuthservId";
            }
            let dkim_passed = parse_one_authres_header(&header_value, from_domain);
            res.push((authserv_id.to_string(), dkim_passed));
        }
    }

    res
}

/// The headers can contain comments that look like this:
/// ```text
/// Authentication-Results: (this is a comment) gmx.net; (another; comment) dkim=pass;
/// ```
fn remove_comments(header: &str) -> Cow<'_, str> {
    // In Pomsky, this is:
    //     "(" Codepoint* lazy ")"
    // See https://playground.pomsky-lang.org/?text=%22(%22%20Codepoint*%20lazy%20%22)%22
    static RE: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"\([\s\S]*?\)").unwrap());

    RE.replace_all(header, " ")
}

/// Parses a single Authentication-Results header, like:
///
/// ```text
/// Authentication-Results:  gmx.net; dkim=pass header.i=@slack.com
/// ```
fn parse_one_authres_header(header_value: &str, from_domain: &str) -> DkimResult {
    if let Some((before_dkim_part, dkim_to_end)) = header_value.split_once("dkim=") {
        // Check that the character right before `dkim=` is a space or a tab
        // so that we wouldn't e.g. mistake `notdkim=pass` for `dkim=pass`
        if before_dkim_part.ends_with(' ') || before_dkim_part.ends_with('\t') {
            let dkim_part = dkim_to_end.split(';').next().unwrap_or_default();
            let dkim_parts: Vec<_> = dkim_part.split_whitespace().collect();
            if let Some(&"pass") = dkim_parts.first() {
                // DKIM headers contain a header.d or header.i field
                // that says which domain signed. We have to check ourselves
                // that this is the same domain as in the From header.
                let header_d: &str = &format!("header.d={}", &from_domain);
                let header_i: &str = &format!("header.i=@{}", &from_domain);

                if dkim_parts.contains(&header_d) || dkim_parts.contains(&header_i) {
                    // We have found a `dkim=pass` header!
                    return DkimResult::Passed;
                }
            } else {
                // dkim=fail, dkim=none, ...
                return DkimResult::Failed;
            }
        }
    }

    DkimResult::Nothing
}

/// ## About authserv-ids
///
/// After having checked DKIM, our email server adds an Authentication-Results header.
///
/// Now, an attacker could just add an Authentication-Results header that says dkim=pass
/// in order to make us think that DKIM was correct in their From-forged email.
///
/// In order to prevent this, each email server adds its authserv-id to the
/// Authentication-Results header, e.g. Testrun's authserv-id is `testrun.org`, Gmail's
/// is `mx.google.com`. When Testrun gets a mail delivered from outside, it will then
/// remove any Authentication-Results headers whose authserv-id is also `testrun.org`.
///
/// We need to somehow find out the authserv-id(s) of our email server, so that
/// we can use the Authentication-Results with the right authserv-id.
///
/// ## What this function does
///
/// When receiving an email, this function is called and updates the candidates for
/// our server's authserv-id, i.e. what we think our server's authserv-id is.
///
/// Usually, every incoming email has Authentication-Results  with our server's
/// authserv-id, so, the intersection of the existing authserv-ids and the incoming
/// authserv-ids for our server's authserv-id is a good guess for our server's
/// authserv-id. When this intersection is empty, we assume that the authserv-id has
/// changed and start over with the new authserv-ids.
///
/// See [`handle_authres`].
async fn update_authservid_candidates(
    context: &Context,
    authres: &ParsedAuthresHeaders,
) -> Result<()> {
    let mut new_ids: BTreeSet<&str> = authres
        .iter()
        .map(|(authserv_id, _dkim_passed)| authserv_id.as_str())
        .collect();
    if new_ids.is_empty() {
        // The incoming message doesn't contain any authentication results, maybe it's a
        // self-sent or a mailer-daemon message
        return Ok(());
    }

    let old_config = context.get_config(Config::AuthservIdCandidates).await?;
    let old_ids = parse_authservid_candidates_config(&old_config);
    let intersection: BTreeSet<&str> = old_ids.intersection(&new_ids).copied().collect();
    if !intersection.is_empty() {
        new_ids = intersection;
    }
    // If there were no AuthservIdCandidates previously, just start with
    // the ones from the incoming email

    if old_ids != new_ids {
        let new_config = new_ids.into_iter().collect::<Vec<_>>().join(" ");
        context
            .set_config(Config::AuthservIdCandidates, Some(&new_config))
            .await?;
        // Updating the authservid candidates may mean that we now consider
        // emails as "failed" which "passed" previously, so we need to
        // reset our expectation which DKIMs work.
        clear_dkim_works(context).await?
    }
    Ok(())
}

/// Use the parsed authres and the authservid candidates to compute whether DKIM passed
/// and whether a keychange should be allowed.
///
/// We track in the `sending_domains` table whether we get positive Authentication-Results
/// for mails from a contact (meaning that their provider properly authenticates against
/// our provider).
///
/// Once a contact is known to come with positive Authentication-Resutls (dkim: pass),
/// we don't accept Autocrypt key changes if they come with negative Authentication-Results.
async fn compute_dkim_results(
    context: &Context,
    mut authres: ParsedAuthresHeaders,
    from_domain: &str,
    message_time: i64,
) -> Result<DkimResults> {
    let mut dkim_passed = false;

    let ids_config = context.get_config(Config::AuthservIdCandidates).await?;
    let ids = parse_authservid_candidates_config(&ids_config);

    // Remove all foreign authentication results
    authres.retain(|(authserv_id, _dkim_passed)| ids.contains(authserv_id.as_str()));

    if authres.is_empty() {
        // If the authentication results are empty, then our provider doesn't add them
        // and an attacker could just add their own Authentication-Results, making us
        // think that DKIM passed. So, in this case, we can as well assume that DKIM passed.
        dkim_passed = true;
    } else {
        for (_authserv_id, current_dkim_passed) in authres {
            match current_dkim_passed {
                DkimResult::Passed => {
                    dkim_passed = true;
                    break;
                }
                DkimResult::Failed => {
                    dkim_passed = false;
                    break;
                }
                DkimResult::Nothing => {
                    // Continue looking for an Authentication-Results header
                }
            }
        }
    }

    let last_working_timestamp = dkim_works_timestamp(context, from_domain).await?;
    let mut dkim_should_work = dkim_should_work(last_working_timestamp)?;
    if message_time > last_working_timestamp && dkim_passed {
        set_dkim_works_timestamp(context, from_domain, message_time).await?;
        dkim_should_work = true;
    }

    Ok(DkimResults {
        dkim_passed,
        dkim_should_work,
        allow_keychange: dkim_passed || !dkim_should_work,
    })
}

/// Whether DKIM in emails from this domain should be considered to work.
fn dkim_should_work(last_working_timestamp: i64) -> Result<bool> {
    // When we get an email with valid DKIM-Authentication-Results,
    // then we assume that DKIM works for 30 days from this time on.
    let should_work_until = last_working_timestamp + 3600 * 24 * 30;

    let dkim_ever_worked = last_working_timestamp > 0;

    // We're using time() here and not the time when the message
    // claims to have been sent (passed around as `message_time`)
    // because otherwise an attacker could just put a time way
    // in the future into the `Date` header and then we would
    // assume that DKIM doesn't have to be valid anymore.
    let dkim_should_work_now = should_work_until > time();
    Ok(dkim_ever_worked && dkim_should_work_now)
}

async fn dkim_works_timestamp(context: &Context, from_domain: &str) -> Result<i64, anyhow::Error> {
    let last_working_timestamp: i64 = context
        .sql
        .query_get_value(
            "SELECT dkim_works FROM sending_domains WHERE domain=?",
            paramsv![from_domain],
        )
        .await?
        .unwrap_or(0);
    Ok(last_working_timestamp)
}

async fn set_dkim_works_timestamp(
    context: &Context,
    from_domain: &str,
    timestamp: i64,
) -> Result<()> {
    context
        .sql
        .execute(
            "INSERT INTO sending_domains (domain, dkim_works) VALUES (?,?)
                ON CONFLICT(domain) DO UPDATE SET dkim_works=excluded.dkim_works",
            paramsv![from_domain, timestamp],
        )
        .await?;
    Ok(())
}

async fn clear_dkim_works(context: &Context) -> Result<()> {
    context
        .sql
        .execute("DELETE FROM sending_domains", ())
        .await?;
    Ok(())
}

fn parse_authservid_candidates_config(config: &Option<String>) -> BTreeSet<&str> {
    config
        .as_deref()
        .map(|c| c.split_whitespace().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing)]
    use std::time::Duration;

    use tokio::fs;
    use tokio::io::AsyncReadExt;

    use super::*;
    use crate::aheader::EncryptPreference;
    use crate::e2ee;
    use crate::message;
    use crate::mimeparser;
    use crate::peerstate::Peerstate;
    use crate::securejoin::get_securejoin_qr;
    use crate::securejoin::join_securejoin;
    use crate::test_utils;
    use crate::test_utils::TestContext;
    use crate::test_utils::TestContextManager;
    use crate::tools;

    #[test]
    fn test_remove_comments() {
        let header = "Authentication-Results: mx3.messagingengine.com;
    dkim=pass (1024-bit rsa key sha256) header.d=riseup.net;"
            .to_string();
        assert_eq!(
            remove_comments(&header),
            "Authentication-Results: mx3.messagingengine.com;
    dkim=pass   header.d=riseup.net;"
        );

        let header = ") aaa (".to_string();
        assert_eq!(remove_comments(&header), ") aaa (");

        let header = "((something weird) no comment".to_string();
        assert_eq!(remove_comments(&header), "  no comment");

        let header = "🎉(🎉(🎉))🎉(".to_string();
        assert_eq!(remove_comments(&header), "🎉 )🎉(");

        // Comments are allowed to include whitespace
        let header = "(com\n\t\r\nment) no comment (comment)".to_string();
        assert_eq!(remove_comments(&header), "  no comment  ");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_parse_authentication_results() -> Result<()> {
        let t = TestContext::new().await;
        t.configure_addr("alice@gmx.net").await;
        let bytes = b"Authentication-Results:  gmx.net; dkim=pass header.i=@slack.com
Authentication-Results:  gmx.net; dkim=pass header.i=@amazonses.com";
        let mail = mailparse::parse_mail(bytes)?;
        let actual = parse_authres_headers(&mail.get_headers(), "slack.com");
        assert_eq!(
            actual,
            vec![
                ("gmx.net".to_string(), DkimResult::Passed),
                ("gmx.net".to_string(), DkimResult::Nothing)
            ]
        );

        let bytes = b"Authentication-Results:  gmx.net; notdkim=pass header.i=@slack.com
Authentication-Results:  gmx.net; notdkim=pass header.i=@amazonses.com";
        let mail = mailparse::parse_mail(bytes)?;
        let actual = parse_authres_headers(&mail.get_headers(), "slack.com");
        assert_eq!(
            actual,
            vec![
                ("gmx.net".to_string(), DkimResult::Nothing),
                ("gmx.net".to_string(), DkimResult::Nothing)
            ]
        );

        let bytes = b"Authentication-Results:  gmx.net; dkim=pass header.i=@amazonses.com";
        let mail = mailparse::parse_mail(bytes)?;
        let actual = parse_authres_headers(&mail.get_headers(), "slack.com");
        assert_eq!(actual, vec![("gmx.net".to_string(), DkimResult::Nothing)],);

        // Weird Authentication-Results from Outlook without an authserv-id
        let bytes = b"Authentication-Results: spf=pass (sender IP is 40.92.73.85)
    smtp.mailfrom=hotmail.com; dkim=pass (signature was verified)
    header.d=hotmail.com;dmarc=pass action=none
    header.from=hotmail.com;compauth=pass reason=100";
        let mail = mailparse::parse_mail(bytes)?;
        let actual = parse_authres_headers(&mail.get_headers(), "hotmail.com");
        // At this point, the most important thing to test is that there are no
        // authserv-ids with whitespace in them.
        assert_eq!(
            actual,
            vec![("invalidAuthservId".to_string(), DkimResult::Passed)]
        );

        let bytes = b"Authentication-Results:  gmx.net; dkim=none header.i=@slack.com
Authentication-Results:  gmx.net; dkim=pass header.i=@slack.com";
        let mail = mailparse::parse_mail(bytes)?;
        let actual = parse_authres_headers(&mail.get_headers(), "slack.com");
        assert_eq!(
            actual,
            vec![
                ("gmx.net".to_string(), DkimResult::Failed),
                ("gmx.net".to_string(), DkimResult::Passed)
            ]
        );

        // ';' in comments
        let bytes = b"Authentication-Results: mx1.riseup.net;
	dkim=pass (1024-bit key; unprotected) header.d=yandex.ru header.i=@yandex.ru header.a=rsa-sha256 header.s=mail header.b=avNJu6sw;
	dkim-atps=neutral";
        let mail = mailparse::parse_mail(bytes)?;
        let actual = parse_authres_headers(&mail.get_headers(), "yandex.ru");
        assert_eq!(
            actual,
            vec![("mx1.riseup.net".to_string(), DkimResult::Passed)]
        );

        let bytes = br#"Authentication-Results: box.hispanilandia.net;
	dkim=fail reason="signature verification failed" (2048-bit key; secure) header.d=disroot.org header.i=@disroot.org header.b="kqh3WUKq";
	dkim-atps=neutral
Authentication-Results: box.hispanilandia.net; dmarc=pass (p=quarantine dis=none) header.from=disroot.org
Authentication-Results: box.hispanilandia.net; spf=pass smtp.mailfrom=adbenitez@disroot.org"#;
        let mail = mailparse::parse_mail(bytes)?;
        let actual = parse_authres_headers(&mail.get_headers(), "disroot.org");
        assert_eq!(
            actual,
            vec![
                ("box.hispanilandia.net".to_string(), DkimResult::Failed),
                ("box.hispanilandia.net".to_string(), DkimResult::Nothing),
                ("box.hispanilandia.net".to_string(), DkimResult::Nothing),
            ]
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_update_authservid_candidates() -> Result<()> {
        let t = TestContext::new_alice().await;

        update_authservid_candidates_test(&t, &["mx3.messagingengine.com"]).await;
        let candidates = t.get_config(Config::AuthservIdCandidates).await?.unwrap();
        assert_eq!(candidates, "mx3.messagingengine.com");

        // "mx4.messagingengine.com" seems to be the new authserv-id, DC should accept it
        update_authservid_candidates_test(&t, &["mx4.messagingengine.com"]).await;
        let candidates = t.get_config(Config::AuthservIdCandidates).await?.unwrap();
        assert_eq!(candidates, "mx4.messagingengine.com");

        // A message without any Authentication-Results headers shouldn't remove all
        // candidates since it could be a mailer-daemon message or so
        update_authservid_candidates_test(&t, &[]).await;
        let candidates = t.get_config(Config::AuthservIdCandidates).await?.unwrap();
        assert_eq!(candidates, "mx4.messagingengine.com");

        update_authservid_candidates_test(&t, &["mx4.messagingengine.com", "someotherdomain.com"])
            .await;
        let candidates = t.get_config(Config::AuthservIdCandidates).await?.unwrap();
        assert_eq!(candidates, "mx4.messagingengine.com");

        Ok(())
    }

    /// Calls update_authservid_candidates(), meant for using in a test.
    ///
    /// update_authservid_candidates() only looks at the keys of its
    /// `authentication_results` parameter. So, this function takes `incoming_ids`
    /// and adds some AuthenticationResults to get the HashMap we need.
    async fn update_authservid_candidates_test(context: &Context, incoming_ids: &[&str]) {
        let v = incoming_ids
            .iter()
            .map(|id| (id.to_string(), DkimResult::Passed))
            .collect();
        update_authservid_candidates(context, &v).await.unwrap()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_realworld_authentication_results() -> Result<()> {
        let mut test_failed = false;

        let dir = tools::read_dir("test-data/message/dkimchecks-2022-09-28/".as_ref())
            .await
            .unwrap();
        let mut bytes = Vec::new();
        for entry in dir {
            if !entry.file_type().await.unwrap().is_dir() {
                continue;
            }
            let self_addr = entry.file_name().into_string().unwrap();
            let self_domain = EmailAddress::new(&self_addr).unwrap().domain;
            let authres_parsing_works = [
                "ik.me",
                "web.de",
                "posteo.de",
                "gmail.com",
                "hotmail.com",
                "mail.ru",
                "aol.com",
                "yahoo.com",
                "icloud.com",
                "fastmail.com",
                "mail.de",
                "outlook.com",
                "gmx.de",
                "testrun.org",
            ]
            .contains(&self_domain.as_str());

            let t = TestContext::new().await;
            t.configure_addr(&self_addr).await;
            if !authres_parsing_works {
                println!("========= Receiving as {} =========", &self_addr);
            }

            // Simulate receiving all emails once, so that we have the correct authserv-ids
            let mut dir = tools::read_dir(&entry.path()).await.unwrap();

            // The ordering in which the emails are received can matter;
            // the test _should_ pass for every ordering.
            dir.sort_by_key(|d| d.file_name());
            //rand::seq::SliceRandom::shuffle(&mut dir[..], &mut rand::thread_rng());

            for entry in &dir {
                let mut file = fs::File::open(entry.path()).await?;
                bytes.clear();
                file.read_to_end(&mut bytes).await.unwrap();

                let mail = mailparse::parse_mail(&bytes)?;
                let from = &mimeparser::get_from(&mail.headers).unwrap().addr;

                let res = handle_authres(&t, &mail, from, time()).await?;
                assert!(res.allow_keychange);
            }

            for entry in &dir {
                let mut file = fs::File::open(entry.path()).await?;
                bytes.clear();
                file.read_to_end(&mut bytes).await.unwrap();

                let mail = mailparse::parse_mail(&bytes)?;
                let from = &mimeparser::get_from(&mail.headers).unwrap().addr;

                let res = handle_authres(&t, &mail, from, time()).await?;
                if !res.allow_keychange {
                    println!(
                        "!!!!!! FAILURE Receiving {:?}, keychange is not allowed !!!!!!",
                        entry.path()
                    );
                    test_failed = true;
                }

                let from_domain = EmailAddress::new(from).unwrap().domain;
                assert_eq!(
                    res.dkim_should_work,
                    dkim_should_work(dkim_works_timestamp(&t, &from_domain).await?)?
                );
                assert_eq!(res.dkim_passed, res.dkim_should_work);

                // delta.blinzeln.de and gmx.de have invalid DKIM, so the DKIM check should fail
                let expected_result = (from_domain != "delta.blinzeln.de") && (from_domain != "gmx.de")
                    // These are (fictional) forged emails where the attacker added a fake
                    // Authentication-Results before sending the email
                    && from != "forged-authres-added@example.com"
                    // Other forged emails
                    && !from.starts_with("forged");

                if res.dkim_passed != expected_result {
                    if authres_parsing_works {
                        println!(
                            "!!!!!! FAILURE Receiving {:?}, order {:#?} wrong result: !!!!!!",
                            entry.path(),
                            dir.iter().map(|e| e.file_name()).collect::<Vec<_>>()
                        );
                        test_failed = true;
                    }
                    println!("From {}: {}", from_domain, res.dkim_passed);
                }
            }
        }

        assert!(!test_failed);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_handle_authres() {
        let t = TestContext::new().await;

        // Even if the format is wrong and parsing fails, handle_authres() shouldn't
        // return an Err because this would prevent the message from being added
        // to the database and downloaded again and again
        let bytes = b"From: invalid@from.com
Authentication-Results: dkim=";
        let mail = mailparse::parse_mail(bytes).unwrap();
        handle_authres(&t, &mail, "invalid@rom.com", time())
            .await
            .unwrap();
    }

    #[ignore = "Disallowing keychanges is disabled for now"]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_handle_authres_fails() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;

        // Bob sends Alice a message, so she gets his key
        tcm.send_recv_accept(&bob, &alice, "Hi").await;

        // We don't need bob anymore, let's make sure it's not accidentally used
        drop(bob);

        // Assume Alice receives an email from bob@example.net with
        // correct DKIM -> `set_dkim_works()` was called
        set_dkim_works_timestamp(&alice, "example.net", time()).await?;
        // And Alice knows her server's authserv-id
        alice
            .set_config(Config::AuthservIdCandidates, Some("example.org"))
            .await?;

        tcm.section("An attacker, bob2, sends a from-forged email to Alice!");

        // Sleep to make sure key reset is ignored because of DKIM failure
        // and not because reordering is suspected.
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        let bob2 = tcm.unconfigured().await;
        bob2.configure_addr("bob@example.net").await;
        e2ee::ensure_secret_key_exists(&bob2).await?;

        let chat = bob2.create_chat(&alice).await;
        let mut sent = bob2
            .send_text(chat.id, "Please send me lots of money")
            .await;

        sent.payload
            .insert_str(0, "Authentication-Results: example.org; dkim=fail\n");

        let received = alice.recv_msg(&sent).await;

        // Assert that the error tells the user about the problem
        assert!(received.error.unwrap().contains("DKIM failed"));

        let bob_state = Peerstate::from_addr(&alice, "bob@example.net")
            .await?
            .unwrap();

        // Encryption preference is still mutual.
        assert_eq!(bob_state.prefer_encrypt, EncryptPreference::Mutual);

        // Also check that the keypair was not changed
        assert_eq!(
            bob_state.public_key.unwrap(),
            test_utils::bob_keypair().public
        );

        // Since Alice didn't change the key, Bob can't read her message
        let received = tcm
            .try_send_recv(&alice, &bob2, "My credit card number is 1234")
            .await;
        assert!(!received.text.as_ref().unwrap().contains("1234"));
        assert!(received.error.is_some());

        tcm.section("Turns out bob2 wasn't an attacker at all, Bob just has a new phone and DKIM just stopped working.");
        tcm.section("To fix the key problems, Bob scans Alice's QR code.");

        let qr = get_securejoin_qr(&alice.ctx, None).await.unwrap();
        join_securejoin(&bob2.ctx, &qr).await.unwrap();

        loop {
            if let Some(mut sent) = bob2.pop_sent_msg_opt(Duration::ZERO).await {
                sent.payload
                    .insert_str(0, "Authentication-Results: example.org; dkim=fail\n");
                alice.recv_msg(&sent).await;
            } else if let Some(sent) = alice.pop_sent_msg_opt(Duration::ZERO).await {
                bob2.recv_msg(&sent).await;
            } else {
                break;
            }
        }

        // Unfortunately, securejoin currently doesn't work with authres-checking,
        // so these checks would fail:

        // let contact_bob = alice.add_or_lookup_contact(&bob2).await;
        // assert_eq!(
        //     contact_bob.is_verified(&alice.ctx).await.unwrap(),
        //     VerifiedStatus::BidirectVerified
        // );

        // let contact_alice = bob2.add_or_lookup_contact(&alice).await;
        // assert_eq!(
        //     contact_alice.is_verified(&bob2.ctx).await.unwrap(),
        //     VerifiedStatus::BidirectVerified
        // );

        // // Bob can read Alice's messages again
        // let received = tcm
        //     .try_send_recv(&alice, &bob2, "Can you read this again?")
        //     .await;
        // assert_eq!(received.text.as_ref().unwrap(), "Can you read this again?");
        // assert!(received.error.is_none());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_autocrypt_in_mailinglist_ignored() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;

        let alice_bob_chat = alice.create_chat(&bob).await;
        let bob_alice_chat = bob.create_chat(&alice).await;
        let mut sent = alice.send_text(alice_bob_chat.id, "hellooo").await;
        sent.payload
            .insert_str(0, "List-Post: <mailto:deltachat-community.example.net>\n");
        bob.recv_msg(&sent).await;
        let peerstate = Peerstate::from_addr(&bob, "alice@example.org").await?;
        assert!(peerstate.is_none());

        // Do the same without the mailing list header, this time the peerstate should be accepted
        let sent = alice
            .send_text(alice_bob_chat.id, "hellooo without mailing list")
            .await;
        bob.recv_msg(&sent).await;
        let peerstate = Peerstate::from_addr(&bob, "alice@example.org").await?;
        assert!(peerstate.is_some());

        // This also means that Bob can now write encrypted to Alice:
        let mut sent = bob
            .send_text(bob_alice_chat.id, "hellooo in the mailinglist again")
            .await;
        assert!(sent.load_from_db().await.get_showpadlock());

        // But if Bob writes to a mailing list, Alice doesn't show a padlock
        // since she can't verify the signature without accepting Bob's key:
        sent.payload
            .insert_str(0, "List-Post: <mailto:deltachat-community.example.net>\n");
        let rcvd = alice.recv_msg(&sent).await;
        assert!(!rcvd.get_showpadlock());
        assert_eq!(&rcvd.text.unwrap(), "hellooo in the mailinglist again");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_authres_in_mailinglist_ignored() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;

        // Assume Bob received an email from something@example.net with
        // correct DKIM -> `set_dkim_works()` was called
        set_dkim_works_timestamp(&bob, "example.org", time()).await?;
        // And Bob knows his server's authserv-id
        bob.set_config(Config::AuthservIdCandidates, Some("example.net"))
            .await?;

        let alice_bob_chat = alice.create_chat(&bob).await;
        let mut sent = alice.send_text(alice_bob_chat.id, "hellooo").await;
        sent.payload
            .insert_str(0, "List-Post: <mailto:deltachat-community.example.net>\n");
        sent.payload
            .insert_str(0, "Authentication-Results: example.net; dkim=fail\n");
        let rcvd = bob.recv_msg(&sent).await;
        assert!(rcvd.error.is_none());

        // Do the same without the mailing list header, this time the failed
        // authres isn't ignored
        let mut sent = alice
            .send_text(alice_bob_chat.id, "hellooo without mailing list")
            .await;
        sent.payload
            .insert_str(0, "Authentication-Results: example.net; dkim=fail\n");
        let rcvd = bob.recv_msg(&sent).await;

        // Disallowing keychanges is disabled for now:
        // assert!(rcvd.error.unwrap().contains("DKIM failed"));
        // The message info should contain a warning:
        assert!(message::get_msg_info(&bob, rcvd.id)
            .await
            .unwrap()
            .contains("KEYCHANGES NOT ALLOWED"));

        Ok(())
    }
}
