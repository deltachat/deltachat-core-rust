//! # SQLite connection pool.
//!
//! The connection pool holds a number of SQLite connections and allows to allocate them.
//! When allocated connection is dropped, underlying connection is returned back to the pool.
//!
//! The pool is organized as a stack. It always allocates the most recently used connection.
//! Each SQLite connection has its own page cache, so allocating recently used connections
//! improves the performance compared to, for example, organizing the pool as a queue
//! and returning the least recently used connection each time.

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Weak};

use anyhow::{Context, Result};
use parking_lot::Mutex;
use rusqlite::Connection;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Inner connection pool.
#[derive(Debug)]
struct InnerPool {
    /// Available connections.
    connections: Mutex<Vec<Connection>>,

    /// Counts the number of available connections.
    semaphore: Arc<Semaphore>,
}

impl InnerPool {
    /// Puts a connection into the pool.
    ///
    /// The connection could be new or returned back.
    fn put(&self, connection: Connection) {
        let mut connections = self.connections.lock();
        connections.push(connection);
        drop(connections);
    }
}

/// Pooled connection.
pub struct PooledConnection {
    /// Weak reference to the pool used to return the connection back.
    pool: Weak<InnerPool>,

    /// Only `None` right after moving the connection back to the pool.
    conn: Option<Connection>,

    /// Semaphore permit, dropped after returning the connection to the pool.
    _permit: OwnedSemaphorePermit,
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        // Put the connection back unless the pool is already dropped.
        if let Some(pool) = self.pool.upgrade() {
            if let Some(conn) = self.conn.take() {
                pool.put(conn);
            }
        }
    }
}

impl Deref for PooledConnection {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        self.conn.as_ref().unwrap()
    }
}

impl DerefMut for PooledConnection {
    fn deref_mut(&mut self) -> &mut Connection {
        self.conn.as_mut().unwrap()
    }
}

/// Connection pool.
#[derive(Clone, Debug)]
pub struct Pool {
    /// Reference to the actual connection pool.
    inner: Arc<InnerPool>,
}

impl Pool {
    /// Creates a new connection pool.
    pub fn new(connections: Vec<Connection>) -> Self {
        let semaphore = Arc::new(Semaphore::new(connections.len()));
        let inner = Arc::new(InnerPool {
            connections: Mutex::new(connections),
            semaphore,
        });
        Pool { inner }
    }

    /// Retrieves a connection from the pool.
    pub async fn get(&self) -> Result<PooledConnection> {
        let permit = self.inner.semaphore.clone().acquire_owned().await?;
        let mut connections = self.inner.connections.lock();
        let conn = connections
            .pop()
            .context("got a permit when there are no connections in the pool")?;
        let conn = PooledConnection {
            pool: Arc::downgrade(&self.inner),
            conn: Some(conn),
            _permit: permit,
        };
        Ok(conn)
    }
}
