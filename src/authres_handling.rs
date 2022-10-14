//! Parsing and handling of the Authentication-Results header.
//! See the comment on [`handle_authres`] for more.

use std::borrow::Cow;
use std::collections::HashSet;

use anyhow::Result;
use mailparse::MailHeaderMap;
use mailparse::ParsedMail;
use once_cell::sync::Lazy;

use crate::config::Config;
use crate::context::Context;
use crate::headerdef::HeaderDef;
use crate::tools::EmailAddress;

/// `authres` is short for the Authentication-Results header, which contains info
/// about whether DKIM and SPF passed.
///
/// To mitigate from forgery, we remember for each sending domain whether it is known
/// to have valid DKIM. If an email from such a domain comes with invalid DKIM,
/// we don't allow changing the autocrypt key.
pub(crate) async fn handle_authres(
    context: &Context,
    mail: &ParsedMail<'_>,
    from: &str,
) -> Result<bool> {
    let from_domain = match EmailAddress::new(from) {
        Ok(email) => email.domain,
        Err(e) => {
            warn!(context, "invalid email {:#}", e);
            // This email is invalid, but don't return an error, we still want to
            // add a stub to the database so that it's not downloaded again
            return Ok(false);
        }
    };

    let authentication_results = parse_authres_headers(&mail.get_headers(), &from_domain);
    update_authservid_candidates(context, &authentication_results).await?;
    let allow_keychange =
        should_allow_keychange(context, authentication_results, &from_domain).await?;
    Ok(allow_keychange)
}

// #[derive(Debug, PartialEq, Eq)]
// struct AuthenticationResults {
//     dkim_passed: bool,
// }

type AuthservId = String;

#[derive(Debug, PartialEq)]
enum DkimResult {
    /// Some(true): The header explicitly said that DKIM passed
    Passed,
    /// Some(false): The header explicitly said that DKIM failed
    Failed,
    /// None: The header didn't say anything about DKIM;
    /// this might mean that it failed or that it wasn't checked.
    Nothing,
}

type AuthenticationResults = Vec<(AuthservId, DkimResult)>;

fn parse_authres_headers(
    headers: &mailparse::headers::Headers<'_>,
    from_domain: &str,
) -> AuthenticationResults {
    let mut res = Vec::new();
    for header_value in headers.get_all_values(HeaderDef::AuthenticationResults.into()) {
        let header_value = remove_comments(&header_value);

        if let Some(mut authserv_id) = header_value.split(';').next() {
            if authserv_id.contains(char::is_whitespace) || authserv_id.is_empty() {
                // Outlook violates the RFC by not adding an authserv-id at all, which we notice
                // because there is whitespace in the first identifier before the ';'.
                // Authentication-Results-parsing still works securely because they remove incoming
                // Authentication-Results headers.
                // Just use an arbitrary authserv-id, it will work for Outlook, and in general,
                // with providers not implementing the RFC correctly, someone can trick us
                // into thinking that an incoming email is DKIM-correct, anyway.
                // The most important thing here is that we have some valid `authserv_id`.
                // TODO is this comment understandable?
                authserv_id = "invalidAuthservId";
            }
            let dkim_passed = parse_one_authres_header(&header_value, from_domain);
            res.push((authserv_id.to_string(), dkim_passed));
        }
    }

    res
}

fn remove_comments(header: &str) -> Cow<'_, str> {
    // Written in Pomsky, the regex is: "(" Codepoint* lazy ")"
    // See https://playground.pomsky-lang.org/?text=%22(%22%20Codepoint*%20lazy%20%22)%22
    static RE: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"\([\s\S]*?\)").unwrap());

    RE.replace_all(header, " ")
}

/// Parses the Authentication-Results headers belonging to a specific authserv-id
/// and returns whether they say that DKIM passed.
/// TODO document better
fn parse_one_authres_header(header_value: &str, from_domain: &str) -> DkimResult {
    if let Some((_start, dkim_to_end)) = header_value.split_once("dkim=") {
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

    DkimResult::Nothing
}

// TODO this is only half of the algorithm we thought of; we also wanted to save how sure we are
// about the authserv id. Like, a same-domain email is more trustworthy.
async fn update_authservid_candidates(
    context: &Context,
    authentication_results: &AuthenticationResults,
) -> Result<()> {
    let mut new_ids: HashSet<&str> = authentication_results
        .iter()
        .map(|(authserv_id, _dkim_passed)| authserv_id.as_str())
        .collect();
    if new_ids.is_empty() {
        // The incoming message doesn't contain any authentication results, maybe it's a
        // self-sent or a mailer-daemon message
        return Ok(());
    }

    let old_config = context.get_config(Config::AuthservidCandidates).await?;
    let old_ids = parse_authservid_candidates_config(&old_config);
    let intersection: HashSet<&str> = old_ids.intersection(&new_ids).copied().collect();
    if !intersection.is_empty() {
        new_ids = intersection;
    }
    // If there were no AuthservIdCandidates previously, just start with
    // the ones from the incoming email

    if old_ids != new_ids {
        let new_config = new_ids.into_iter().collect::<Vec<_>>().join(" ");
        context
            .set_config(Config::AuthservidCandidates, Some(&new_config))
            .await?;
        // Updating the authservid candidates may mean that we now consider
        // emails as "failed" which "passed" previously, so we need to
        // reset our expectation which DKIMs work.
        clear_dkim_works(context).await?
    }
    Ok(())
}

/// We disallow changes to the autocrypt key if DKIM failed, but worked in the past,
/// because we then assume that the From header is forged.
async fn should_allow_keychange(
    context: &Context,
    mut authentication_results: AuthenticationResults,
    from_domain: &str,
) -> Result<bool> {
    let mut dkim_passed = false; // TODO what do we want to do if there are multiple or no authservid candidates?

    let ids_config = context.get_config(Config::AuthservidCandidates).await?;
    let ids = parse_authservid_candidates_config(&ids_config);

    // Remove all foreign authentication results
    authentication_results
        .retain_mut(|(authserv_id, _dkim_passed)| ids.contains(authserv_id.as_str()));

    if authentication_results.is_empty() {
        // If the authentication results are empty, then our provider doesn't add them
        // and an attacker could just add their own Authentication-Results, making us
        // think that DKIM passed. So, in this case, we can as well assume that DKIM passed.
        dkim_passed = true;
    } else {
        for (_authserv_id, current_dkim_passed) in authentication_results {
            match current_dkim_passed {
                DkimResult::Passed => {
                    dkim_passed = true;
                    break;
                }
                DkimResult::Failed => {
                    dkim_passed = false;
                    break;
                }
                DkimResult::Nothing => {}
            }
        }
    }

    let dkim_works = dkim_works(context, from_domain).await?;
    if !dkim_works && dkim_passed {
        set_dkim_works(context, from_domain).await?;
    }

    // //TODO dbg
    // if dkim_passed {
    //     let works_now = dkim_known_to_work(context, from_domain).await.unwrap();
    //     println!("should_work {should_work} dkim_passed {dkim_passed} works_now {works_now}");
    //     assert!(works_now);
    // }

    Ok(dkim_passed || !dkim_works)
}

async fn dkim_works(context: &Context, from_domain: &str) -> Result<bool> {
    Ok(context
        .sql
        .query_get_value(
            "SELECT dkim_works FROM sending_domains WHERE domain=?;",
            paramsv![from_domain],
        )
        .await?
        .unwrap_or(false))
}

async fn set_dkim_works(context: &Context, from_domain: &str) -> Result<()> {
    context
        .sql
        .execute(
            "INSERT INTO sending_domains (domain, dkim_works) VALUES (?1,1)
                ON CONFLICT(domain) DO UPDATE SET dkim_works=1 WHERE domain=?1;",
            paramsv![from_domain],
        )
        .await?;
    Ok(())
}

async fn clear_dkim_works(context: &Context) -> Result<()> {
    context
        .sql
        .execute("DELETE FROM sending_domains", paramsv![])
        .await?;
    Ok(())
}

fn parse_authservid_candidates_config(config: &Option<String>) -> HashSet<&str> {
    config
        .as_deref()
        .map(|c| c.split_whitespace().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing)]
    use tokio::fs;
    use tokio::io::AsyncReadExt;

    use super::*;
    use crate::mimeparser;
    use crate::test_utils::TestContext;

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

        let bytes = b"Authentication-Results:  gmx.net; dkim=pass header.i=@amazonses.com";
        let mail = mailparse::parse_mail(bytes)?;
        let actual = parse_authres_headers(&mail.get_headers(), "slack.com");
        // TODO Actually, we could be able to tell that DkimResult::Failed here, since a check was done
        // but the From domain didn't match.
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

        // Usually, MUAs put their Authentication-Results to the top, so if in doubt,
        // headers from the top should be preferred
        // TODO this has to be checked somewhere else now

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

        //         let bytes = b"Authentication-Results: mx1.messagingengine.com;
        //     x-csa=none;
        //     x-me-sender=none;
        //     x-ptr=pass smtp.helo=nx184.node01.secure-mailgate.com
        //       policy.ptr=nx184.node01.secure-mailgate.com
        // Authentication-Results: mx1.messagingengine.com;
        //     bimi=skipped (DMARC did not pass)
        // Authentication-Results: mx1.messagingengine.com;
        //     arc=none (no signatures found)
        // Authentication-Results: mx1.messagingengine.com;
        //     dkim=none (no signatures found);
        //     dmarc=none policy.published-domain-policy=none
        //       policy.applied-disposition=none policy.evaluated-disposition=none
        //       (p=none,d=none,d.eval=none) policy.policy-from=p
        //       header.from=delta.blinzeln.de;
        //     iprev=pass smtp.remote-ip=89.22.108.184
        //       (nx184.node01.secure-mailgate.com);
        //     spf=none smtp.mailfrom=nami.lefherz@delta.blinzeln.de
        //       smtp.helo=nx184.node01.secure-mailgate.com";
        //         let mail = mailparse::parse_mail(bytes)?;
        //         let actual = parse_authres_headers(&mail.get_headers(), "delta.blinzeln.de");
        //         assert_eq!(actual, vec![("mx1.messagingengine.com".to_string(), false)]);

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
        let candidates = t.get_config(Config::AuthservidCandidates).await?.unwrap();
        assert_eq!(candidates, "mx3.messagingengine.com");

        update_authservid_candidates_test(&t, &["mx4.messagingengine.com"]).await;
        let candidates = t.get_config(Config::AuthservidCandidates).await?.unwrap();
        assert_eq!(candidates, "");

        // "mx4.messagingengine.com" seems to be the new authserv-id, DC should accept it
        update_authservid_candidates_test(&t, &["mx4.messagingengine.com"]).await;
        let candidates = t.get_config(Config::AuthservidCandidates).await?.unwrap();
        assert_eq!(candidates, "mx4.messagingengine.com");

        // A message without any Authentication-Results headers shouldn't remove all
        // candidates since it could be a mailer-daemon message or so
        update_authservid_candidates_test(&t, &[]).await;
        let candidates = t.get_config(Config::AuthservidCandidates).await?.unwrap();
        assert_eq!(candidates, "mx4.messagingengine.com");

        update_authservid_candidates_test(&t, &["mx4.messagingengine.com", "someotherdomain.com"])
            .await;
        let candidates = t.get_config(Config::AuthservidCandidates).await?.unwrap();
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

        let mut dir = fs::read_dir("test-data/message/dkimchecks-2022-09-28/")
            .await
            .unwrap();
        let mut bytes = Vec::new();
        while let Some(entry) = dir.next_entry().await.unwrap() {
            let self_addr = entry.file_name().into_string().unwrap();
            let self_domain = EmailAddress::new(&self_addr).unwrap().domain;
            let authres_parsing_works = [
                "ik.me",
                "web.de",
                "posteo.de",
                "gmail.com",
                "hotmail.com",
                "mail.ru",
                "delta.blinzeln.de",
                "e.email",
                "mailo.com",
            ]
            .contains(&self_domain.as_str());

            let t = TestContext::new().await;
            t.configure_addr(&self_addr).await;
            if !authres_parsing_works {
                println!("========= Receiving as {self_addr} =========");
            }

            // TODO code duplication with the next while loop
            // Simulate receiving all emails once, so that we have the correct authserv-ids
            let mut dir = fs::read_dir(entry.path()).await.unwrap();
            while let Some(entry) = dir.next_entry().await.unwrap() {
                let mut file = fs::File::open(entry.path()).await?;
                bytes.clear();
                file.read_to_end(&mut bytes).await.unwrap();
                if bytes.is_empty() {
                    continue;
                }
                //println!("{:?}", entry.path());

                let mail = mailparse::parse_mail(&bytes)?;
                let from = &mimeparser::get_from(&mail.headers)[0].addr;

                let allow_keychange = handle_authres(&t, &mail, from).await?;
                assert!(allow_keychange);
            }

            let mut dir = fs::read_dir(entry.path()).await.unwrap();
            while let Some(entry) = dir.next_entry().await.unwrap() {
                let mut file = fs::File::open(entry.path()).await?;
                bytes.clear();
                file.read_to_end(&mut bytes).await.unwrap();
                if bytes.is_empty() {
                    continue;
                }
                //println!("{:?}", entry.path());

                let mail = mailparse::parse_mail(&bytes)?;
                let from = &mimeparser::get_from(&mail.headers)[0].addr;

                let allow_keychange = handle_authres(&t, &mail, from).await?;
                if !allow_keychange {
                    println!(
                        "!!!!!! FAILURE Receiving {:?}, keychange is not allowed !!!!!!",
                        entry.path()
                    );
                    test_failed = true;
                }

                let from_domain = EmailAddress::new(from).unwrap().domain;
                let dkim_result = dkim_works(&t, &from_domain).await.unwrap();
                // println!("From {from_domain}: passed {dkim_passed}, known to work {dkim_known_to_work}");
                let expected_result = from_domain != "delta.blinzeln.de";
                if dkim_result != expected_result {
                    if authres_parsing_works {
                        println!(
                            "!!!!!! FAILURE Receiving {:?}, wrong result: !!!!!!",
                            entry.path()
                        );
                        test_failed = true;
                    }
                    println!("From {from_domain}: {dkim_result}");
                }
            }

            std::mem::forget(t) // TODO dbg
        }

        assert!(!test_failed);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_handle_authres() {
        let t = TestContext::new().await;

        // Even if the format is wrong and parsing fails, handle_authres() shouldn't
        // return an Err because this would prevent the message from being added
        // to the database and
        let bytes = b"Authentication-Results: dkim=";
        let mail = mailparse::parse_mail(bytes).unwrap();
        handle_authres(&t, &mail, "invalidfrom.com").await.unwrap();
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
