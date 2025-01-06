# Exchange Module

This module provides a unified interface for interacting with different cryptocurrency exchanges through a common trait-based abstraction.

## Structure

- `exchange.rs` - Contains the core `Exchange` trait definition and common types
- `ex_binance.rs` - Binance exchange implementation
- `ex_bybit.rs` - Bybit exchange implementation

## Key Features

### Exchange Trait

The `Exchange` trait defines a standard interface for common exchange operations:

- Market Data Operations
  - Time synchronization
  - Fee information retrieval
  - Symbol information
  - Market data streaming
  - Order book management

- Trading Operations
  - Leverage settings
  - Order placement/amendment/cancellation
  - Batch order operations
  - Private data streaming

### Market Data Types

- `MarketData` enum for unified market data handling from different exchanges
- `TradeType` enum for exchange-specific trade data formats
- Standardized order book implementation via the `OrderBook` trait

## Usage

To use an exchange:

1. Initialize with API credentials:

```rust
let exchange = ExchangeClient::init(api_key, api_secret);
```

2. Access market data:

```rust
let market_data = exchange.market_subscribe(symbols, sender).await;
```

3. Place orders:

```rust
let order = exchange.place_order(symbol, price, qty, is_buy).await;
```

4. Manage positions:

```rust
let leverage = exchange.set_leverage(symbol, leverage).await;
```

## Supported Exchanges

- Binance Futures
- Bybit Perpetual

## Implementation Details

- Asynchronous operations using Tokio
- Error handling with exchange-specific error types
- Websocket streaming for real-time data
- Order book management with price/time priority
- Support for order batching and amendments

The module enables building trading systems with unified exchange access while maintaining exchange-specific optimizations.
