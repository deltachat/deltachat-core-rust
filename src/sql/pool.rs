//! Connection pool.

use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Weak};

use parking_lot::{Condvar, Mutex};
use rusqlite::Connection;

/// Inner connection pool.
struct InnerPool {
    /// Available connections.
    connections: Mutex<Vec<Connection>>,

    /// Conditional variable to notify about added connections.
    ///
    /// Used to wait for available connection when the pool is empty.
    cond: Condvar,
}

impl InnerPool {
    /// Puts a connection into the pool.
    ///
    /// The connection could be new or returned back.
    fn put(&self, connection: Connection) {
        let mut connections = self.connections.lock();
        connections.push(connection);
        drop(connections);
        self.cond.notify_one();
    }
}

/// Pooled connection.
pub struct PooledConnection {
    /// Weak reference to the pool used to return the connection back.
    pool: Weak<InnerPool>,

    /// Only `None` right after moving the connection back to the pool.
    conn: Option<Connection>,
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
#[derive(Clone)]
pub struct Pool {
    /// Reference to the actual connection pool.
    inner: Arc<InnerPool>,
}

impl fmt::Debug for Pool {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Pool")
    }
}

impl Pool {
    /// Creates a new connection pool.
    pub fn new(connections: Vec<Connection>) -> Self {
        let inner = Arc::new(InnerPool {
            connections: Mutex::new(connections),
            cond: Condvar::new(),
        });
        Pool { inner }
    }

    /// Retrieves a connection from the pool.
    pub fn get(&self) -> PooledConnection {
        let mut connections = self.inner.connections.lock();

        loop {
            if let Some(conn) = connections.pop() {
                return PooledConnection {
                    pool: Arc::downgrade(&self.inner),
                    conn: Some(conn),
                };
            }

            self.inner.cond.wait(&mut connections);
        }
    }
}
