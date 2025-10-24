# reinhardt-websockets

WebSocket support for the Reinhardt framework.

## Overview

WebSocket protocol support for real-time bidirectional communication. Includes connection management, message routing, room management, and WebSocket handler traits for building interactive applications.

## Features

### Implemented ✓

#### Connection Management

- `WebSocketConnection`: Manages individual WebSocket connections
  - Connection ID tracking
  - Send text, binary, and JSON messages
  - Connection state management (open/closed)
  - Async message sending with error handling
- `WebSocketError` and `WebSocketResult`: Comprehensive error handling
  - Connection errors
  - Send/receive errors
  - Protocol errors

#### Message Types

- `Message` enum: Multiple message types support
  - Text messages
  - Binary messages
  - Ping/Pong messages
  - Close messages with status codes
  - JSON serialization/deserialization helpers
- `WebSocketMessage` struct: Structured message format with timestamps
  - Message type classification
  - JSON data payload
  - Optional timestamp support

#### Room Management

- `RoomManager`: Multi-client room management
  - Join/leave room operations
  - Broadcast messages to specific rooms
  - Broadcast messages to all rooms
  - Room size tracking
  - List all active rooms
  - Thread-safe with async/await support

#### Handler Traits

- `WebSocketHandler`: Trait for implementing custom WebSocket handlers
  - `on_message`: Handle incoming messages
  - `on_connect`: Handle new connections
  - `on_disconnect`: Handle disconnections

### Planned

- WebSocket routing integration
- Authentication and authorization hooks
- Rate limiting and throttling
- Automatic reconnection support
- Message compression
- Custom protocol support
- Channel layers for distributed systems
- Consumer classes for advanced patterns
- Integration with Reinhardt middleware system
