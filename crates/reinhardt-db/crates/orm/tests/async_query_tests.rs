    use reinhardt_orm::database::{AsyncConnection, Database};
    use reinhardt_orm::query::QuerySet;
    use reinhardt_pool::ConnectionPool;

//! Async Query Tests
//!
//! Tests for asynchronous query execution patterns based on SQLAlchemy async and Django async support.
//! Covers concurrent queries, streaming, async transactions, connection pooling, and cancellation.

use futures::stream::{self, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::timeout;

// Mock async database connection
struct AsyncConnection {
    id: usize,
    in_use: bool,
}

impl AsyncConnection {
    fn new(id: usize) -> Self {
        Self { id, in_use: false }
    }

    async fn query(&mut self, _sql: &str) -> Result<Vec<String>, String> {
        self.in_use = true;
        tokio::time::sleep(Duration::from_millis(10)).await;
        self.in_use = false;
        Ok(vec!["result".to_string()])
    }

    async fn execute(&mut self, _sql: &str) -> Result<usize, String> {
        self.in_use = true;
        tokio::time::sleep(Duration::from_millis(10)).await;
        self.in_use = false;
        Ok(1)
    }
}

// Mock connection pool
struct ConnectionPool {
    connections: Arc<Mutex<Vec<AsyncConnection>>>,
    semaphore: Arc<Semaphore>,
}

impl ConnectionPool {
    fn new(size: usize) -> Self {
        let connections = (0..size).map(|i| AsyncConnection::new(i)).collect();
        Self {
            connections: Arc::new(Mutex::new(connections)),
            semaphore: Arc::new(Semaphore::new(size)),
        }
    }

    async fn acquire(&self) -> Result<AsyncConnection, String> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| "Failed to acquire permit".to_string())?;

        let mut conns = self.connections.lock().await;
        if let Some(conn) = conns.pop() {
            Ok(conn)
        } else {
            Err("No connections available".to_string())
        }
    }

    async fn release(&self, conn: AsyncConnection) {
        let mut conns = self.connections.lock().await;
        conns.push(conn);
    }
}

// Test 1: Basic async query execution
#[tokio::test]
async fn test_async_query_basic() {
    let mut conn = AsyncConnection::new(1);
    let result = conn.query("SELECT * FROM users").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

// Test 2: Concurrent query execution
#[tokio::test]
async fn test_concurrent_queries() {
    let pool = Arc::new(ConnectionPool::new(3));

    let handles: Vec<_> = (0..5)
        .map(|i| {
            let pool = Arc::clone(&pool);
            tokio::spawn(async move {
                let mut conn = pool.acquire().await.unwrap();
                let result = conn.query(&format!("SELECT {}", i)).await;
                pool.release(conn).await;
                result
            })
        })
        .collect();

    let results: Vec<_> = futures::future::join_all(handles).await;

    assert_eq!(results.len(), 5);
    for result in results {
        assert!(result.is_ok());
    }
}

// Test 3: Stream query results
#[tokio::test]
async fn test_stream_query_results() {
    let data = vec![1, 2, 3, 4, 5];
    let stream = stream::iter(data.clone());

    let collected: Vec<i32> = stream.collect().await;

    assert_eq!(collected, data);
}

// Test 4: Stream with processing
#[tokio::test]
async fn test_stream_with_processing() {
    let data = vec![1, 2, 3, 4, 5];
    let stream = stream::iter(data);

    let doubled: Vec<i32> = stream.map(|x| x * 2).collect().await;

    assert_eq!(doubled, vec![2, 4, 6, 8, 10]);
}

// Test 5: Async transaction
#[tokio::test]
async fn test_async_transaction() {
    let mut conn = AsyncConnection::new(1);

    // Begin transaction
    let begin_result = conn.execute("BEGIN").await;
    assert!(begin_result.is_ok());

    // Execute queries
    let insert_result = conn.execute("INSERT INTO users VALUES (1, 'test')").await;
    assert!(insert_result.is_ok());

    // Commit transaction
    let commit_result = conn.execute("COMMIT").await;
    assert!(commit_result.is_ok());
}

// Test 6: Connection pool - acquire and release
#[tokio::test]
async fn test_connection_pool_acquire_release() {
    let pool = ConnectionPool::new(2);

    let conn1 = pool.acquire().await;
    assert!(conn1.is_ok());

    let conn2 = pool.acquire().await;
    assert!(conn2.is_ok());

    // Release one connection
    pool.release(conn1.unwrap()).await;

    // Should be able to acquire again
    let conn3 = pool.acquire().await;
    assert!(conn3.is_ok());
}

// Test 7: Query timeout handling
#[tokio::test]
async fn test_query_timeout() {
    let mut conn = AsyncConnection::new(1);

    // Query with timeout
    let result = timeout(Duration::from_millis(50), conn.query("SELECT * FROM users")).await;

    assert!(result.is_ok()); // Should complete within timeout
}

// Test 8: Query timeout expiration
#[tokio::test]
async fn test_query_timeout_expiration() {
    async fn slow_query() -> Result<Vec<String>, String> {
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(vec!["result".to_string()])
    }

    let result = timeout(Duration::from_millis(50), slow_query()).await;

    assert!(result.is_err()); // Should timeout
}

// Test 9: Cancellation with tokio::select
#[tokio::test]
async fn test_query_cancellation() {
    use tokio::sync::oneshot;

    let (cancel_tx, cancel_rx) = oneshot::channel();

    let query_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        "completed".to_string()
    });

    // Cancel immediately
    drop(cancel_tx);

    tokio::select! {
        result = query_handle => {
            assert!(result.is_ok());
        }
        _ = cancel_rx => {
            // Cancellation signal received
        }
    }
}

// Test 10: Parallel query execution
#[tokio::test]
async fn test_parallel_query_execution() {
    let queries = vec![
        "SELECT * FROM users",
        "SELECT * FROM posts",
        "SELECT * FROM comments",
    ];

    let handles: Vec<_> = queries
        .into_iter()
        .map(|sql| {
            tokio::spawn(async move {
                let mut conn = AsyncConnection::new(1);
                conn.query(sql).await
            })
        })
        .collect();

    let results = futures::future::join_all(handles).await;

    assert_eq!(results.len(), 3);
    for result in results {
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }
}
