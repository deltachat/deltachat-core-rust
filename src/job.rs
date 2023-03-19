//! # Job module.
//!
//! This module implements a job queue maintained in the SQLite database
//! and job types.

#![allow(missing_docs)]

use std::fmt;
use std::sync::atomic::Ordering;

use anyhow::{Context as _, Result};
use deltachat_derive::{FromSql, ToSql};
use rand::{thread_rng, Rng};

use crate::context::Context;
use crate::imap::Imap;
use crate::scheduler::InterruptInfo;
use crate::tools::time;

// results in ~3 weeks for the last backoff timespan
const JOB_RETRIES: u32 = 17;

/// Job try result.
#[derive(Debug, Display)]
pub enum Status {
    Finished(Result<()>),
    RetryNow,
}

#[macro_export]
macro_rules! job_try {
    ($expr:expr) => {
        match $expr {
            std::result::Result::Ok(val) => val,
            std::result::Result::Err(err) => {
                return $crate::job::Status::Finished(Err(err.into()));
            }
        }
    };
    ($expr:expr,) => {
        $crate::job_try!($expr)
    };
}

#[derive(
    Debug,
    Display,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    FromPrimitive,
    ToPrimitive,
    FromSql,
    ToSql,
)]
#[repr(u32)]
pub enum Action {
    // This job will download partially downloaded messages completely
    // and is added when download_full() is called.
    // Most messages are downloaded automatically on fetch
    // and do not go through this job.
    DownloadMsg = 250,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub job_id: u32,
    pub action: Action,
    pub foreign_id: u32,
    pub desired_timestamp: i64,
    pub added_timestamp: i64,
    pub tries: u32,
}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}, action {}", self.job_id, self.action)
    }
}

impl Job {
    pub fn new(action: Action, foreign_id: u32) -> Self {
        let timestamp = time();

        Self {
            job_id: 0,
            action,
            foreign_id,
            desired_timestamp: timestamp,
            added_timestamp: timestamp,
            tries: 0,
        }
    }

    /// Deletes the job from the database.
    async fn delete(self, context: &Context) -> Result<()> {
        if self.job_id != 0 {
            context
                .sql
                .execute("DELETE FROM jobs WHERE id=?;", paramsv![self.job_id as i32])
                .await?;
        }

        Ok(())
    }

    /// Saves the job to the database, creating a new entry if necessary.
    ///
    /// The Job is consumed by this method.
    pub(crate) async fn save(self, context: &Context) -> Result<()> {
        info!(context, "saving job {:?}", self);

        if self.job_id != 0 {
            context
                .sql
                .execute(
                    "UPDATE jobs SET desired_timestamp=?, tries=? WHERE id=?;",
                    paramsv![
                        self.desired_timestamp,
                        i64::from(self.tries),
                        self.job_id as i32,
                    ],
                )
                .await?;
        } else {
            context.sql.execute(
                "INSERT INTO jobs (added_timestamp, action, foreign_id, desired_timestamp) VALUES (?,?,?,?);",
                paramsv![
                    self.added_timestamp,
                    self.action,
                    self.foreign_id,
                    self.desired_timestamp
                ]
            ).await?;
        }

        Ok(())
    }
}

pub(crate) enum Connection<'a> {
    Inbox(&'a mut Imap),
}

impl<'a> Connection<'a> {
    fn inbox(&mut self) -> &mut Imap {
        match self {
            Connection::Inbox(imap) => imap,
        }
    }
}

pub(crate) async fn perform_job(context: &Context, mut connection: Connection<'_>, mut job: Job) {
    info!(context, "job {} started...", &job);

    let try_res = match perform_job_action(context, &mut job, &mut connection, 0).await {
        Status::RetryNow => perform_job_action(context, &mut job, &mut connection, 1).await,
        x => x,
    };

    match try_res {
        Status::RetryNow => {
            let tries = job.tries + 1;

            if tries < JOB_RETRIES {
                info!(context, "increase job {} tries to {}", job, tries);
                job.tries = tries;
                let time_offset = get_backoff_time_offset(tries);
                job.desired_timestamp = time() + time_offset;
                info!(
                    context,
                    "job #{} not succeeded on try #{}, retry in {} seconds.",
                    job.job_id,
                    tries,
                    time_offset
                );
                job.save(context).await.unwrap_or_else(|err| {
                    error!(context, "failed to save job: {:#}", err);
                });
            } else {
                info!(
                    context,
                    "remove job {} as it exhausted {} retries", job, JOB_RETRIES
                );
                job.delete(context).await.unwrap_or_else(|err| {
                    error!(context, "failed to delete job: {:#}", err);
                });
            }
        }
        Status::Finished(res) => {
            if let Err(err) = res {
                warn!(
                    context,
                    "remove job {} as it failed with error {:#}", job, err
                );
            } else {
                info!(context, "remove job {} as it succeeded", job);
            }

            job.delete(context).await.unwrap_or_else(|err| {
                error!(context, "failed to delete job: {:#}", err);
            });
        }
    }
}

async fn perform_job_action(
    context: &Context,
    job: &mut Job,
    connection: &mut Connection<'_>,
    tries: u32,
) -> Status {
    info!(context, "begin immediate try {} of job {}", tries, job);

    let try_res = match job.action {
        Action::DownloadMsg => job.download_msg(context, connection.inbox()).await,
    };

    info!(context, "Finished immediate try {} of job {}", tries, job);

    try_res
}

fn get_backoff_time_offset(tries: u32) -> i64 {
    // Exponential backoff
    let n = 2_i32.pow(tries - 1) * 60;
    let mut rng = thread_rng();
    let r: i32 = rng.gen();
    let mut seconds = r % (n + 1);
    if seconds < 1 {
        seconds = 1;
    }
    i64::from(seconds)
}

pub(crate) async fn schedule_resync(context: &Context) -> Result<()> {
    context.resync_request.store(true, Ordering::Relaxed);
    context
        .scheduler
        .interrupt_inbox(InterruptInfo {
            probe_network: false,
        })
        .await;
    Ok(())
}

/// Adds a job to the database, scheduling it.
pub async fn add(context: &Context, job: Job) -> Result<()> {
    job.save(context).await.context("failed to save job")?;

    info!(context, "interrupt: imap");
    context
        .scheduler
        .interrupt_inbox(InterruptInfo::new(false))
        .await;
    Ok(())
}

/// Load jobs from the database.
///
/// The `probe_network` parameter decides how to query
/// jobs, this is tricky and probably wrong currently. Look at the
/// SQL queries for details.
pub(crate) async fn load_next(context: &Context, info: &InterruptInfo) -> Result<Option<Job>> {
    info!(context, "loading job");

    let query;
    let params;
    let t = time();

    if !info.probe_network {
        // processing for first-try and after backoff-timeouts:
        // process jobs in the order they were added.
        query = r#"
SELECT id, action, foreign_id, param, added_timestamp, desired_timestamp, tries
FROM jobs
WHERE desired_timestamp<=?
ORDER BY action DESC, added_timestamp
LIMIT 1;
"#;
        params = paramsv![t];
    } else {
        // processing after call to dc_maybe_network():
        // process _all_ pending jobs that failed before
        // in the order of their backoff-times.
        query = r#"
SELECT id, action, foreign_id, param, added_timestamp, desired_timestamp, tries
FROM jobs
WHERE tries>0
ORDER BY desired_timestamp, action DESC
LIMIT 1;
"#;
        params = paramsv![];
    };

    loop {
        let job_res = context
            .sql
            .query_row_optional(query, params.clone(), |row| {
                let job = Job {
                    job_id: row.get("id")?,
                    action: row.get("action")?,
                    foreign_id: row.get("foreign_id")?,
                    desired_timestamp: row.get("desired_timestamp")?,
                    added_timestamp: row.get("added_timestamp")?,
                    tries: row.get("tries")?,
                };

                Ok(job)
            })
            .await;

        match job_res {
            Ok(job) => return Ok(job),
            Err(err) => {
                // Remove invalid job from the DB
                info!(context, "cleaning up job, because of {:#}", err);

                // TODO: improve by only doing a single query
                let id = context
                    .sql
                    .query_row(query, params.clone(), |row| row.get::<_, i32>(0))
                    .await
                    .context("Failed to retrieve invalid job ID from the database")?;
                context
                    .sql
                    .execute("DELETE FROM jobs WHERE id=?;", paramsv![id])
                    .await
                    .with_context(|| format!("Failed to delete invalid job {id}"))?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestContext;

    async fn insert_job(context: &Context, foreign_id: i64, valid: bool) {
        let now = time();
        context
            .sql
            .execute(
                "INSERT INTO jobs
                   (added_timestamp, action, foreign_id, desired_timestamp)
                 VALUES (?, ?, ?, ?);",
                paramsv![
                    now,
                    if valid {
                        Action::DownloadMsg as i32
                    } else {
                        -1
                    },
                    foreign_id,
                    now
                ],
            )
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_load_next_job_two() -> Result<()> {
        // We want to ensure that loading jobs skips over jobs which
        // fails to load from the database instead of failing to load
        // all jobs.
        let t = TestContext::new().await;
        insert_job(&t, 1, false).await; // This can not be loaded into Job struct.
        let jobs = load_next(&t, &InterruptInfo::new(false)).await?;
        assert!(jobs.is_none());

        insert_job(&t, 1, true).await;
        let jobs = load_next(&t, &InterruptInfo::new(false)).await?;
        assert!(jobs.is_some());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_load_next_job_one() -> Result<()> {
        let t = TestContext::new().await;

        insert_job(&t, 1, true).await;

        let jobs = load_next(&t, &InterruptInfo::new(false)).await?;
        assert!(jobs.is_some());
        Ok(())
    }
}
