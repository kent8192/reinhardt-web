# GraphQL Performance Benchmarks

## Overview

This document presents performance benchmarks comparing:

- **Direct GraphQL**: Baseline async-graphql execution
- **gRPC GraphQL**: GraphQL queries executed via gRPC protocol

## Benchmark Results

All benchmarks were run with `--quick` mode on a development machine.
Times are in microseconds (µs).

### Direct GraphQL (Baseline)

| Test                    | Time (µs) |
| ----------------------- | --------- |
| Simple Query            | 2.98      |
| Query with Arguments    | 4.37      |
| Query with Computation  | 4.02      |
| Nested Query (depth 10) | 4.44      |
| Simple Mutation         | 4.42      |

### gRPC GraphQL

| Test                    | Time (µs) | Overhead vs Direct |
| ----------------------- | --------- | ------------------ |
| Simple Query            | 3.61      | +21% (+0.63 µs)    |
| Query with Arguments    | 4.84      | +11% (+0.47 µs)    |
| Query with Computation  | 4.24      | +5% (+0.22 µs)     |
| Nested Query (depth 10) | 5.20      | +17% (+0.76 µs)    |

### Query Complexity (Direct GraphQL)

Impact of nested query depth on performance:

| Depth | Time (µs) |
| ----- | --------- |
| 5     | 4.19      |
| 10    | 4.41      |
| 20    | 4.84      |
| 50    | 5.58      |

## Analysis

### Key Findings

1. **Low Overhead**: gRPC adds only 5-21% overhead for GraphQL queries
2. **Consistent Performance**: Both direct and gRPC execution are highly performant (< 6 µs for most queries)
3. **Scalability**: Performance scales well with query complexity
4. **Sub-microsecond Overhead**: gRPC serialization/deserialization adds 0.2-0.8 µs

### When to Use Each Approach

#### Use Direct GraphQL When:

- Running within the same process
- Microsecond-level latency is critical
- No network communication required
- Maximum performance is needed

#### Use gRPC GraphQL When:

- Microservices architecture
- Language-agnostic clients needed
- Strong typing and code generation required
- Network communication is necessary
- Type-safe client-server contracts desired

### Performance Characteristics

- **Direct GraphQL**: 3-4 µs average, best for in-process execution
- **gRPC GraphQL**: 4-5 µs average, excellent for network communication
- **Overhead**: 0.2-0.8 µs (5-21%) for gRPC serialization

### Comparison with WebSocket

Note: While we benchmark gRPC overhead, WebSocket performance for GraphQL subscriptions typically involves:

- Connection establishment: ~1-5 ms (one-time cost)
- Message overhead: ~0.5-2 µs per message (similar to gRPC)
- Streaming efficiency: Both gRPC and WebSocket are efficient for real-time data

For subscriptions:

- **gRPC**: Better for typed, strongly-defined schemas with bidirectional streaming
- **WebSocket**: Better for browser-based clients and JSON-based protocols

## Recommendations

1. **Use Direct GraphQL** for monolithic applications or in-process execution
2. **Use gRPC GraphQL** for:
   - Microservices with typed contracts
   - Multi-language environments
   - When type safety across network boundaries is important
3. **Performance is Excellent** in both cases for typical workloads

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench --package reinhardt-graphql --features graphql-grpc

# Run specific benchmark group
cargo bench --package reinhardt-graphql --features graphql-grpc -- direct_graphql

# Quick mode (faster, less precise)
cargo bench --package reinhardt-graphql --features graphql-grpc -- --quick
```

## Hardware

Benchmarks should be run on consistent hardware. Results may vary based on:

- CPU performance
- Memory speed
- System load
- Rust compiler optimizations

## Conclusion

GraphQL over gRPC adds minimal overhead (5-21%, or 0.2-0.8 µs) while providing:

- Strong typing
- Cross-language support
- Efficient binary serialization
- Built-in streaming support

The overhead is negligible for most real-world applications where network latency (typically 1-100 ms) dominates the performance profile.
