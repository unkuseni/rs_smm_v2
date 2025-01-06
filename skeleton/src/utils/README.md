# Utils Module

A collection of utility modules providing common functionality for the trading system.

## Modules

### Bot (`bot.rs`)
Telegram bot integration for notifications and alerts:
- Singleton bot instance management
- Asynchronous message sending
- Error handling for bot operations

### Config (`config.rs`)
Configuration management utilities:
- TOML file reading and parsing
- Real-time config file watching
- Configuration update notifications

### EMA (`ema.rs`)
Exponential Moving Average implementation:
- Configurable window sizes
- Custom alpha values
- Historical value tracking
- Initialization state management

### Local Order Book (`localorderbook.rs`)
Order book management trait defining common operations:
- Order book updates and maintenance
- Price level tracking
- Market metrics calculation
- Order book analysis functions

### Logger (`logger.rs`)
Structured logging system:
- Multiple log levels (Success, Info, Debug, Warning, Error, Critical)
- Timestamp formatting
- Formatted message output

### Models (`models.rs`)
Core data structures:
- Exchange-specific client models
- Order book implementations
- Market data structures
- Order management types

### Number (`number.rs`)
Mathematical utility functions:
- Square root calculation
- Exponential decay
- Geometric weights
- Linear and geometric spacing
- Number rounding and formatting

### Time (`time.rs`)
Time-related utilities:
- Timestamp generation
- Date formatting
- Time formatting
- Calendar calculations

## Usage

```rust
// Bot usage
let bot = LiveBot::new()?;
bot.send_message("Trade executed").await?;

// Config usage
let config: Config = read_toml("config.toml")?;
watch_config("config.toml", sender).await?;

// EMA usage
let mut ema = EMA::new(14);
let value = ema.update(price);

// Logger usage
Logger::log(LogLevel::Info, "Starting trading session");

// Number utilities
let weights = geometric_weights(0.95, 10, false);
let rounded = 10.523_f64.round_to(2);

// Time utilities
let timestamp = generate_timestamp()?;
let (hours, mins, secs, is_pm) = get_formatted_time();
```

## Features

- Thread-safe implementations
- Async/await support
- Error handling with custom error types
- Real-time data processing
- Memory-efficient data structures
- Performance-optimized calculations

## Dependencies

- `tokio` for async runtime
- `serde` for serialization
- `toml` for configuration
- `teloxide` for Telegram integration
- `ordered-float` for ordered floating point operations
- `num-traits` for generic number operations

## Testing

Each module includes unit tests and integration tests covering core functionality.
Run tests using:

```bash
cargo test --package rs_smm_v2 --lib utils
```

## Documentation

Detailed documentation is available for each module. Generate documentation using:

```bash
cargo doc --no-deps --open
```
