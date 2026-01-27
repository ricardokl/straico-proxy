# Async Patterns

## Core Principles

1. **Zero-Allocation** - Future combinators over `async move` blocks
2. **Future Composition** - Chain operations with combinators
3. **Stream Processing** - Use `StreamExt` for streaming
4. **Monomorphization** - Use generics for zero-cost abstraction

## Pattern: Future Combinators

### Avoid: Async blocks (allocates)

```rust
async fn parse(response: reqwest::Response) -> Result<Value, Error> {
    let value = response.json::<T>().await?;
    let converted = convert(value)?;
    Ok(converted)
}
```

### Prefer: Future combinators (zero-alloc)

```rust
fn parse(response: reqwest::Response) -> impl Future<Output=Result<Value, Error>> {
    response.json::<T>()
        .map_err(Error::from)
        .then(|result| {
            let final_result = result.and_then(|v| convert(v)?);
            future::ready(final_result)
        })
}
```

**Benefits:** No heap allocation, compiler can optimize entire chain.

## Common Combinators

| Combinator | Purpose |
|------------|---------|
| `future::ready(x)` | Wrap sync value in future |
| `.then(f)` | Chain async with sync operations |
| `.and_then(f)` | Chain async operations |
| `.map(f)` | Transform result |
| `.map_err(f)` | Convert error type |

## Stream Processing

### Basic Stream Mapping

```rust
stream
    .map_err(Error::from)        // Convert errors
    .map(|bytes| parse(bytes))     // Transform data
    .try_collect()                   // Collect to Result<Vec>
    .await
```

### Stream Composition

```rust
let response_stream = initial_chunk
    .chain(heartbeat)
    .chain(actual_response)
    .chain(done);
```

## Monomorphization

While the proxy now uses a single concrete `StraicoProvider` type, the patterns shown here are useful for understanding how the code was structured and can be applied if multi-provider support is added in the future.

Generic functions generate specialized code at compile time:

```rust
async fn handle<P: ChatProvider>(provider: &P, request: Request)
    -> Result<HttpResponse, Error>
```

Compiler generates specialized versions for each concrete type:
- `handle::<StraicoProvider>`
- `handle::<GenericProvider<Groq>>` (if multi-provider is restored)
- etc.

**Result:** No runtime dispatch, enables inlining and optimization.

## Remote Handle Pattern

Split a future to wait for it in a stream:

```rust
let (remote, remote_handle) = future_response.remote_handle();

let heartbeat = stream.repeat(chunk)
    .take_until(remote);  // Stop when remote completes

let response_stream = remote_handle
    .map_ok(SseChunk::from)
    .into_stream();
```

## Stream Operators

| Operator | Purpose | Example |
|----------|---------|---------|
| `stream::once(fut)` | Single-item stream | `stream::once(future::ready(item))` |
| `stream::repeat(item)` | Infinite stream | `stream::repeat(heartbeat)` |
| `.chain(other)` | Concatenate streams | `s1.chain(s2)` |
| `.map(f)` | Transform items | `stream.map(|b| b.len())` |
| `.map_err(f)` | Transform errors | `stream.map_err(Error::from)` |
| `.take_until(fut)` | Stop when future completes | `stream.take_until(remote)` |
| `.into_stream()` | Future to Stream | `future.into_stream()` |

## Anti-Patterns

### ❌ Unnecessary `.await`

```rust
async fn bad() -> Result<T, Error> {
    let a = fetch1().await?;
    let b = fetch2().await?;
    Ok(a + b)
}
```

### ✅ Prefer combinators

```rust
async fn good() -> Result<T, Error> {
    fetch1()
        .and_then(|a| fetch2().map_ok(|b| a + b))
        .await
}
```

### ❌ Blocking in async

```rust
async fn bad() -> Result<T, Error> {
    std::fs::read_to_string("file.txt")?  // Blocks runtime
}
```

### ✅ Use spawn_blocking

```rust
async fn good() -> Result<T, Error> {
    tokio::task::spawn_blocking(|| {
        std::fs::read_to_string("file.txt")
    }).await??
}
```

## Performance Notes

- **Async blocks**: Allocate ~200 bytes for future state machine
- **Combinators**: Zero allocation (stack-only futures)
- **Streaming**: Process chunks incrementally vs holding full response in memory
- **Monomorphization**: Eliminates vtable lookups
