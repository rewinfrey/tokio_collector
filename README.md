# tokio_collector

### About

`tokio_collector` is a demo CLI that illustrates how to coordinate between a main loop and a background collector process.

**Key Points:**

- **Asynchronous Execution:** Uses `tokio::spawn` to run the background collector concurrently with the main loop.
- **Inter-Task Communication:** Employs a bounded `mpsc::channel` for sending data from the main loop to the collector.
- **Simulated Tick:** Utilizes a repeating `interval` to generate random numbers at each tick, which are then sent over the channel.
- **Buffer Flushing:**
  - The collector maintains an internal buffer of random numbers.
  - The buffer is flushed when it reaches the configured `FLUSH_THRESHOLD`.
  - The buffer is also flushed at regular intervals defined by the flush interval.
- **Signal Handling:** The main loop uses `tokio::select!` with `tokio::signal::ctrl_c()` to listen for a `SIGINT` signal.
- **Graceful Shutdown:** Upon receiving `SIGINT`, the channel sender is dropped, prompting the collector to perform a final cleanup flush of any remaining items.

### Run

Run with default configuration parameters:

```shell
cargo run
```

Or run with custom configuration parameters:

```shell
TICK_INTERVAL_MS=100 FLUSH_INTERVAL_SECS=2 CHANNEL_CAPACITY=20 FLUSH_THRESHOLD=10 cargo run
```

### Configuration

The following env vars can be set to manipulate the configurable parameters for this simulation:

| env var | default | description |
|---------|---------|-------------|
| `TICK_INTERVAL_MS` | 500 | Number of milliseconds between ticks of the main loop |
| `FLUSH_INTERVAL_SECS` | 10 | Number of seconds between automatic flushes of the background collector's buffer |
| `CHANNEL_CAPACITY` | 100 | Maximum capacity of the bounded channel used for inter-process communication |
| `FLUSH_THRESHOLD` | 50 | Number of items in the collector's buffer required to trigger a flush |
