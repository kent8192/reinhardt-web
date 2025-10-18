/// Advanced connection pool types
/// Based on SQLAlchemy's pool implementations
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub trait ConnectionPool: Send + Sync {
    fn get_connection(&self) -> Result<PooledConnection, PoolError>;
    fn return_connection(&self, conn: PooledConnection);
    fn size(&self) -> usize;
    fn active_connections(&self) -> usize;
}

#[derive(Clone)]
pub struct PooledConnection {
    pub id: usize,
    pub created_at: Instant,
    pub last_used: Instant,
    // In a production implementation, this would contain the actual database connection
    // For now, we keep metadata for testing and demonstration purposes
}

#[derive(Debug)]
pub enum PoolError {
    NoConnectionsAvailable,
    ConnectionFailed(String),
    Timeout,
    MaxConnectionsReached,
}

/// Internal pool state shared across threads
struct PoolState {
    available: VecDeque<PooledConnection>,
    active: usize,
    next_id: usize,
}

/// Queue-based connection pool (FIFO)
pub struct QueuePool {
    pub max_connections: usize,
    pub timeout: Duration,
    state: Arc<Mutex<PoolState>>,
}

impl QueuePool {
    /// Create a new queue-based connection pool with FIFO behavior
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::pool_types::QueuePool;
    /// use std::time::Duration;
    ///
    /// let pool = QueuePool::new(10, Duration::from_secs(30));
    /// assert_eq!(pool.max_connections, 10);
    /// assert_eq!(pool.timeout, Duration::from_secs(30));
    /// ```
    pub fn new(max_connections: usize, timeout: Duration) -> Self {
        Self {
            max_connections,
            timeout,
            state: Arc::new(Mutex::new(PoolState {
                available: VecDeque::new(),
                active: 0,
                next_id: 1,
            })),
        }
    }

    fn create_connection(&self, id: usize) -> PooledConnection {
        PooledConnection {
            id,
            created_at: Instant::now(),
            last_used: Instant::now(),
        }
    }
}

impl ConnectionPool for QueuePool {
    fn get_connection(&self) -> Result<PooledConnection, PoolError> {
        let mut state = self.state.lock().unwrap();

        // Try to get an existing connection from the pool
        if let Some(mut conn) = state.available.pop_front() {
            conn.last_used = Instant::now();
            state.active += 1;
            return Ok(conn);
        }

        // Create a new connection if under max limit
        if state.active < self.max_connections {
            let id = state.next_id;
            state.next_id += 1;
            state.active += 1;
            return Ok(self.create_connection(id));
        }

        // Pool is exhausted
        Err(PoolError::MaxConnectionsReached)
    }

    fn return_connection(&self, conn: PooledConnection) {
        let mut state = self.state.lock().unwrap();
        state.available.push_back(conn);
        if state.active > 0 {
            state.active -= 1;
        }
    }

    fn size(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.available.len() + state.active
    }

    fn active_connections(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.active
    }
}

/// No pooling - create connection on demand
pub struct NullPool {
    next_id: Arc<Mutex<usize>>,
}

impl NullPool {
    /// Create a new NullPool that creates connections on demand without pooling
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::pool_types::{NullPool, ConnectionPool};
    ///
    /// let pool = NullPool::new();
    /// assert_eq!(pool.size(), 0); // No connections stored in pool
    /// assert_eq!(pool.active_connections(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            next_id: Arc::new(Mutex::new(1)),
        }
    }
}

impl Default for NullPool {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionPool for NullPool {
    fn get_connection(&self) -> Result<PooledConnection, PoolError> {
        let mut id = self.next_id.lock().unwrap();
        let conn_id = *id;
        *id += 1;

        Ok(PooledConnection {
            id: conn_id,
            created_at: Instant::now(),
            last_used: Instant::now(),
        })
    }

    fn return_connection(&self, _conn: PooledConnection) {
        // No-op for NullPool - connections are discarded
    }

    fn size(&self) -> usize {
        0
    }

    fn active_connections(&self) -> usize {
        0
    }
}

/// Single connection shared across all requests
pub struct StaticPool {
    connection: Arc<Mutex<Option<PooledConnection>>>,
}

impl StaticPool {
    /// Create a new StaticPool that maintains a single shared connection
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::pool_types::{StaticPool, ConnectionPool};
    ///
    /// let pool = StaticPool::new();
    /// assert_eq!(pool.size(), 1); // Always one connection
    /// ```
    pub fn new() -> Self {
        Self {
            connection: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for StaticPool {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionPool for StaticPool {
    fn get_connection(&self) -> Result<PooledConnection, PoolError> {
        let mut conn_opt = self.connection.lock().unwrap();

        // If connection doesn't exist, create it
        if conn_opt.is_none() {
            *conn_opt = Some(PooledConnection {
                id: 1,
                created_at: Instant::now(),
                last_used: Instant::now(),
            });
        }

        // Clone the connection (it's always the same logical connection)
        Ok(conn_opt.as_ref().unwrap().clone())
    }

    fn return_connection(&self, _conn: PooledConnection) {
        // No-op for StaticPool - connection is kept alive
    }

    fn size(&self) -> usize {
        1
    }

    fn active_connections(&self) -> usize {
        let conn_opt = self.connection.lock().unwrap();
        if conn_opt.is_some() {
            1
        } else {
            0
        }
    }
}

/// One connection per thread
pub struct SingletonThreadPool {
    connections: Arc<Mutex<std::collections::HashMap<std::thread::ThreadId, PooledConnection>>>,
    next_id: Arc<Mutex<usize>>,
}

impl SingletonThreadPool {
    /// Create a new SingletonThreadPool that maintains one connection per thread
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::pool_types::{SingletonThreadPool, ConnectionPool};
    ///
    /// let pool = SingletonThreadPool::new();
    /// assert_eq!(pool.size(), 0); // No connections until first request
    /// ```
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(std::collections::HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }
}

impl Default for SingletonThreadPool {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionPool for SingletonThreadPool {
    fn get_connection(&self) -> Result<PooledConnection, PoolError> {
        let thread_id = std::thread::current().id();
        let mut connections = self.connections.lock().unwrap();

        // If connection exists for this thread, return it (cloned)
        if let Some(conn) = connections.get(&thread_id) {
            return Ok(conn.clone());
        }

        // Create new connection for this thread
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;

        let conn = PooledConnection {
            id,
            created_at: Instant::now(),
            last_used: Instant::now(),
        };

        connections.insert(thread_id, conn.clone());
        Ok(conn)
    }

    fn return_connection(&self, _conn: PooledConnection) {
        // Kept in thread-specific storage
    }

    fn size(&self) -> usize {
        let connections = self.connections.lock().unwrap();
        connections.len()
    }

    fn active_connections(&self) -> usize {
        let connections = self.connections.lock().unwrap();
        connections.len()
    }
}

/// Async-compatible queue pool
pub struct AsyncAdaptedQueuePool {
    pub max_connections: usize,
    // Note: In a real implementation, this would include:
    // - tokio::sync::Semaphore for async connection limiting
    // - Arc<Mutex<VecDeque<PooledConnection>>> for async-safe queue
    // - async get_connection and return_connection methods
}

impl AsyncAdaptedQueuePool {
    /// Create a new async-compatible queue pool for use with async runtimes
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::pool_types::AsyncAdaptedQueuePool;
    ///
    /// let pool = AsyncAdaptedQueuePool::new(20);
    /// assert_eq!(pool.max_connections, 20);
    /// ```
    pub fn new(max_connections: usize) -> Self {
        Self { max_connections }
    }
}

/// Assertion pool for testing (detects connection leaks)
pub struct AssertionPool {
    // Note: In a real implementation, this would include:
    // - HashSet<ConnectionId> for tracking active connections
    // - Panic on drop if connections not returned
    // - Debug logging for connection lifecycle
}

// Advanced pool features (future implementation):
// - Pre-ping functionality: Check connection validity before returning from pool
// - Connection recycling: Reset connection state after return to pool
// - Pool overflow handling: Create temporary connections beyond max_connections
// - Connection timeout: Remove stale connections after idle timeout
// - Pool statistics: Track get/return counts, wait times, active connections

pub struct PoolStatistics {
    pub total_connections: usize,
    pub active_connections: usize,
    pub idle_connections: usize,
    pub total_requests: usize,
    pub failed_requests: usize,
}

impl PoolStatistics {
    /// Create a new PoolStatistics instance for tracking connection pool metrics
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::pool_types::PoolStatistics;
    ///
    /// let stats = PoolStatistics::new();
    /// assert_eq!(stats.total_connections, 0);
    /// assert_eq!(stats.active_connections, 0);
    /// assert_eq!(stats.idle_connections, 0);
    /// assert_eq!(stats.total_requests, 0);
    /// assert_eq!(stats.failed_requests, 0);
    /// ```
    pub fn new() -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            idle_connections: 0,
            total_requests: 0,
            failed_requests: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_pool_creation() {
        let pool = QueuePool::new(10, Duration::from_secs(30));
        assert_eq!(pool.max_connections, 10);
    }

    #[test]
    fn test_queue_pool_timeout() {
        let pool = QueuePool::new(5, Duration::from_secs(10));
        assert_eq!(pool.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_null_pool_size() {
        let pool = NullPool::new();
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_static_pool_size() {
        let pool = StaticPool::new();
        assert_eq!(pool.size(), 1);
    }

    #[test]
    fn test_async_adapted_queue_pool_creation() {
        let pool = AsyncAdaptedQueuePool::new(20);
        assert_eq!(pool.max_connections, 20);
    }

    #[test]
    fn test_queue_pool_different_sizes() {
        let small_pool = QueuePool::new(1, Duration::from_secs(30));
        let large_pool = QueuePool::new(100, Duration::from_secs(30));

        assert_eq!(small_pool.max_connections, 1);
        assert_eq!(large_pool.max_connections, 100);
    }

    #[test]
    fn test_queue_pool_different_timeouts() {
        let short_timeout = QueuePool::new(10, Duration::from_secs(5));
        let long_timeout = QueuePool::new(10, Duration::from_millis(500));

        assert_eq!(short_timeout.timeout, Duration::from_secs(5));
        assert_eq!(long_timeout.timeout, Duration::from_millis(500));
    }

    #[test]
    fn test_multiple_pool_types() {
        let queue_pool = QueuePool::new(10, Duration::from_secs(30));
        let null_pool = NullPool::new();
        let static_pool = StaticPool::new();

        assert_eq!(queue_pool.max_connections, 10);
        assert_eq!(null_pool.size(), 0);
        assert_eq!(static_pool.size(), 1);
    }

    #[test]
    fn test_async_pool_different_sizes() {
        let pool1 = AsyncAdaptedQueuePool::new(5);
        let pool2 = AsyncAdaptedQueuePool::new(50);

        assert_eq!(pool1.max_connections, 5);
        assert_eq!(pool2.max_connections, 50);
    }

    #[test]
    fn test_pool_error_types() {
        let timeout_error = PoolError::Timeout;
        let max_error = PoolError::MaxConnectionsReached;

        matches!(timeout_error, PoolError::Timeout);
        matches!(max_error, PoolError::MaxConnectionsReached);
    }

    #[test]
    fn test_pooled_connection_creation() {
        let now = Instant::now();
        let conn = PooledConnection {
            id: 1,
            created_at: now,
            last_used: now,
        };

        assert_eq!(conn.id, 1);
        assert!(conn.created_at.elapsed().as_secs() < 1);
    }

    #[test]
    fn test_pooled_connection_different_ids() {
        let now = Instant::now();
        let conn1 = PooledConnection {
            id: 1,
            created_at: now,
            last_used: now,
        };
        let conn2 = PooledConnection {
            id: 2,
            created_at: now,
            last_used: now,
        };

        assert_ne!(conn1.id, conn2.id);
    }

    #[test]
    fn test_pooled_connection_age() {
        let now = Instant::now();
        let conn = PooledConnection {
            id: 1,
            created_at: now,
            last_used: now,
        };

        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(conn.created_at.elapsed().as_millis() >= 10);
    }

    #[test]
    fn test_queue_pool_zero_timeout() {
        let pool = QueuePool::new(10, Duration::from_secs(0));
        assert_eq!(pool.timeout, Duration::from_secs(0));
    }

    #[test]
    fn test_large_queue_pool() {
        let pool = QueuePool::new(1000, Duration::from_secs(60));
        assert_eq!(pool.max_connections, 1000);
    }
}
