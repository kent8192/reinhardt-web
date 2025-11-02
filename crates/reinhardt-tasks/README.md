# reinhardt-tasks

Background task processing

## Overview

Background task queue for executing long-running or scheduled tasks asynchronously.

Supports task scheduling, retries, task priorities, and multiple worker processes.

## Features

### Implemented ✓

#### Core Task System

- **Task Trait**: Basic task interface
  - Task ID (`TaskId`): UUID-based unique identifier
  - Task name and priority management
  - Priority range: 0-9 (default: 5)
- **TaskExecutor Trait**: Asynchronous task execution interface
- **TaskStatus**: Task lifecycle management
  - `Pending`: Waiting
  - `Running`: Executing
  - `Success`: Completed successfully
  - `Failure`: Failed
  - `Retry`: Retrying

#### Task Backends

- **TaskBackend Trait**: Task backend abstraction interface
  - Task enqueuing (`enqueue`)
  - Task dequeuing (`dequeue`)
  - Status retrieval (`get_status`)
  - Status update (`update_status`)
- **DummyBackend**: Dummy backend for testing
  - Simple implementation that always succeeds
- **ImmediateBackend**: Backend for immediate execution
  - For synchronous task execution
- **RedisBackend** (feature: `redis-backend`): Redis-based distributed task queue
  - Task metadata storage using Redis
  - Queue-based task distribution
  - Customizable key prefix
- **SqliteBackend** (feature: `database-backend`): SQLite-based task persistence
  - Task storage in SQLite database
  - Automatic table creation
  - FIFO-based task retrieval

#### Task Queue

- **TaskQueue**: Task queue management
  - Configurable queue name
  - Retry count configuration (default: 3)
  - Task enqueuing via backend
- **QueueConfig**: Queue configuration
  - Customizable queue name
  - Maximum retry count setting

#### Task Scheduling

- **Scheduler**: Task scheduler
  - Task and schedule registration
  - Foundation for schedule-based task execution
- **Schedule Trait**: Schedule interface
  - Next execution time calculation
- **CronSchedule**: Cron expression-based scheduling
  - Cron expression storage and management

#### Worker System

- **Worker**: Task worker
  - Concurrent execution count configuration (default: 4)
  - Task retrieval and execution from backend
  - Graceful shutdown
  - Task processing loop (polling-based)
  - Error handling and status updates
  - Shutdown signaling via broadcast channel
- **WorkerConfig**: Worker configuration
  - Worker name setting
  - Concurrent execution count customization
  - Polling interval configuration (default: 1 second)

#### Task Chains

- **TaskChain**: Task chain management
  - Sequential execution of multiple tasks
  - Chain status management (Pending, Running, Completed, Failed)
  - Task addition and chain progression control
- **TaskChainBuilder**: Builder pattern for chain construction
  - Fluent interface for adding tasks
  - Bulk task addition
- **ChainStatus**: Chain lifecycle management

#### Result Handling

- **TaskOutput**: Task execution result
  - Task ID and string representation of result
- **TaskResult**: Task result type
  - Error handling via Result type
- **TaskResultMetadata**: Result metadata with status
  - Management of status, result, error, and timestamp
- **ResultBackend Trait**: Result persistence interface
  - Result storage (`store_result`)
  - Result retrieval (`get_result`)
  - Result deletion (`delete_result`)
- **MemoryResultBackend**: In-memory result backend
  - Result storage for testing
  - Concurrent access control via RwLock

#### Retry & Backoff

- **RetryStrategy**: Retry strategy configuration
  - Exponential backoff (`exponential_backoff`)
  - Fixed delay (`fixed_delay`)
  - No retry (`no_retry`)
  - Configuration for max retries, initial delay, max delay, multiplier
  - Jitter support (Thundering Herd Problem mitigation)
- **RetryState**: Retry state tracking
  - Retry attempt count recording
  - Next retry delay calculation
  - Retry eligibility determination
  - State reset

#### Error Handling

- **TaskError**: Task-related errors
  - Execution failure (`ExecutionFailed`)
  - Task not found (`TaskNotFound`)
  - Queue error (`QueueError`)
  - Serialization failure (`SerializationFailed`)
  - Timeout (`Timeout`)
  - Max retries exceeded (`MaxRetriesExceeded`)
- **TaskExecutionError**: Backend execution errors
  - Execution failure, task not found, backend error

## Testing

Redis backend tests are executed using TestContainers:

```bash
cargo test --package reinhardt-tasks --features all-backends
```

Tests run serially with `#[serial(redis)]` attribute to prevent Redis container conflicts.
