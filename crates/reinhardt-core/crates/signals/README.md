# reinhardt-signals

Event-driven hooks for model lifecycle events - Enhanced implementation compatible with Django signals

## Overview

Type-safe signal system for decoupled communication between components. Provides lifecycle signals for models, migrations, requests, and custom events. Supports both asynchronous and synchronous signal dispatch patterns with advanced features like middleware, signal composition, and performance monitoring.

## Installation

Add `reinhardt` to your `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["signals"] }

# Or use a preset:
# reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }      # All features
```

Then import signal features:

```rust
use reinhardt::core::signals::{Signal, post_save, pre_save, post_delete, pre_delete};
use reinhardt::core::signals::{SignalError, SignalDispatcher, AsyncSignalDispatcher};
use reinhardt::core::signals::{SignalContext, SignalMetrics};
```

**Note:** Signal features are included in the `standard` and `full` feature presets.

## Implemented Features ✓

### Core Signal System

- **Signal**: Generic event dispatcher with type-safe receivers
- **SignalName**: Type-safe signal name wrapper with built-in constants
  - Built-in names: `PRE_SAVE`, `POST_SAVE`, `PRE_DELETE`, `POST_DELETE`, etc.
  - Custom names: `SignalName::custom("my_signal")`
  - Validated custom names: `SignalName::custom_validated("my_signal")` (enforces snake_case, prevents reserved names)
- **SignalDispatcher**: Common trait for all signal dispatchers (async and sync)
- **AsyncSignalDispatcher**: Trait extending SignalDispatcher with async-specific methods
- **SignalError**: Error type for signal operations
- **SignalRegistry**: Global registry for signal management
  - `get_signal()`: Get or create a signal by `SignalName`
  - `get_signal_with_string()`: Get or create a signal by string name
  - Ensures singleton behavior - same signal name returns same instance

### Signal Connection & Dispatch

- **Basic Connection**: `connect()` for simple receiver registration
- **Connection with Options**: `connect_with_options()` with sender filtering, dispatch_uid, and priority
- **Full Options**: `connect_with_full_options()` including predicates
- **Conditional Receivers**: `connect_if()` for predicate-based execution
- **Priority-based Execution**: Receivers sorted by priority (higher executes first)
- **Sender Filtering**: TypeId-based sender type filtering
- **dispatch_uid**: Prevents duplicate receiver registration with unique identifiers
- **Disconnect**: `disconnect()` to remove receivers by dispatch_uid
- **Disconnect All**: `disconnect_all()` to clear all receivers

### Signal Dispatch Methods

- **Standard Send**: `send()` for normal signal dispatch
- **Send with Sender**: `send_with_sender()` with sender type filtering
- **Robust Send**: `send_robust()` catches errors without stopping other receivers
- **Async Send**: `send_async()` for fire-and-forget dispatch

### Signal Middleware

- **SignalMiddleware Trait**: Intercept and transform signals at various stages
- **before_send**: Hook before signal is sent to receivers
- **after_send**: Hook after signal has been sent to all receivers
- **before_receiver**: Hook when a receiver is about to execute
- **after_receiver**: Hook after a receiver has executed
- **Middleware Chaining**: Multiple middlewares can be added to a signal
- **Early Termination**: Middleware can stop signal propagation

### Signal Composition

- **Chain**: `chain()` connects signals in sequence
- **Chain with Transformation**: `chain_with()` transforms data between signals
- **Merge**: `Signal::merge()` combines multiple signals into one
- **Filter**: `filter()` creates a filtered signal based on predicates
- **Map**: `map()` transforms signal values through a function

### Testing Utilities

- **SignalSpy**: Testing utility to record and assert signal calls
  - `call_count()`: Number of times signal was sent
  - `was_called()`: Check if signal was called
  - `was_called_with_count()`: Check exact call count
  - `total_receivers_called()`: Total receiver executions
  - `has_errors()`: Check for any errors
  - `errors()`: Get all error messages
  - `reset()`: Clear recorded calls
  - `instances()`: Get all sent instances
  - `last_instance()`: Get the last sent instance

### Performance Monitoring

- **SignalMetrics**: Performance metrics collection
  - `send_count`: Total number of signals sent
  - `receiver_executions`: Total receiver executions
  - `failed_executions`: Number of failed executions
  - `success_rate()`: Success rate percentage
  - `avg_execution_time()`: Average receiver execution time
  - `min_execution_time()`: Minimum execution time
  - `max_execution_time()`: Maximum execution time
- **Zero-cost**: Metrics use atomic operations with minimal overhead
- **Thread-safe**: Concurrent metrics collection
- **Resettable**: `reset_metrics()` for testing and monitoring

### Signal Context

- **SignalContext**: Pass metadata alongside signals
  - `insert()`: Add context values
  - `get()`: Retrieve context values
  - `contains_key()`: Check key existence
  - `remove()`: Remove context values
  - `clear()`: Clear all context data
  - `keys()`: Get all context keys

### Built-in Model Lifecycle Signals

- **pre_save**: Before saving a model instance
- **post_save**: After saving a model instance
- **pre_delete**: Before deleting a model instance
- **post_delete**: After deleting a model instance
- **pre_init**: At the beginning of model initialization (includes `PreInitEvent`)
- **post_init**: At the end of model initialization (includes `PostInitEvent`)
- **m2m_changed**: When many-to-many relationships change
  - Includes `M2MChangeEvent` with action type and related objects
  - Supports `M2MAction` enum: PreAdd, PostAdd, PreRemove, PostRemove, PreClear, PostClear

### Built-in Migration Signals

- **pre_migrate**: Before running migrations (includes `MigrationEvent`)
- **post_migrate**: After running migrations (includes `MigrationEvent`)

### Built-in Request Signals

- **request_started**: When an HTTP request starts (includes `RequestStartedEvent`)
- **request_finished**: When an HTTP request finishes (includes `RequestFinishedEvent`)
- **got_request_exception**: When an exception occurs during request handling (includes `GotRequestExceptionEvent`)

### Built-in Management Signals

- **setting_changed**: When a configuration setting is changed (includes `SettingChangedEvent`)
- **class_prepared**: When a model class is prepared (includes `ClassPreparedEvent`)

### Database Lifecycle Events (SQLAlchemy-style)

Module: `db_events`

- **before_insert**: Before inserting a record
- **after_insert**: After inserting a record
- **before_update**: Before updating a record
- **after_update**: After updating a record
- **before_delete**: Before deleting a record
- **after_delete**: After deleting a record
- **DbEvent**: Generic database event structure with table, id, and data fields

### Synchronous Signal Support

Module: `dispatch`

- **SyncSignal**: Django-style synchronous signal dispatcher
- **Weak References**: Supports weak receiver references for automatic cleanup
- **use_caching**: Optional caching for improved performance
- **Compatible API**: Mimics Django's Signal class interface

### Developer Convenience

- **connect_receiver! Macro**: Simplified receiver connection syntax supporting all connection options
- **Receiver Registry**: Automatic signal connection via declarative registration
  - `ReceiverRegistryEntry`: Registry entry for automatic connection
  - `auto_connect_receivers()`: Connect all registered receivers at once
  - Used by `#[receiver]` macro for auto-discovery

## Rust-Specific Enhancements ✓

- **Compile-time Type Safety**: TypeId-based sender filtering catches errors at compile time
- **Zero-cost Abstractions**: Efficient Arc-based receiver storage with atomic metrics
- **Memory Safety**: Automatic cleanup with Rust's ownership system
- **Async/Await Native**: Built on Tokio for efficient async execution
- **Ergonomic Macros**: `connect_receiver!` macro for cleaner syntax
- **Thread Safety**: RwLock for concurrent receiver access
- **Performance Monitoring**: Built-in metrics with atomic operations

## Advanced Features ✓

### Signal Batching

Aggregate multiple signal emissions into single dispatches for improved performance.

- **SignalBatcher**: Batches signals based on size and time thresholds
- **BatchConfig**: Configuration for batch behavior (max size, flush interval)
- Automatic and manual flush support

### Signal Debugging

Visual debugging tools for signal flow analysis.

- **SignalDebugger**: Middleware for tracking signal execution
- **DebugEvent**: Records signal events with timestamps
- **SignalStats**: Statistics about signal performance
- Generate detailed debug reports

### Dead Letter Queue

Handles failed signals with retry logic and backoff strategies.

- **DeadLetterQueue**: Queue for failed signals
- **RetryStrategy**: Configurable retry strategies (Immediate, FixedDelay, ExponentialBackoff, LinearBackoff)
- **DlqConfig**: Configure max retries and queue limits

### Signal History

Track signal emission patterns over time.

- **SignalHistory**: Records signal emissions with timestamps
- **HistoryEntry**: Single history entry with payload and metadata
- **HistoryConfig**: Configure history size and filtering
- Query and analyze past emissions

### Signal Persistence

Store and replay signals from durable storage.

- **PersistentSignal**: Automatically persists signals to storage
- **SignalStore**: Abstract trait for storage backends
- **MemoryStore**: In-memory storage implementation
- **StoredSignal**: Stored signal with metadata
- Event sourcing support

### Performance Profiling

Detailed performance analysis of signal systems.

- **SignalProfiler**: Middleware for performance tracking
- **ReceiverProfile**: Per-receiver performance statistics
- Track execution times (min/max/avg)
- Identify bottlenecks and slow receivers

### Signal Replay

Replay previously stored signals for debugging and testing.

- **SignalReplayer**: Replay stored signals
- **ReplayConfig**: Configure replay behavior
- **ReplaySpeed**: Control replay speed (Instant, Realtime, Fast, Custom)
- Useful for debugging and event sourcing

### Signal Throttling

Rate-limit signal emissions to protect downstream systems.

- **SignalThrottle**: Throttle signal emissions
- **ThrottleStrategy**: Various strategies (FixedWindow, SlidingWindow, TokenBucket, LeakyBucket)
- **ThrottleConfig**: Configure rate limits and window sizes

### Signal Visualization

Generate visual representations of signal connections.

- **SignalGraph**: Graph representation of signal flow
- **SignalNode**: Nodes in the signal graph (signals, receivers, middleware)
- **SignalEdge**: Connections between nodes
- Export to DOT (Graphviz), Mermaid, and ASCII formats

### Documentation Generation

Auto-generate documentation from signal metadata.

- **SignalDocGenerator**: Generate documentation for signals
- **SignalDocumentation**: Documentation for a single signal
- **ReceiverDocumentation**: Documentation for receivers
- Generate markdown documentation

## Integration Features ✓

### ORM Integration

Automatic signal dispatch from ORM operations.

- **OrmSignalAdapter**: Bridges ORM events to signals
- **OrmEventListener**: Trait for receiving ORM events
- Automatically dispatches pre_save, post_save, pre_delete, post_delete signals

### Transaction Support

Signals tied to database transaction lifecycle.

- **TransactionContext**: Context for transaction signals
- **TransactionSignals**: Manual transaction signal control
- **on_commit()**: Signal for transaction commit
- **on_rollback()**: Signal for transaction rollback
- **on_begin()**: Signal for transaction begin

### Distributed Signals

Cross-service signal dispatch via message brokers.

- **DistributedSignal**: Signal that works across services
- **MessageBroker**: Abstract trait for message brokers
- **InMemoryBroker**: In-memory broker for testing
- **DistributedEvent**: Event wrapper with source service info
- Support for Redis Pub/Sub, RabbitMQ, Kafka (via broker implementations)

### WebSocket Integration

Real-time signal propagation to connected WebSocket clients.

- **WebSocketSignalBridge**: Bridge signals to WebSocket connections
- **WebSocketMessage**: Message format for WebSocket events
- Broadcast signals to all connected clients
- Subscribe/unsubscribe support

### GraphQL Subscriptions

Signal-based GraphQL subscription support.

- **GraphQLSubscriptionBridge**: Bridge signals to GraphQL subscriptions
- **SubscriptionEvent**: GraphQL subscription event format
- Connect signals to GraphQL subscription resolvers
- Stream-based subscription updates

## Dependency Injection Integration ✓

Signals can be integrated with Reinhardt's dependency injection system.

- **InjectableSignal**: Trait for DI-aware signals
- **ReceiverContext**: Context passed to receivers with DI support
- Requires `di` feature flag
- Allows receivers to resolve dependencies from DI container

Example usage:

```rust
#[cfg(feature = "di")]
use reinhardt::core::signals::{Signal, InjectableSignal, ReceiverContext};

#[cfg(feature = "di")]
signal.connect_with_context(|instance, ctx: ReceiverContext| async move {
    // Resolve dependencies from ctx
    Ok(())
});
```

## Usage Examples

## Basic Signal Connection

```rustuse reinhardt_signals::{post_save, Signal, SignalError};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct User {
    id: i32,
    name: String,
}

// Connect a receiver to the post_save signalpost_save::<User>().connect(|instance: Arc<User>| async move {
    println!("User saved: {:?}", instance);
    Ok(())
});

// Send the signallet user = User { id: 1, name: "Alice".to_string() };
post_save::<User>().send(user).await?;
```

## Sender Filtering

```rustuse std::any::TypeId;

struct BlogPost;struct ForumPost;

// Connect receiver that only listens to BlogPost signalspost_save::<Post>().connect_with_options(
    |instance: Arc<Post>| async move {
        println!("Blog post saved!");
        Ok(())
    },
    Some(TypeId::of::<BlogPost>()),  // Only trigger for BlogPost
    None,
);

// This will trigger the receiverpost_save::<Post>()
    .send_with_sender(post, Some(TypeId::of::<BlogPost>()))
    .await?;

// This will NOT trigger the receiverpost_save::<Post>()
    .send_with_sender(post, Some(TypeId::of::<ForumPost>()))
    .await?;
```

## Prevent Duplicate Registration with dispatch_uid

```rustuse reinhardt_signals::connect_receiver;

// First registrationconnect_receiver!(
    post_save::<User>(),
    |instance| async move { Ok(()) },
    dispatch_uid = "my_unique_handler"
);

// This will replace the first registration (not duplicate)connect_receiver!(
    post_save::<User>(),
    |instance| async move { Ok(()) },
    dispatch_uid = "my_unique_handler"
);
```

## Robust Error Handling

```rust
// Send signal robustly - continues even if a receiver failslet results = post_save::<User>().send_robust(user, None).await;

for result in results {
    match result {
        Ok(_) => println!("Receiver succeeded"),
        Err(e) => eprintln!("Receiver failed: {}", e),
    }
}
```

## Using the connect_receiver! Macro

```rustuse reinhardt_signals::{connect_receiver, post_save};

// Simple connectionconnect_receiver!(post_save::<User>(), my_receiver);

// With dispatch_uidconnect_receiver!(
    post_save::<User>(),
    my_receiver,
    dispatch_uid = "unique_id"
);

// With sender filteringconnect_receiver!(
    post_save::<User>(),
    my_receiver,
    sender = BlogPost
);

// With bothconnect_receiver!(
    post_save::<User>(),
    my_receiver,
    sender = BlogPost,
    dispatch_uid = "blog_handler"
);
```

## Priority-based Execution

```rustuse reinhardt_signals::{connect_receiver, post_save};

// Higher priority receivers execute firstconnect_receiver!(
    post_save::<User>(),
    |instance| async move {
        println!("Critical: Log to audit system");
        Ok(())
    },
    priority = 100  // Executes first
);

connect_receiver!(
    post_save::<User>(),
    |instance| async move {
        println!("Normal: Send notification email");
        Ok(())
    },
    priority = 50  // Executes second
);

connect_receiver!(
    post_save::<User>(),
    |instance| async move {
        println!("Low: Update cache");
        Ok(())
    },
    priority = 10  // Executes last
);

// Can combine priority with other optionsconnect_receiver!(
    post_save::<User>(),
    my_receiver,
    sender = AdminUser,
    priority = 200,
    dispatch_uid = "admin_handler"
);
```

## Conditional Receivers (Predicates)

```rustuse reinhardt_signals::post_save;

// Only execute for users with admin rolepost_save::<User>().connect_if(
    |instance| async move {
        println!("Admin user saved: {:?}", instance.name);
        Ok(())
    },
    |user| user.is_admin  // Predicate - only executes if true
);

// Only execute for active userspost_save::<User>().connect_if(
    |instance| async move {
        send_welcome_email(&instance).await?;
        Ok(())
    },
    |user| user.is_active
);

// Complex conditionspost_save::<User>().connect_if(
    |instance| async move {
        alert_security_team(&instance).await?;
        Ok(())
    },
    |user| user.login_attempts > 5 && !user.is_locked
);

// Combine with priority and other optionssignal.connect_with_full_options(
    |instance| async move {
        process_premium_user(&instance).await?;
        Ok(())
    },
    None,  // sender_type_id
    Some("premium_handler".to_string()),  // dispatch_uid
    100,  // priority
    Some(|user: &User| user.is_premium),  // predicate
);
```

## Signal Middleware

Middleware allows you to intercept and modify signal behavior at various stages:

```rustuse reinhardt_signals::{Signal, SignalMiddleware, SignalError};
use std::sync::Arc;

// Create a logging middlewarestruct LoggingMiddleware;

#[async_trait::async_trait]
impl SignalMiddleware<User> for LoggingMiddleware {
    async fn before_send(&self, instance: &User) -> Result<bool, SignalError> {
        println!("Signal about to be sent for user: {}", instance.id);
        Ok(true) // Return false to stop signal propagation
    }

    async fn after_send(&self, instance: &User, results: &[Result<(), SignalError>]) -> Result<(), SignalError> {
        println!("Signal sent. {} receivers executed", results.len());
        Ok(())
    }

    async fn before_receiver(&self, instance: &User, dispatch_uid: Option<&str>) -> Result<bool, SignalError> {
        println!("Receiver {:?} about to execute", dispatch_uid);
        Ok(true) // Return false to skip this receiver
    }

    async fn after_receiver(&self, instance: &User, dispatch_uid: Option<&str>, result: &Result<(), SignalError>) -> Result<(), SignalError> {
        if result.is_err() {
            println!("Receiver {:?} failed", dispatch_uid);
        }
        Ok(())
    }
}

// Add middleware to a signallet signal = post_save::<User>();
signal.add_middleware(LoggingMiddleware);

// Create middleware for authentication/authorizationstruct AuthMiddleware {
    required_role: String,
}

#[async_trait::async_trait]
impl SignalMiddleware<User> for AuthMiddleware {
    async fn before_send(&self, instance: &User) -> Result<bool, SignalError> {
        if !instance.has_role(&self.required_role) {
            return Ok(false); // Block signal if user doesn't have required role
        }
        Ok(true)
    }
}
```

## Testing with SignalSpy

`SignalSpy` is a testing utility that records signal calls for assertion:

```rustuse reinhardt_signals::{Signal, SignalSpy};

#[tokio::test]
async fn test_user_creation() {
    let signal = post_save::<User>();
    let spy = SignalSpy::new();

    // Attach spy as middleware
    signal.add_middleware(spy.clone());

    // Connect receiver
    signal.connect(|user| async move {
        send_welcome_email(&user).await?;
        Ok(())
    });

    // Perform action
    let user = User::new("Alice");
    signal.send(user).await.unwrap();

    // Assert signal was called
    assert!(spy.was_called());
    assert_eq!(spy.call_count(), 1);
    assert_eq!(spy.total_receivers_called(), 1);
    assert!(!spy.has_errors());
}

#[tokio::test]
async fn test_error_handling() {
    let signal = post_save::<User>();
    let spy = SignalSpy::new();
    signal.add_middleware(spy.clone());

    // Receiver that might fail
    signal.connect(|user| async move {
        if user.email.is_empty() {
            return Err(SignalError::new("Email required"));
        }
        Ok(())
    });

    let user = User { email: String::new(), ..Default::default() };
    let _ = signal.send_robust(user, None).await;

    // Check for errors
    assert!(spy.has_errors());
    let errors = spy.errors();
    assert_eq!(errors[0], "Email required");
}
```

## Custom Signals

```rustuse reinhardt_signals::Signal;

// Define a custom signallet payment_completed = Signal::<PaymentInfo>::new("payment_completed");

// Connect receiverspayment_completed.connect(|info| async move {
    println!("Payment completed: ${}", info.amount);
    Ok(())
});

// Send the signalpayment_completed.send(payment_info).await?;
```

## Disconnecting Receivers

```rustlet signal = post_save::<User>();

// Connect with dispatch_uidconnect_receiver!(
    signal,
    my_receiver,
    dispatch_uid = "removable_handler"
);

// Later, disconnect itsignal.disconnect("removable_handler");
```

## Built-in Signal Types

Reinhardt provides a comprehensive set of signal types for different framework events:

## Model Lifecycle Signals

```rustuse reinhardt_signals::{pre_init, post_init, pre_save, post_save, pre_delete, post_delete, PreInitEvent, PostInitEvent};

// Pre-init: Called before model initializationpre_init::<User>().connect(|event| async move {
    println!("Initializing model: {}", event.model_type);
    Ok(())
});

// Post-init: Called after model initializationpost_init::<User>().connect(|event| async move {
    println!("User initialized: {:?}", event.instance);
    Ok(())
});

// Model save/delete signalspre_save::<User>().connect(|user| async move { Ok(()) });
post_save::<User>().connect(|user| async move { Ok(()) });pre_delete::<User>().connect(|user| async move { Ok(()) });
post_delete::<User>().connect(|user| async move { Ok(()) });
```

## Many-to-Many Relationship Signals

```rustuse reinhardt_signals::{m2m_changed, M2MAction, M2MChangeEvent};

m2m_changed::<User, Group>().connect(|event| async move {
    match event.action {
        M2MAction::PostAdd => println!("Added {} groups to user", event.related.len()),
        M2MAction::PostRemove => println!("Removed {} groups from user", event.related.len()),
        M2MAction::PostClear => println!("Cleared all groups from user"),
        _ => {}
    }
    Ok(())
});

// Sending m2m_changed signallet event = M2MChangeEvent::new(user, M2MAction::PostAdd, vec![group1, group2])
    .with_reverse(false)
    .with_model_name("Group");m2m_changed::<User, Group>().send(event).await?;
```

## Migration Signals

```rustuse reinhardt_signals::{pre_migrate, post_migrate, MigrationEvent};

// Pre-migrate: Before running migrationspre_migrate().connect(|event| async move {
    println!("Running migration {} for app {}", event.migration_name, event.app_name);
    Ok(())
});

// Post-migrate: After running migrationspost_migrate().connect(|event| async move {
    println!("Completed migration: {}", event.migration_name);
    Ok(())
});

// Sending migration signalslet event = MigrationEvent::new("myapp", "0001_initial")
    .with_plan(vec!["CreateModel".to_string()]);pre_migrate().send(event).await?;
```

## Request Handling Signals

```rustuse reinhardt_signals::{request_started, request_finished, got_request_exception};
use reinhardt_signals::{RequestStartedEvent, RequestFinishedEvent, GotRequestExceptionEvent};

// Request startedrequest_started().connect(|event| async move {
    println!("Request started: {:?}", event.environ);
    Ok(())
});

// Request finishedrequest_finished().connect(|event| async move {
    println!("Request completed");
    Ok(())
});

// Exception handlinggot_request_exception().connect(|event| async move {
    eprintln!("Request error: {}", event.error_message);
    Ok(())
});
```

## Management Signals

```rustuse reinhardt_signals::{setting_changed, class_prepared};
use reinhardt_signals::{SettingChangedEvent, ClassPreparedEvent};

// Setting changedsetting_changed().connect(|event| async move {
    println!("Setting {} changed from {:?} to {}",
        event.setting_name, event.old_value, event.new_value);
    Ok(())
});

// Class preparedclass_prepared().connect(|event| async move {
    println!("Model {} prepared for app {}", event.model_name, event.app_label);
    Ok(())
});
```

## Signal Composition

Reinhardt signals support powerful composition patterns for building complex event flows:

## Chaining Signals

```rustuse reinhardt_signals::Signal;

let user_created = Signal::<User>::new("user_created");let send_welcome_email = Signal::<User>::new("send_welcome_email");

// Chain signals - when user_created is sent, send_welcome_email is automatically triggereduser_created.chain(&send_welcome_email);

send_welcome_email.connect(|user| async move {
    email_service.send_welcome(&user).await?;
    Ok(())
});

// Sending to user_created will trigger both signalsuser_created.send(new_user).await?;
```

## Chaining with Transformation

```rustlet user_created = Signal::<User>::new("user_created");
let send_notification = Signal::<Notification>::new("send_notification");

// Transform User to Notification when chaininguser_created.chain_with(&send_notification, |user: Arc<User>| {
    Notification {
        user_id: user.id,
        message: format!("Welcome, {}!", user.name),
        priority: Priority::High,
    }
});
```

## Merging Multiple Signals

```rustlet user_login = Signal::<User>::new("user_login");
let user_signup = Signal::<User>::new("user_signup");let password_reset = Signal::<User>::new("password_reset");

// Merge multiple signals into onelet any_user_activity = Signal::merge(vec![&user_login, &user_signup, &password_reset]);

// This receiver triggers for any of the three eventsany_user_activity.connect(|user| async move {
    update_last_activity(&user).await?;
    Ok(())
});
```

## Filtering Signal Emissions

```rustlet user_signal = Signal::<User>::new("user_changes");

// Create a filtered signal that only triggers for admin userslet admin_signal = user_signal.filter(|user| user.is_admin);

admin_signal.connect(|admin_user| async move {
    log_admin_action(&admin_user).await?;
    Ok(())
});

// Only admin users will trigger the filtered signaluser_signal.send(regular_user).await?; // Won't trigger admin_signal
user_signal.send(admin_user).await?;   // Will trigger admin_signal
```

## Mapping Signal Values

```rustlet user_signal = Signal::<User>::new("user_signal");

// Map User to user IDlet user_id_signal: Signal<i32> = user_signal.map(|user: Arc<User>| user.id);

user_id_signal.connect(|user_id| async move {
    println!("User ID: {}", user_id);
    Ok(())
});
```

## Complex Composition

Combine multiple composition operators for sophisticated event flows:

```rustlet user_signal = Signal::<User>::new("users");

// Filter for admin users, then map to their IDslet admin_ids: Signal<i32> = user_signal
    .filter(|user| user.is_admin)
    .map(|user: Arc<User>| user.id);

admin_ids.connect(|admin_id| async move {
    audit_log.record_admin_activity(*admin_id).await?;
    Ok(())
});
```

## Performance Metrics

Monitor signal performance with built-in metrics collection:

```rustlet signal = Signal::<User>::new("user_updates");

signal.connect(|user| async move {
    process_user(&user).await?;
    Ok(())
});

// Send some signalsfor i in 0..100 {
    signal.send(create_user(i)).await?;
}

// Get metricslet metrics = signal.metrics();
println!("Signals sent: {}", metrics.send_count);println!("Receivers executed: {}", metrics.receiver_executions);
println!("Success rate: {:.2}%", metrics.success_rate());println!("Avg execution time: {:?}", metrics.avg_execution_time());
println!("Min execution time: {:?}", metrics.min_execution_time());println!("Max execution time: {:?}", metrics.max_execution_time());

// Reset metricssignal.reset_metrics();
```

**Metrics Available:**

- `send_count` - Total number of times the signal was sent
- `receiver_executions` - Total number of receiver executions
- `failed_executions` - Number of failed receiver executions
- `success_rate()` - Success rate as a percentage (0-100)
- `avg_execution_time()` - Average receiver execution time
- `min_execution_time()` - Minimum receiver execution time
- `max_execution_time()` - Maximum receiver execution time

**Features:**

- Zero-cost when not accessed
- Thread-safe atomic operations
- Shared across cloned signals
- Resettable for testing and monitoring

## Django vs Reinhardt Signals Comparison

| Feature             | Django | Reinhardt | Notes                                      |
|---------------------|--------|-----------|--------------------------------------------|
| Sender filtering    | ✅      | ✅         | Rust uses TypeId for type-safe filtering   |
| dispatch_uid        | ✅      | ✅         | Prevents duplicate registration            |
| send_robust         | ✅      | ✅         | Continues execution even if receivers fail |
| Weak references     | ✅      | ✅         | Available in dispatch module               |
| @receiver decorator | ✅      | ✅         | Use `connect_receiver!` macro              |
| Async support       | ⚠️     | ✅         | Native async/await support                 |
| Type safety         | ❌      | ✅         | Compile-time type checking                 |
| Memory safety       | ⚠️     | ✅         | Guaranteed by Rust ownership               |
| Middleware          | ❌      | ✅         | Intercept signals at multiple stages       |
| Signal composition  | ❌      | ✅         | Chain, merge, filter, and map signals      |
| Performance metrics | ❌      | ✅         | Built-in performance monitoring            |
| Predicates          | ❌      | ✅         | Conditional receiver execution             |
| Priority ordering   | ❌      | ✅         | Execute receivers in priority order        |

## Performance

Reinhardt signals are designed for high performance:

- **Arc-based storage**: Efficient cloning of receivers with minimal overhead
- **RwLock for concurrency**: Multiple readers, single writer for optimal throughput
- **Zero allocations** for sender filtering (TypeId comparison)
- **Atomic metrics**: Lock-free performance monitoring
- **Async runtime**: Leverages Tokio for efficient async execution
- **No heap allocations**: For simple signal dispatch paths

## Migration from Django

```python
# Django
from django.db.models.signals import post_savefrom django.dispatch import receiver

@receiver(post_save, sender=User)def on_user_saved(sender, instance, created, **kwargs):
    print(f"User saved: {instance}")
```

```rust
// Reinhardtuse reinhardt_signals::{connect_receiver, post_save};

connect_receiver!(
    post_save::<User>(),
    |instance| async move {
        println!("User saved: {:?}", instance);
        Ok(())
    },
    sender = UserModel
);
```

## License

This crate is part of the Reinhardt project and follows the same licensing terms.