//! DNS resolution and cache.

use anyhow::{Context as _, Result};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::str::FromStr;
use tokio::net::lookup_host;
use tokio::time::timeout;

use super::load_connection_timestamp;
use crate::context::Context;
use crate::tools::time;

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

/// Load hardcoded cache if everything else fails.
///
/// See <https://support.delta.chat/t/no-dns-resolution-result/2778> and
/// <https://github.com/deltachat/deltachat-core-rust/issues/4920> for reasons.
///
/// In the future we may pre-resolve all provider database addresses
/// and build them in.
fn load_hardcoded_cache(hostname: &str, port: u16) -> Vec<SocketAddr> {
    match hostname {
        "mail.sangham.net" => {
            vec![
                SocketAddr::new(
                    IpAddr::V6(Ipv6Addr::new(0x2a01, 0x4f8, 0xc17, 0x798c, 0, 0, 0, 1)),
                    port,
                ),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(159, 69, 186, 85)), port),
            ]
        }
        "nine.testrun.org" => {
            vec![
                SocketAddr::new(
                    IpAddr::V6(Ipv6Addr::new(0x2a01, 0x4f8, 0x241, 0x4ce8, 0, 0, 0, 2)),
                    port,
                ),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(116, 202, 233, 236)), port),
            ]
        }
        "disroot.org" => {
            vec![SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(178, 21, 23, 139)),
                port,
            )]
        }
        "mail.riseup.net" => {
            vec![
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(198, 252, 153, 70)), port),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(198, 252, 153, 71)), port),
            ]
        }
        "imap.gmail.com" => {
            vec![
                SocketAddr::new(
                    IpAddr::V6(Ipv6Addr::new(0x2a00, 0x1450, 0x400c, 0xc1f, 0, 0, 0, 0x6c)),
                    port,
                ),
                SocketAddr::new(
                    IpAddr::V6(Ipv6Addr::new(0x2a00, 0x1450, 0x400c, 0xc1f, 0, 0, 0, 0x6d)),
                    port,
                ),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(142, 250, 110, 109)), port),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(142, 250, 110, 108)), port),
            ]
        }
        "smtp.gmail.com" => {
            vec![
                SocketAddr::new(
                    IpAddr::V6(Ipv6Addr::new(0x2a00, 0x1450, 0x4013, 0xc04, 0, 0, 0, 0x6c)),
                    port,
                ),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(142, 250, 110, 109)), port),
            ]
        }
        _ => Vec::new(),
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
    let mut res: Vec<(Option<i64>, SocketAddr)> = Vec::new();
    for addr in input {
        let timestamp =
            load_connection_timestamp(context, alpn, host, addr.port(), &addr.ip().to_string())
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
