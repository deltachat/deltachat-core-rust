//! Connection pool.

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Weak};

use anyhow::{Context, Result};
use crossbeam_queue::ArrayQueue;
use rusqlite::Connection;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Inner connection pool.
#[derive(Debug)]
struct InnerPool {
    /// Available connections.
    connections: ArrayQueue<Connection>,

    /// Counts the number of available connections.
    semaphore: Arc<Semaphore>,
}

impl InnerPool {
    /// Puts a connection into the pool.
    ///
    /// The connection could be new or returned back.
    fn put(&self, connection: Connection) {
        self.connections.force_push(connection);
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
        let inner = Arc::new(InnerPool {
            connections: ArrayQueue::new(connections.len()),
            semaphore: Arc::new(Semaphore::new(connections.len())),
        });
        for connection in connections {
            inner.connections.force_push(connection);
        }
        Pool { inner }
    }

    /// Retrieves a connection from the pool.
    pub async fn get(&self) -> Result<PooledConnection> {
        let permit = self.inner.semaphore.clone().acquire_owned().await?;
        let conn = self
            .inner
            .connections
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
