//! DNS resolution and cache.

use anyhow::{Context as _, Result};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::str::FromStr;
use tokio::net::lookup_host;
use tokio::time::timeout;

use crate::context::Context;
use crate::tools::time;

async fn lookup_host_with_timeout(hostname: &str, port: u16) -> Result<Vec<SocketAddr>> {
    let res = timeout(super::TIMEOUT, lookup_host((hostname, port)))
        .await
        .context("DNS lookup timeout")?
        .context("DNS lookup failure")?;
    Ok(res.collect())
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

/// Looks up hostname and port using DNS and updates the address resolution cache.
///
/// If `load_cache` is true, appends cached results not older than 30 days to the end
/// or entries from fallback cache if there are no cached addresses.
pub(crate) async fn lookup_host_with_cache(
    context: &Context,
    hostname: &str,
    port: u16,
    load_cache: bool,
) -> Result<Vec<SocketAddr>> {
    let now = time();
    let mut resolved_addrs = match lookup_host_with_timeout(hostname, port).await {
        Ok(res) => res,
        Err(err) => {
            warn!(
                context,
                "DNS resolution for {hostname}:{port} failed: {err:#}."
            );
            Vec::new()
        }
    };

    for addr in &resolved_addrs {
        let ip_string = addr.ip().to_string();
        if ip_string == hostname {
            // IP address resolved into itself, not interesting to cache.
            continue;
        }

        info!(context, "Resolved {}:{} into {}.", hostname, port, &addr);

        // Update the cache.
        context
            .sql
            .execute(
                "INSERT INTO dns_cache
                 (hostname, address, timestamp)
                 VALUES (?, ?, ?)
                 ON CONFLICT (hostname, address)
                 DO UPDATE SET timestamp=excluded.timestamp",
                (hostname, ip_string, now),
            )
            .await?;
    }

    if load_cache {
        for cached_address in context
            .sql
            .query_map(
                "SELECT address
                 FROM dns_cache
                 WHERE hostname = ?
                 AND ? < timestamp + 30 * 24 * 3600
                 ORDER BY timestamp DESC",
                (hostname, now),
                |row| {
                    let address: String = row.get(0)?;
                    Ok(address)
                },
                |rows| {
                    rows.collect::<std::result::Result<Vec<_>, _>>()
                        .map_err(Into::into)
                },
            )
            .await?
        {
            match IpAddr::from_str(&cached_address) {
                Ok(ip_addr) => {
                    let addr = SocketAddr::new(ip_addr, port);
                    if !resolved_addrs.contains(&addr) {
                        resolved_addrs.push(addr);
                    }
                }
                Err(err) => {
                    warn!(
                        context,
                        "Failed to parse cached address {:?}: {:#}.", cached_address, err
                    );
                }
            }
        }

        if resolved_addrs.is_empty() {
            // Load hardcoded cache if everything else fails.
            //
            // See <https://support.delta.chat/t/no-dns-resolution-result/2778> and
            // <https://github.com/deltachat/deltachat-core-rust/issues/4920> for reasons.
            //
            // In the future we may pre-resolve all provider database addresses
            // and build them in.
            match hostname {
                "mail.sangham.net" => {
                    resolved_addrs.push(SocketAddr::new(
                        IpAddr::V6(Ipv6Addr::new(0x2a01, 0x4f8, 0xc17, 0x798c, 0, 0, 0, 1)),
                        port,
                    ));
                    resolved_addrs.push(SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::new(159, 69, 186, 85)),
                        port,
                    ));
                }
                "nine.testrun.org" => {
                    resolved_addrs.push(SocketAddr::new(
                        IpAddr::V6(Ipv6Addr::new(0x2a01, 0x4f8, 0x241, 0x4ce8, 0, 0, 0, 2)),
                        port,
                    ));
                    resolved_addrs.push(SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::new(116, 202, 233, 236)),
                        port,
                    ));
                }
                "disroot.org" => {
                    resolved_addrs.push(SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::new(178, 21, 23, 139)),
                        port,
                    ));
                }
                "mail.riseup.net" => {
                    resolved_addrs.push(SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::new(198, 252, 153, 70)),
                        port,
                    ));
                    resolved_addrs.push(SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::new(198, 252, 153, 71)),
                        port,
                    ));
                }
                "imap.gmail.com" => {
                    resolved_addrs.push(SocketAddr::new(
                        IpAddr::V6(Ipv6Addr::new(0x2a00, 0x1450, 0x400c, 0xc1f, 0, 0, 0, 0x6c)),
                        port,
                    ));
                    resolved_addrs.push(SocketAddr::new(
                        IpAddr::V6(Ipv6Addr::new(0x2a00, 0x1450, 0x400c, 0xc1f, 0, 0, 0, 0x6d)),
                        port,
                    ));
                    resolved_addrs.push(SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::new(142, 250, 110, 109)),
                        port,
                    ));
                    resolved_addrs.push(SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::new(142, 250, 110, 108)),
                        port,
                    ));
                }
                "smtp.gmail.com" => {
                    resolved_addrs.push(SocketAddr::new(
                        IpAddr::V6(Ipv6Addr::new(0x2a00, 0x1450, 0x4013, 0xc04, 0, 0, 0, 0x6c)),
                        port,
                    ));
                    resolved_addrs.push(SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::new(142, 250, 110, 109)),
                        port,
                    ));
                }
                _ => {}
            }
        }
    }

    Ok(resolved_addrs)
}
