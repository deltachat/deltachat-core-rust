//! TODO file comment.

use std::collections::HashMap;
use std::collections::HashSet;

use anyhow::{Context as _, Result};
use mailparse::MailHeaderMap;

use crate::config::Config;
use crate::context::Context;
use crate::headerdef::HeaderDef;

use crate::tools;
use crate::tools::EmailAddress;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AuthenticationResults {
    dkim_passed: bool,
}

pub(crate) type AuthservId = String;

pub(crate) fn parse_authentication_results(
    headers: &mailparse::headers::Headers<'_>,
    from: &str,
) -> Result<HashMap<AuthservId, AuthenticationResults>> {
    // TODO old comment:
    // TODO this doesn't work for e.g. GMX which sells @gmx.de addresses, but uses gmx.net as its server
    // Config::ConfiguredProvider doesn't work for e.g. Gmail which uses mx.google.com.
    //
    // We could self-send a message during configure and use the Authentication-Results header from there -
    // this works for e.g. GMX, but not for Testrun and GMAIL.
    // -> Alternatively, we could send a message to nonexistent@example.com and wait for the NDN. This works
    //    for Gmail, but the Testrun NDN doesn't contain such a header, and GMX returns an error directly
    //    while sending.
    //
    // We could save this info in the provider db, but this only works for these providers.

    // let from = match from.first() {
    //     Some(f) => &f.addr,
    //     None => return Ok(HashMap::new()),
    // }; // TODO
    let sender_domain = EmailAddress::new(from)?.domain;

    let mut header_map: HashMap<AuthservId, Vec<String>> = HashMap::new();
    for header_value in headers.get_all_values(HeaderDef::AuthenticationResults.into()) {
        // TODO there could be a comment [CFWS] before the self domain. Do we care? Probably not.
        let mut authserv_id = header_value.split(';').next().context("Empty header")?; // TODO do we really want to return Err here if it's empty
        if authserv_id.contains(char::is_whitespace) {
            // Outlook violates the RFC by not adding an authserv-id at all, which we notice
            // because there is whitespace in the first identifier before the ';'.
            // Authentication-Results-parsing still works securely because they remove incoming
            // Authentication-Results headers.
            // Just use an arbitrary authserv-id, it will work for Outlook, and in general,
            // with providers not implementing the RFC correctly, someone can trick us
            // into thinking that an incoming email is DKIM-correct, anyway.
            // TODO is this comment understandable?
            authserv_id = "invalidAuthservId"
        }
        header_map
            .entry(authserv_id.to_string())
            .or_default()
            .push(header_value);
    }

    let mut authresults_map = HashMap::new();
    for (authserv_id, headers) in header_map {
        let dkim_passed = authresults_dkim_passed(&headers, &sender_domain)?;
        authresults_map.insert(authserv_id, AuthenticationResults { dkim_passed });
    }

    Ok(authresults_map)
}

/// Parses the Authentication-Results headers belonging to a specific authserv-id
/// and returns whether they say that DKIM passed.
/// TODO document better
/// TODO if there are multiple headers and one says `pass`, one says `fail`, `none`
/// or whatever, then we still interpret that as `pass` - is this a problem?
fn authresults_dkim_passed(headers: &[String], sender_domain: &str) -> Result<bool> {
    for header_value in headers {
        if let Some((_start, dkim_to_end)) = header_value.split_once("dkim=") {
            let dkim_part = dkim_to_end
                .split(';')
                .next()
                .context("what the hell TODO")?;
            let dkim_parts: Vec<_> = dkim_part.split_whitespace().collect();
            if let Some(&"pass") = dkim_parts.first() {
                let header_d: &str = &format!("header.d={}", &sender_domain);
                let header_i: &str = &format!("header.i=@{}", &sender_domain);

                if dkim_parts.contains(&header_d) || dkim_parts.contains(&header_i) {
                    // We have found a `dkim=pass` header!
                    return Ok(true);
                }
            } else {
                // dkim=fail, dkim=none or whatever
                return Ok(false);
            }
        }
    }

    Ok(false)
}

fn parse_authservid_candidates_config(config: &Option<String>) -> HashSet<&str> {
    config
        .as_deref()
        .map(|c| c.split_whitespace().collect())
        .unwrap_or_default()
}

// TODO this is only half of the algorithm we thought of; we also wanted to save how sure we are
// about the authserv id. Like, a same-domain email is more trustworthy.
pub(crate) async fn update_authservid_candidates(
    context: &Context,
    authentication_results: &HashMap<AuthservId, AuthenticationResults>,
) -> Result<()> {
    let mut new_ids: HashSet<_> = authentication_results.keys().map(String::as_str).collect();
    if new_ids.is_empty() {
        // The incoming message doesn't contain any authentication results, maybe it's a
        // self-sent or a mailer-daemon message
        return Ok(());
    }

    let old_config = context.get_config(Config::AuthservIdCandidates).await?;
    let old_ids = parse_authservid_candidates_config(&old_config);
    if !old_ids.is_empty() {
        new_ids = old_ids.intersection(&new_ids).copied().collect();
    }
    // If there were no AuthservIdCandidates previously, just start with
    // the ones from the incoming email

    if old_ids != new_ids {
        let new_config = new_ids.into_iter().collect::<Vec<_>>().join(" ");
        context
            .set_config(Config::AuthservIdCandidates, Some(&new_config))
            .await?;
    }
    Ok(())
}

/// We disallow changes to the autocrypt key if DKIM failed, but worked in the past,
/// because we then assume that the From header is forged.
pub(crate) async fn should_allow_keychange(
    context: &Context,
    authentication_results: &HashMap<String, AuthenticationResults>,
    from: &str,
) -> Result<bool> {
    // TODO code duplication with update_authservid_candidates()
    let mut dkim_passed = true; // TODO what do we want to do if there are multiple or no authservid candidates?

    // If the authentication results are empty, then our provider doesn't add them
    // and an attacker could just add their own Authentication-Results, making us
    // think that DKIM passed. So, in this case, we can as well assume that DKIM passed.
    if !authentication_results.is_empty() {
        if let Some(ids_config) = context.get_config(Config::AuthservIdCandidates).await? {
            let ids = ids_config.split(' ').filter(|s| !s.is_empty());
            println!("{}", &ids_config);
            if let Some(authserv_id) = tools::single_value(ids) {
                dbg!(&authentication_results, &ids_config);
                // TODO unwrap
                dkim_passed = authentication_results.get(authserv_id).unwrap().dkim_passed;
            }
        }
    }

    let sending_domain = from.parse::<EmailAddress>().unwrap().domain; // TODO unwrap
    let dkim_known_to_work = context
        .sql
        .query_get_value(
            "SELECT correct_dkim FROM sending_domains WHERE domain=?;",
            paramsv![sending_domain],
        )
        .await?
        .unwrap_or(false);

    if !dkim_known_to_work && dkim_passed {
        context
            .sql
            .execute(
                "UPDATE sending_domains SET correct_dkim=1 WHERE domain=?;",
                paramsv![sending_domain],
            )
            .await?;
    }

    Ok(dkim_passed || !dkim_known_to_work)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing)]
    use tokio::fs;
    use tokio::io::AsyncReadExt;

    use super::*;
    use crate::mimeparser;
    use crate::test_utils::TestContext;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_parse_authentication_results() -> Result<()> {
        let t = TestContext::new().await;
        t.configure_addr("alice@gmx.net").await;
        let bytes = b"Authentication-Results:  gmx.net; dkim=pass header.i=@slack.com
Authentication-Results:  gmx.net; dkim=pass header.i=@amazonses.com";
        let mail = mailparse::parse_mail(bytes)?;
        let actual = parse_authentication_results(&mail.get_headers(), "info@slack.com").unwrap();
        assert_eq!(
            actual,
            [(
                "gmx.net".to_string(),
                AuthenticationResults { dkim_passed: true }
            )]
            .into()
        );

        let bytes = b"Authentication-Results:  gmx.net; dkim=pass header.i=@amazonses.com";
        let mail = mailparse::parse_mail(bytes)?;
        let actual = parse_authentication_results(&mail.get_headers(), "info@slack.com").unwrap();
        assert_eq!(
            actual,
            [(
                "gmx.net".to_string(),
                AuthenticationResults { dkim_passed: false }
            )]
            .into()
        );

        // Weird Authentication-Results from Outlook without an authserv-id
        let bytes = b"Authentication-Results: spf=pass (sender IP is 40.92.73.85)
        smtp.mailfrom=hotmail.com; dkim=pass (signature was verified)
        header.d=hotmail.com;dmarc=pass action=none
        header.from=hotmail.com;compauth=pass reason=100";
        let mail = mailparse::parse_mail(bytes)?;
        let actual =
            parse_authentication_results(&mail.get_headers(), "alice@hotmail.com").unwrap();
        // At this point, the most important thing to test is that there are no
        // authserv-ids with whitespace in them.
        assert_eq!(
            actual,
            [(
                "invalidAuthservId".to_string(),
                AuthenticationResults { dkim_passed: true }
            )]
            .into()
        );

        // Usually, MUAs put their Authentication-Results to the top, so if in doubt,
        // headers from the top should be preferred
        let bytes = b"Authentication-Results:  gmx.net; dkim=none header.i=@slack.com
Authentication-Results:  gmx.net; dkim=pass header.i=@slack.com";
        let mail = mailparse::parse_mail(bytes)?;
        let actual = parse_authentication_results(&mail.get_headers(), "info@slack.com").unwrap();
        assert_eq!(
            actual,
            [(
                "gmx.net".to_string(),
                AuthenticationResults { dkim_passed: false }
            )]
            .into()
        );

        // TODO test that foreign Auth-Res headers are ignored

        //         check_parse_authentication_results_combination(
        //             "alice@testrun.org",
        //             // TODO actually the address is alice@gmx.de, but then it doesn't work because `header.d=gmx.net`:
        //             b"From: alice@gmx.net
        // Authentication-Results: testrun.org;
        // 	dkim=pass header.d=gmx.net header.s=badeba3b8450 header.b=Gug6p4zD;
        // 	dmarc=pass (policy=none) header.from=gmx.de;
        // 	spf=pass (testrun.org: domain of alice@gmx.de designates 212.227.17.21 as permitted sender) smtp.mailfrom=alice@gmx.de",
        //             AuthenticationResults::Passed,
        //         )
        //         .await;

        //         check_parse_authentication_results_combination(
        //             "alice@testrun.org",
        //             br#"From: hocuri@testrun.org
        // Authentication-Results: box.hispanilandia.net; dmarc=none (p=none dis=none) header.from=nauta.cu
        // Authentication-Results: box.hispanilandia.net; spf=pass smtp.mailfrom=adbenitez@nauta.cu
        // Authentication-Results: testrun.org;
        // 	dkim=fail ("body hash did not verify") header.d=nauta.cu header.s=nauta header.b=YrWhU6qk;
        // 	dmarc=none;
        // 	spf=pass (testrun.org: domain of "test1-bounces+hocuri=testrun.org@hispanilandia.net" designates 51.15.127.36 as permitted sender) smtp.mailfrom="test1-bounces+hocuri=testrun.org@hispanilandia.net"
        // "#,
        //             AuthenticationResults::Failed,
        //         )
        //         .await;

        //         check_parse_authentication_results_combination(

        //             // TODO fails because mx.google.com, not google.com
        //             "alice@gmail.com",
        //             br#"From: not-so-fake@hispanilandia.net
        // Authentication-Results: mx.google.com;
        //        dkim=pass header.i=@hispanilandia.net header.s=mail header.b="Ih5Sz2/P";
        //        spf=pass (google.com: domain of not-so-fake@hispanilandia.net designates 51.15.127.36 as permitted sender) smtp.mailfrom=not-so-fake@hispanilandia.net;
        //        dmarc=pass (p=QUARANTINE sp=QUARANTINE dis=NONE) header.from=hispanilandia.net"#,
        //             AuthenticationResults::Passed,
        //         )
        //         .await;

        //         check_parse_authentication_results_combination(
        //             "alice@nauta.cu",
        //             br#"From: adb <adbenitez@disroot.org>
        // Authentication-Results: box.hispanilandia.net;
        // 	dkim=fail reason="signature verification failed" (2048-bit key; secure) header.d=disroot.org header.i=@disroot.org header.b="kqh3WUKq";
        // 	dkim-atps=neutral
        // Authentication-Results: box.hispanilandia.net; dmarc=pass (p=quarantine dis=none) header.from=disroot.org
        // Authentication-Results: box.hispanilandia.net; spf=pass smtp.mailfrom=adbenitez@disroot.org"#,
        //             AuthenticationResults::Passed,
        //         )
        //         .await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_update_authservid_candidates() -> Result<()> {
        let t = TestContext::new_alice().await;

        update_authservid_candidates_test(&t, &["mx3.messagingengine.com"]).await;
        let candidates = t.get_config(Config::AuthservIdCandidates).await?.unwrap();
        assert_eq!(candidates, "mx3.messagingengine.com");

        update_authservid_candidates_test(&t, &["mx4.messagingengine.com"]).await;
        let candidates = t.get_config(Config::AuthservIdCandidates).await?.unwrap();
        assert_eq!(candidates, "");

        // "mx4.messagingengine.com" seems to be the new authserv-id, DC should accept it
        update_authservid_candidates_test(&t, &["mx4.messagingengine.com"]).await;
        let candidates = t.get_config(Config::AuthservIdCandidates).await?.unwrap();
        assert_eq!(candidates, "mx4.messagingengine.com");

        Ok(())
    }

    /// TODO document
    async fn update_authservid_candidates_test(context: &Context, incoming_ids: &[&str]) {
        let map = incoming_ids
            .iter()
            // update_authservid_candidates() only looks at the keys of the HashMap argument,
            // so just provide some arbitrary values
            .map(|id| (id.to_string(), AuthenticationResults { dkim_passed: true }))
            .collect();
        update_authservid_candidates(context, &map).await.unwrap()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_realworld_authentication_results() -> Result<()> {
        let mut dir = fs::read_dir("test-data/message/dkimchecks-2022-09-28/")
            .await
            .unwrap();
        let mut bytes = Vec::new();
        while let Some(entry) = dir.next_entry().await.unwrap() {
            let self_addr = entry.file_name().into_string().unwrap();
            let mut dir = fs::read_dir(entry.path()).await.unwrap();

            let t = TestContext::new().await;
            t.configure_addr(&self_addr).await;

            while let Some(entry) = dir.next_entry().await.unwrap() {
                let mut file = fs::File::open(entry.path()).await?;
                bytes.clear();
                file.read_to_end(&mut bytes).await.unwrap();
                if bytes.is_empty() {
                    continue;
                }
                println!("{:?}", entry.path());

                let mail = mailparse::parse_mail(&bytes)?;
                let from = &mimeparser::get_from(&mail.headers)[0].addr;

                // TODO code duplication with create_decryption_info()
                let authentication_results =
                    parse_authentication_results(&mail.get_headers(), from)?;
                update_authservid_candidates(&t, &authentication_results).await?;
                let allow_keychange =
                    should_allow_keychange(&t, &authentication_results, from).await?;

                assert!(allow_keychange);

                // check_parse_authentication_results_combination(
                //     &self_addr,
                //     &bytes,
                //     AuthenticationResults::Passed,
                // )
                // .await;
            }

            std::mem::forget(t) // TODO dbg
        }
        Ok(())
    }

    // async fn check_parse_authentication_results_combination(
    //     self_addr: &str,
    //     header_bytes: &[u8],
    //     expected_result: AuthenticationResults,
    // ) {
    //     let t = TestContext::new().await;
    //     t.set_primary_self_addr(self_addr).await.unwrap();
    //     let mail = mailparse::parse_mail(body)?;

    //     let actual = parse_authentication_results(&t, &mail.get_headers(), &from)?;
    //     //assert_eq!(message.authentication_results, expected_result);
    //     if message.authentication_results != expected_result {
    //         eprintln!(
    //             "EXPECTED {expected_result:?}, GOT {:?}, SELF {}, FROM {:?}",
    //             message.authentication_results,
    //             self_addr,
    //             message.from.first().map(|i| &i.addr),
    //         )
    //     } else {
    //         eprintln!(
    //             "CORRECT {:?}, SELF {}, FROM {:?}",
    //             message.authentication_results,
    //             self_addr,
    //             message.from.first().map(|i| &i.addr),
    //         )
    //     }
    // }
}
