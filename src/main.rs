use rand::Rng;
use std::env;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{interval, Instant};

struct Config {
    tick_interval_ms: u64,    // main loop tick interval in milliseconds
    flush_interval_secs: u64, // background flush interval in seconds
    channel_capacity: usize,  // capacity of the bounded channel
    flush_threshold: usize,   // flush when the buffer reaches this size
}

impl Config {
    fn new() -> Self {
        fn get_env_or_default<T: std::str::FromStr>(key: &str, default: T) -> T {
            env::var(key)
                .ok()
                .and_then(|s| s.parse::<T>().ok())
                .unwrap_or(default)
        }

        let tick_interval_ms = get_env_or_default("TICK_INTERVAL_MS", 500);
        let flush_interval_secs = get_env_or_default("FLUSH_INTERVAL_SECS", 10);
        let channel_capacity = get_env_or_default("CHANNEL_CAPACITY", 100);
        // For flush_threshold, if not provided, default to channel_capacity / 2.
        let flush_threshold = get_env_or_default("FLUSH_THRESHOLD", channel_capacity / 2);

        Config {
            tick_interval_ms,
            flush_interval_secs,
            channel_capacity,
            flush_threshold,
        }
    }
}

#[tokio::main]
async fn main() {
    let config = Config::new();
    println!("Configuration:");
    println!(
        "  Main loop tick interval (ms): {}",
        config.tick_interval_ms
    );
    println!(
        "  Background collector flush interval (secs): {}",
        config.flush_interval_secs
    );
    println!("  Channel capacity: {}", config.channel_capacity);
    println!("  Flush threshold: {}", config.flush_threshold);
    println!("Starting main loop. Press Ctrl-C to exit gracefully...\n");

    // Create a bounded channel with capacity from the config.
    let (tx, rx) = mpsc::channel::<u32>(config.channel_capacity);

    // Spawn the background collector task, passing the flush threshold and flush interval.
    let collector_handle = tokio::spawn(background_collector(
        rx,
        config.flush_threshold,
        config.flush_interval_secs,
    ));

    // Create a ticker for generating random numbers using the configured tick interval.
    let mut ticker = interval(Duration::from_millis(config.tick_interval_ms));

    // Create a future that completes on Ctrl-C.
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            // On each tick, generate a random number and send it to the collector.
            _ = ticker.tick() => {
                let num = rand::thread_rng().gen_range(1..=100);
                for _ in 0..rand::thread_rng().gen_range(1..=10) {
                    // Send the random number; if the channel is full, await until there is room.
                    if let Err(e) = tx.send(num).await {
                        eprintln!("Error sending number {}: {}", num, e);
                    }
                }
            },
            // Listen for the SIGINT signal.
            _ = &mut ctrl_c => {
                println!("\nSIGINT received. Shutting down gracefully...");
                break;
            }
        }
    }

    // Shutdown steps:
    // 1. Drop the sender so the collector will eventually receive None.
    drop(tx);
    // 2. Wait for the collector task to finish its final flush.
    if let Err(e) = collector_handle.await {
        eprintln!("Background collector task failed: {}", e);
    }
    println!("Shutdown complete.");
}

/// The background collector receives random numbers, buffers them,
/// and flushes the buffer either on a regular interval or when the buffer reaches the threshold.
async fn background_collector(
    mut rx: mpsc::Receiver<u32>,
    flush_threshold: usize,
    flush_interval_secs: u64,
) {
    let mut buffer: Vec<u32> = Vec::new();
    let flush_interval = Duration::from_secs(flush_interval_secs);
    // A ticker that triggers a flush on a set interval.
    let mut flush_ticker = interval(flush_interval);

    loop {
        tokio::select! {
            // Flush on a regular interval.
            _ = flush_ticker.tick() => {
                if !buffer.is_empty() {
                    println!("Collector: Flushing buffer on interval.");
                    flush_buffer(&mut buffer).await;
                }
            },
            // Process new numbers from the channel.
            maybe_num = rx.recv() => {
                match maybe_num {
                    Some(num) => {
                        buffer.push(num);
                        // Flush the buffer if it reaches the configured threshold.
                        if buffer.len() >= flush_threshold {
                            println!("Collector: Flushing buffer on threshold. {} items.", buffer.len());
                            flush_buffer(&mut buffer).await;
                        }
                    },
                    // Channel closed: perform a final flush and exit.
                    None => {
                        if !buffer.is_empty() {
                            println!("Collector: Flushing buffer because channel closed and exiting process");
                            flush_buffer(&mut buffer).await;
                        }
                        break;
                    }
                }
            }
        }
    }
}

/// Simulate a flush operation by printing the buffered numbers.
async fn flush_buffer(buffer: &mut Vec<u32>) {
    println!(
        "Flushing {} items at {:?}: {:?}",
        buffer.len(),
        Instant::now(),
        buffer
    );
    buffer.clear();
}
