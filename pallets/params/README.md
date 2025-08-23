# Pallet Params

A simple Substrate pallet that provides runtime configuration parameters for the Neuroweb blockchain.

## Overview

The Params pallet manages global runtime configuration parameters that can affect the behavior of other pallets and runtime components.

## Features

### Testnet Mode

The pallet stores a boolean `testnet_mode` parameter that indicates whether the runtime is operating in testnet mode. This parameter influences various runtime behaviors:

- **Asset Configuration**: Controls which asset IDs are used (mainnet vs Sepolia testnet)
- **Network Behavior**: Allows different configurations for development/testing vs production
- **Future Extensions**: Provides a foundation for other environment-specific parameters

## Storage

### TestnetMode

```rust
pub type TestnetMode<T> = StorageValue<_, bool, ValueQuery>;
```

- **Type**: `bool`
- **Default**: `false` (mainnet mode)
- **Query**: Returns the current testnet mode status

## Public Interface
### Getters
- `testnet_mode() -> bool`: Returns the current testnet mode status

### Test Helpers
- `set_testnet_mode(testnet_mode: bool)`: Sets the testnet mode (available only with `std` feature for testing)

### Running Tests

```bash
cargo test -p pallet-params
```
