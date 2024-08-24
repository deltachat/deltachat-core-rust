//! DNS resolution and cache.

use anyhow::{Context as _, Result};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::str::FromStr;
use tokio::net::lookup_host;
use tokio::time::timeout;

use super::load_connection_timestamp;
use crate::context::Context;
use crate::tools::time;
use once_cell::sync::Lazy;

/// Inserts entry into DNS cache
/// or updates existing one with a new timestamp.
async fn update_cache(context: &Context, host: &str, addr: &str, now: i64) -> Result<()> {
    context
        .sql
        .execute(
            "INSERT INTO dns_cache
             (hostname, address, timestamp)
             VALUES (?, ?, ?)
             ON CONFLICT (hostname, address)
             DO UPDATE SET timestamp=excluded.timestamp",
            (host, addr, now),
        )
        .await?;
    Ok(())
}

pub(crate) async fn prune_dns_cache(context: &Context) -> Result<()> {
    let now = time();
    context
        .sql
        .execute(
            "DELETE FROM dns_cache
             WHERE ? > timestamp + ?",
            (now, super::CACHE_TTL),
        )
        .await?;
    Ok(())
}

/// Looks up the hostname and updates DNS cache
/// on success.
async fn lookup_host_and_update_cache(
    context: &Context,
    hostname: &str,
    port: u16,
    now: i64,
) -> Result<Vec<SocketAddr>> {
    let res: Vec<SocketAddr> = timeout(super::TIMEOUT, lookup_host((hostname, port)))
        .await
        .context("DNS lookup timeout")?
        .context("DNS lookup failure")?
        .collect();

    for addr in &res {
        let ip_string = addr.ip().to_string();
        if ip_string == hostname {
            // IP address resolved into itself, not interesting to cache.
            continue;
        }

        info!(context, "Resolved {hostname}:{port} into {addr}.");

        // Update the cache.
        update_cache(context, hostname, &ip_string, now).await?;
    }

    Ok(res)
}

// Updates timestamp of the cached entry
// or inserts a new one if cached entry does not exist.
//
// This function should be called when a successful TLS
// connection is established with strict TLS checks.
//
// This increases priority of existing cached entries
// and copies fallback addresses from built-in cache
// into database cache on successful use.
//
// Unlike built-in cache,
// database cache is used even if DNS
// resolver returns a non-empty
// (but potentially incorrect and unusable) result.
pub(crate) async fn update_connect_timestamp(
    context: &Context,
    host: &str,
    address: &str,
) -> Result<()> {
    if host == address {
        return Ok(());
    }

    context
        .sql
        .execute(
            "INSERT INTO dns_cache (hostname, address, timestamp)
                 VALUES (?, ?, ?)
                 ON CONFLICT (hostname, address)
                 DO UPDATE SET timestamp=excluded.timestamp",
            (host, address, time()),
        )
        .await?;
    Ok(())
}

static DNS_PRELOAD: Lazy<HashMap<&'static str, Vec<IpAddr>>> = Lazy::new(|| {
    HashMap::from([
        (
            "mail.sangham.net",
            vec![
                IpAddr::V4(Ipv4Addr::new(159, 69, 186, 85)),
                IpAddr::V6(Ipv6Addr::new(0x2a01, 0x4f8, 0xc17, 0x798c, 0, 0, 0, 1)),
            ],
        ),
        (
            "nine.testrun.org",
            vec![
                IpAddr::V4(Ipv4Addr::new(116, 202, 233, 236)),
                IpAddr::V4(Ipv4Addr::new(128, 140, 126, 197)),
                IpAddr::V4(Ipv4Addr::new(49, 12, 116, 128)),
                IpAddr::V6(Ipv6Addr::new(0x2a01, 0x4f8, 0x241, 0x4ce8, 0, 0, 0, 2)),
            ],
        ),
        (
            "disroot.org",
            vec![IpAddr::V4(Ipv4Addr::new(178, 21, 23, 139))],
        ),
        (
            "imap.gmail.com",
            vec![
                IpAddr::V4(Ipv4Addr::new(142, 250, 110, 108)),
                IpAddr::V4(Ipv4Addr::new(142, 250, 110, 109)),
                IpAddr::V4(Ipv4Addr::new(66, 102, 1, 108)),
                IpAddr::V4(Ipv4Addr::new(66, 102, 1, 109)),
                IpAddr::V6(Ipv6Addr::new(0x2a00, 0x1450, 0x400c, 0xc1f, 0, 0, 0, 0x6c)),
                IpAddr::V6(Ipv6Addr::new(0x2a00, 0x1450, 0x400c, 0xc1f, 0, 0, 0, 0x6d)),
            ],
        ),
        (
            "smtp.gmail.com",
            vec![
                IpAddr::V4(Ipv4Addr::new(142, 250, 110, 109)),
                IpAddr::V6(Ipv6Addr::new(0x2a00, 0x1450, 0x4013, 0xc04, 0, 0, 0, 0x6c)),
            ],
        ),
        (
            "mail.autistici.org",
            vec![
                IpAddr::V4(Ipv4Addr::new(198, 167, 222, 108)),
                IpAddr::V4(Ipv4Addr::new(82, 94, 249, 234)),
                IpAddr::V4(Ipv4Addr::new(93, 190, 126, 19)),
            ],
        ),
        (
            "smtp.autistici.org",
            vec![
                IpAddr::V4(Ipv4Addr::new(198, 167, 222, 108)),
                IpAddr::V4(Ipv4Addr::new(82, 94, 249, 234)),
                IpAddr::V4(Ipv4Addr::new(93, 190, 126, 19)),
            ],
        ),
        (
            "daleth.cafe",
            vec![IpAddr::V4(Ipv4Addr::new(37, 27, 6, 204))],
        ),
        (
            "imap.163.com",
            vec![IpAddr::V4(Ipv4Addr::new(111, 124, 203, 45))],
        ),
        (
            "smtp.163.com",
            vec![IpAddr::V4(Ipv4Addr::new(103, 129, 252, 45))],
        ),
        (
            "imap.aol.com",
            vec![
                IpAddr::V4(Ipv4Addr::new(212, 82, 101, 33)),
                IpAddr::V4(Ipv4Addr::new(87, 248, 98, 69)),
            ],
        ),
        (
            "smtp.aol.com",
            vec![IpAddr::V4(Ipv4Addr::new(87, 248, 97, 31))],
        ),
        (
            "mail.arcor.de",
            vec![IpAddr::V4(Ipv4Addr::new(2, 207, 150, 234))],
        ),
        (
            "imap.arcor.de",
            vec![IpAddr::V4(Ipv4Addr::new(2, 207, 150, 230))],
        ),
        (
            "imap.fastmail.com",
            vec![
                IpAddr::V4(Ipv4Addr::new(103, 168, 172, 43)),
                IpAddr::V4(Ipv4Addr::new(103, 168, 172, 58)),
            ],
        ),
        (
            "smtp.fastmail.com",
            vec![
                IpAddr::V4(Ipv4Addr::new(103, 168, 172, 45)),
                IpAddr::V4(Ipv4Addr::new(103, 168, 172, 60)),
            ],
        ),
        (
            "imap.gmx.net",
            vec![
                IpAddr::V4(Ipv4Addr::new(212, 227, 17, 170)),
                IpAddr::V4(Ipv4Addr::new(212, 227, 17, 186)),
            ],
        ),
        (
            "imap.mail.de",
            vec![IpAddr::V4(Ipv4Addr::new(62, 201, 172, 16))],
        ),
        (
            "smtp.mailbox.org",
            vec![IpAddr::V4(Ipv4Addr::new(185, 97, 174, 196))],
        ),
        (
            "imap.mailbox.org",
            vec![IpAddr::V4(Ipv4Addr::new(185, 97, 174, 199))],
        ),
        (
            "imap.naver.com",
            vec![IpAddr::V4(Ipv4Addr::new(125, 209, 238, 153))],
        ),
        (
            "imap.ouvaton.coop",
            vec![IpAddr::V4(Ipv4Addr::new(194, 36, 166, 20))],
        ),
        (
            "imap.purelymail.com",
            vec![IpAddr::V4(Ipv4Addr::new(18, 204, 123, 63))],
        ),
        (
            "imap.tiscali.it",
            vec![IpAddr::V4(Ipv4Addr::new(213, 205, 33, 10))],
        ),
        (
            "smtp.tiscali.it",
            vec![IpAddr::V4(Ipv4Addr::new(213, 205, 33, 13))],
        ),
        (
            "imap.web.de",
            vec![
                IpAddr::V4(Ipv4Addr::new(212, 227, 17, 162)),
                IpAddr::V4(Ipv4Addr::new(212, 227, 17, 178)),
            ],
        ),
        (
            "imap.ziggo.nl",
            vec![IpAddr::V4(Ipv4Addr::new(84, 116, 6, 3))],
        ),
        (
            "imap.zoho.eu",
            vec![IpAddr::V4(Ipv4Addr::new(185, 230, 214, 25))],
        ),
        (
            "imaps.bluewin.ch",
            vec![
                IpAddr::V4(Ipv4Addr::new(16, 62, 253, 42)),
                IpAddr::V4(Ipv4Addr::new(16, 63, 141, 244)),
                IpAddr::V4(Ipv4Addr::new(16, 63, 146, 183)),
            ],
        ),
        (
            "mail.buzon.uy",
            vec![IpAddr::V4(Ipv4Addr::new(185, 101, 93, 79))],
        ),
        (
            "mail.ecloud.global",
            vec![IpAddr::V4(Ipv4Addr::new(95, 217, 246, 96))],
        ),
        (
            "mail.ende.in.net",
            vec![IpAddr::V4(Ipv4Addr::new(95, 217, 5, 72))],
        ),
        (
            "mail.gmx.net",
            vec![
                IpAddr::V4(Ipv4Addr::new(212, 227, 17, 168)),
                IpAddr::V4(Ipv4Addr::new(212, 227, 17, 190)),
            ],
        ),
        (
            "mail.infomaniak.com",
            vec![
                IpAddr::V4(Ipv4Addr::new(83, 166, 143, 44)),
                IpAddr::V4(Ipv4Addr::new(83, 166, 143, 45)),
            ],
        ),
        (
            "mail.mymagenta.at",
            vec![IpAddr::V4(Ipv4Addr::new(80, 109, 253, 241))],
        ),
        (
            "mail.nubo.coop",
            vec![IpAddr::V4(Ipv4Addr::new(79, 99, 201, 10))],
        ),
        (
            "mail.riseup.net",
            vec![
                IpAddr::V4(Ipv4Addr::new(198, 252, 153, 70)),
                IpAddr::V4(Ipv4Addr::new(198, 252, 153, 71)),
            ],
        ),
        (
            "mail.systemausfall.org",
            vec![
                IpAddr::V4(Ipv4Addr::new(51, 75, 71, 249)),
                IpAddr::V4(Ipv4Addr::new(80, 153, 252, 42)),
            ],
        ),
        (
            "mail.systemli.org",
            vec![IpAddr::V4(Ipv4Addr::new(93, 190, 126, 36))],
        ),
        (
            "mehl.cloud",
            vec![IpAddr::V4(Ipv4Addr::new(95, 217, 223, 172))],
        ),
        (
            "mx.freenet.de",
            vec![
                IpAddr::V4(Ipv4Addr::new(195, 4, 92, 210)),
                IpAddr::V4(Ipv4Addr::new(195, 4, 92, 211)),
                IpAddr::V4(Ipv4Addr::new(195, 4, 92, 212)),
                IpAddr::V4(Ipv4Addr::new(195, 4, 92, 213)),
            ],
        ),
        (
            "newyear.aktivix.org",
            vec![IpAddr::V4(Ipv4Addr::new(162, 247, 75, 192))],
        ),
        (
            "pimap.schulon.org",
            vec![IpAddr::V4(Ipv4Addr::new(194, 77, 246, 20))],
        ),
        (
            "posteo.de",
            vec![
                IpAddr::V4(Ipv4Addr::new(185, 67, 36, 168)),
                IpAddr::V4(Ipv4Addr::new(185, 67, 36, 169)),
            ],
        ),
        (
            "psmtp.schulon.org",
            vec![IpAddr::V4(Ipv4Addr::new(194, 77, 246, 20))],
        ),
        (
            "secureimap.t-online.de",
            vec![
                IpAddr::V4(Ipv4Addr::new(194, 25, 134, 114)),
                IpAddr::V4(Ipv4Addr::new(194, 25, 134, 115)),
                IpAddr::V4(Ipv4Addr::new(194, 25, 134, 50)),
                IpAddr::V4(Ipv4Addr::new(194, 25, 134, 51)),
            ],
        ),
        (
            "securesmtp.t-online.de",
            vec![
                IpAddr::V4(Ipv4Addr::new(194, 25, 134, 110)),
                IpAddr::V4(Ipv4Addr::new(194, 25, 134, 46)),
            ],
        ),
        (
            "smtp.aliyun.com",
            vec![IpAddr::V4(Ipv4Addr::new(47, 246, 136, 232))],
        ),
        (
            "smtp.mail.de",
            vec![IpAddr::V4(Ipv4Addr::new(62, 201, 172, 21))],
        ),
        (
            "smtp.mail.ru",
            vec![
                IpAddr::V4(Ipv4Addr::new(217, 69, 139, 160)),
                IpAddr::V4(Ipv4Addr::new(94, 100, 180, 160)),
            ],
        ),
        (
            "imap.mail.yahoo.com",
            vec![
                IpAddr::V4(Ipv4Addr::new(87, 248, 103, 8)),
                IpAddr::V4(Ipv4Addr::new(212, 82, 101, 24)),
            ],
        ),
        (
            "smtp.mail.yahoo.com",
            vec![IpAddr::V4(Ipv4Addr::new(87, 248, 97, 36))],
        ),
        (
            "imap.mailo.com",
            vec![IpAddr::V4(Ipv4Addr::new(213, 182, 54, 20))],
        ),
        (
            "smtp.mailo.com",
            vec![IpAddr::V4(Ipv4Addr::new(213, 182, 54, 20))],
        ),
        (
            "smtp.naver.com",
            vec![IpAddr::V4(Ipv4Addr::new(125, 209, 238, 155))],
        ),
        (
            "smtp.ouvaton.coop",
            vec![IpAddr::V4(Ipv4Addr::new(194, 36, 166, 20))],
        ),
        (
            "smtp.purelymail.com",
            vec![IpAddr::V4(Ipv4Addr::new(18, 204, 123, 63))],
        ),
        (
            "imap.qq.com",
            vec![IpAddr::V4(Ipv4Addr::new(43, 129, 255, 54))],
        ),
        (
            "smtp.qq.com",
            vec![IpAddr::V4(Ipv4Addr::new(43, 129, 255, 54))],
        ),
        (
            "imap.rambler.ru",
            vec![
                IpAddr::V4(Ipv4Addr::new(81, 19, 77, 169)),
                IpAddr::V4(Ipv4Addr::new(81, 19, 77, 171)),
                IpAddr::V4(Ipv4Addr::new(81, 19, 77, 168)),
                IpAddr::V4(Ipv4Addr::new(81, 19, 77, 170)),
            ],
        ),
        (
            "smtp.rambler.ru",
            vec![
                IpAddr::V4(Ipv4Addr::new(81, 19, 77, 165)),
                IpAddr::V4(Ipv4Addr::new(81, 19, 77, 167)),
                IpAddr::V4(Ipv4Addr::new(81, 19, 77, 166)),
                IpAddr::V4(Ipv4Addr::new(81, 19, 77, 164)),
            ],
        ),
        (
            "imap.vivaldi.net",
            vec![IpAddr::V4(Ipv4Addr::new(31, 209, 137, 15))],
        ),
        (
            "smtp.vivaldi.net",
            vec![IpAddr::V4(Ipv4Addr::new(31, 209, 137, 12))],
        ),
        (
            "imap.vodafonemail.de",
            vec![IpAddr::V4(Ipv4Addr::new(2, 207, 150, 230))],
        ),
        (
            "smtp.vodafonemail.de",
            vec![IpAddr::V4(Ipv4Addr::new(2, 207, 150, 234))],
        ),
        (
            "smtp.web.de",
            vec![
                IpAddr::V4(Ipv4Addr::new(213, 165, 67, 108)),
                IpAddr::V4(Ipv4Addr::new(213, 165, 67, 124)),
            ],
        ),
        (
            "imap.yandex.com",
            vec![IpAddr::V4(Ipv4Addr::new(77, 88, 21, 125))],
        ),
        (
            "smtp.yandex.com",
            vec![IpAddr::V4(Ipv4Addr::new(77, 88, 21, 158))],
        ),
        (
            "smtp.ziggo.nl",
            vec![IpAddr::V4(Ipv4Addr::new(84, 116, 6, 3))],
        ),
        (
            "smtp.zoho.eu",
            vec![IpAddr::V4(Ipv4Addr::new(185, 230, 212, 164))],
        ),
        (
            "smtpauths.bluewin.ch",
            vec![IpAddr::V4(Ipv4Addr::new(195, 186, 120, 54))],
        ),
        (
            "stinpriza.net",
            vec![IpAddr::V4(Ipv4Addr::new(5, 9, 122, 184))],
        ),
        (
            "undernet.uy",
            vec![IpAddr::V4(Ipv4Addr::new(167, 62, 254, 153))],
        ),
        (
            "webbox222.server-home.org",
            vec![IpAddr::V4(Ipv4Addr::new(91, 203, 111, 88))],
        ),
    ])
});

/// Load hardcoded cache if everything else fails.
///
/// See <https://support.delta.chat/t/no-dns-resolution-result/2778> and
/// <https://github.com/deltachat/deltachat-core-rust/issues/4920> for reasons.
///
/// In the future we may pre-resolve all provider database addresses
/// and build them in.
fn load_hardcoded_cache(hostname: &str, port: u16) -> Vec<SocketAddr> {
    if let Some(ips) = DNS_PRELOAD.get(hostname) {
        ips.iter().map(|ip| SocketAddr::new(*ip, port)).collect()
    } else {
        Vec::new()
    }
}

async fn lookup_cache(
    context: &Context,
    host: &str,
    port: u16,
    alpn: &str,
    now: i64,
) -> Result<Vec<SocketAddr>> {
    let mut res = Vec::new();
    for cached_address in context
        .sql
        .query_map(
            "SELECT dns_cache.address
             FROM dns_cache
             LEFT JOIN connection_history
               ON dns_cache.hostname = connection_history.host
               AND dns_cache.address = connection_history.addr
               AND connection_history.port = ?
               AND connection_history.alpn = ?
             WHERE dns_cache.hostname = ?
             AND ? < dns_cache.timestamp + ?
             ORDER BY IFNULL(connection_history.timestamp, dns_cache.timestamp) DESC
             LIMIT 50",
            (port, alpn, host, now, super::CACHE_TTL),
            |row| {
                let address: String = row.get(0)?;
                Ok(address)
            },
            |rows| {
                rows.collect::<std::result::Result<Vec<String>, _>>()
                    .map_err(Into::into)
            },
        )
        .await?
    {
        match IpAddr::from_str(&cached_address) {
            Ok(ip_addr) => {
                let addr = SocketAddr::new(ip_addr, port);
                res.push(addr);
            }
            Err(err) => {
                warn!(
                    context,
                    "Failed to parse cached address {:?}: {:#}.", cached_address, err
                );
            }
        }
    }
    Ok(res)
}

/// Sorts DNS resolution results by connection timestamp in descending order
/// so IP addresses that we recently connected to successfully are tried first.
async fn sort_by_connection_timestamp(
    context: &Context,
    input: Vec<SocketAddr>,
    alpn: &str,
    host: &str,
) -> Result<Vec<SocketAddr>> {
    let mut res: Vec<(Option<i64>, SocketAddr)> = Vec::with_capacity(input.len());
    for addr in input {
        let timestamp = load_connection_timestamp(
            &context.sql,
            alpn,
            host,
            addr.port(),
            Some(&addr.ip().to_string()),
        )
        .await?;
        res.push((timestamp, addr));
    }
    res.sort_by_key(|(ts, _addr)| std::cmp::Reverse(*ts));
    Ok(res.into_iter().map(|(_ts, addr)| addr).collect())
}

/// Looks up hostname and port using DNS and updates the address resolution cache.
///
/// `alpn` is used to sort DNS results by the time we have successfully
/// connected to the IP address using given `alpn`.
/// If result sorting is not needed or `alpn` is unknown,
/// pass empty string here, e.g. for HTTP requests
/// or when resolving the IP address of SOCKS proxy.
///
/// If `load_cache` is true, appends cached results not older than 30 days to the end
/// or entries from fallback cache if there are no cached addresses.
pub(crate) async fn lookup_host_with_cache(
    context: &Context,
    hostname: &str,
    port: u16,
    alpn: &str,
    load_cache: bool,
) -> Result<Vec<SocketAddr>> {
    let now = time();
    let mut resolved_addrs = match lookup_host_and_update_cache(context, hostname, port, now).await
    {
        Ok(res) => res,
        Err(err) => {
            warn!(
                context,
                "DNS resolution for {hostname}:{port} failed: {err:#}."
            );
            Vec::new()
        }
    };
    if !alpn.is_empty() {
        resolved_addrs =
            sort_by_connection_timestamp(context, resolved_addrs, alpn, hostname).await?;
    }

    if load_cache {
        for addr in lookup_cache(context, hostname, port, alpn, now).await? {
            if !resolved_addrs.contains(&addr) {
                resolved_addrs.push(addr);
            }
        }

        if resolved_addrs.is_empty() {
            return Ok(load_hardcoded_cache(hostname, port));
        }
    }

    Ok(resolved_addrs)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::net::update_connection_history;
    use crate::test_utils::TestContext;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_sort_by_connection_timestamp() {
        let alice = &TestContext::new_alice().await;
        let now = time();

        let ipv6_addr = IpAddr::V6(Ipv6Addr::new(0x2a01, 0x4f8, 0x241, 0x4ce8, 0, 0, 0, 2));
        let ipv4_addr = IpAddr::V4(Ipv4Addr::new(116, 202, 233, 236));

        assert_eq!(
            sort_by_connection_timestamp(
                alice,
                vec![
                    SocketAddr::new(ipv6_addr, 993),
                    SocketAddr::new(ipv4_addr, 993)
                ],
                "imap",
                "nine.testrun.org"
            )
            .await
            .unwrap(),
            vec![
                SocketAddr::new(ipv6_addr, 993),
                SocketAddr::new(ipv4_addr, 993)
            ]
        );
        update_connection_history(
            alice,
            "imap",
            "nine.testrun.org",
            993,
            "116.202.233.236",
            now,
        )
        .await
        .unwrap();
        assert_eq!(
            sort_by_connection_timestamp(
                alice,
                vec![
                    SocketAddr::new(ipv6_addr, 993),
                    SocketAddr::new(ipv4_addr, 993)
                ],
                "imap",
                "nine.testrun.org"
            )
            .await
            .unwrap(),
            vec![
                SocketAddr::new(ipv4_addr, 993),
                SocketAddr::new(ipv6_addr, 993),
            ]
        );

        assert_eq!(
            sort_by_connection_timestamp(
                alice,
                vec![
                    SocketAddr::new(ipv6_addr, 465),
                    SocketAddr::new(ipv4_addr, 465)
                ],
                "smtp",
                "nine.testrun.org"
            )
            .await
            .unwrap(),
            vec![
                SocketAddr::new(ipv6_addr, 465),
                SocketAddr::new(ipv4_addr, 465),
            ]
        );
        update_connection_history(
            alice,
            "smtp",
            "nine.testrun.org",
            465,
            "116.202.233.236",
            now,
        )
        .await
        .unwrap();
        assert_eq!(
            sort_by_connection_timestamp(
                alice,
                vec![
                    SocketAddr::new(ipv6_addr, 465),
                    SocketAddr::new(ipv4_addr, 465)
                ],
                "smtp",
                "nine.testrun.org"
            )
            .await
            .unwrap(),
            vec![
                SocketAddr::new(ipv4_addr, 465),
                SocketAddr::new(ipv6_addr, 465),
            ]
        );

        update_connection_history(
            alice,
            "imap",
            "nine.testrun.org",
            993,
            "2a01:4f8:241:4ce8::2",
            now,
        )
        .await
        .unwrap();
        assert_eq!(
            sort_by_connection_timestamp(
                alice,
                vec![
                    SocketAddr::new(ipv6_addr, 993),
                    SocketAddr::new(ipv4_addr, 993)
                ],
                "imap",
                "nine.testrun.org"
            )
            .await
            .unwrap(),
            vec![
                SocketAddr::new(ipv6_addr, 993),
                SocketAddr::new(ipv4_addr, 993)
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_lookup_cache() {
        let alice = &TestContext::new_alice().await;

        let ipv4_addr = IpAddr::V4(Ipv4Addr::new(116, 202, 233, 236));
        let ipv6_addr = IpAddr::V6(Ipv6Addr::new(0x2a01, 0x4f8, 0x241, 0x4ce8, 0, 0, 0, 2));

        let now = time();
        assert!(lookup_cache(alice, "nine.testrun.org", 587, "smtp", now)
            .await
            .unwrap()
            .is_empty());

        update_cache(alice, "nine.testrun.org", "116.202.233.236", now)
            .await
            .unwrap();

        assert_eq!(
            lookup_cache(alice, "nine.testrun.org", 587, "smtp", now)
                .await
                .unwrap(),
            vec![SocketAddr::new(ipv4_addr, 587)]
        );

        // Cache should be returned for other ports and no ALPN as well,
        // port and ALPN should only affect the order
        assert_eq!(
            lookup_cache(alice, "nine.testrun.org", 443, "", now)
                .await
                .unwrap(),
            vec![SocketAddr::new(ipv4_addr, 443)]
        );

        update_cache(alice, "nine.testrun.org", "2a01:4f8:241:4ce8::2", now + 30)
            .await
            .unwrap();

        // New DNS cache entry should go first.
        assert_eq!(
            lookup_cache(alice, "nine.testrun.org", 443, "", now + 60)
                .await
                .unwrap(),
            vec![
                SocketAddr::new(ipv6_addr, 443),
                SocketAddr::new(ipv4_addr, 443)
            ],
        );

        // After successful connection to SMTP over port 465 using IPv4 address,
        // IPv4 address has higher priority.
        update_connection_history(
            alice,
            "smtp",
            "nine.testrun.org",
            465,
            "116.202.233.236",
            now + 100,
        )
        .await
        .unwrap();
        assert_eq!(
            lookup_cache(alice, "nine.testrun.org", 465, "smtp", now + 120)
                .await
                .unwrap(),
            vec![
                SocketAddr::new(ipv4_addr, 465),
                SocketAddr::new(ipv6_addr, 465)
            ]
        );

        // For other ports and ALPNs order remains the same.
        assert_eq!(
            lookup_cache(alice, "nine.testrun.org", 993, "imap", now + 120)
                .await
                .unwrap(),
            vec![
                SocketAddr::new(ipv6_addr, 993),
                SocketAddr::new(ipv4_addr, 993)
            ],
        );
        assert_eq!(
            lookup_cache(alice, "nine.testrun.org", 465, "imap", now + 120)
                .await
                .unwrap(),
            vec![
                SocketAddr::new(ipv6_addr, 465),
                SocketAddr::new(ipv4_addr, 465)
            ],
        );
        assert_eq!(
            lookup_cache(alice, "nine.testrun.org", 993, "smtp", now + 120)
                .await
                .unwrap(),
            vec![
                SocketAddr::new(ipv6_addr, 993),
                SocketAddr::new(ipv4_addr, 993)
            ],
        );
    }
}
