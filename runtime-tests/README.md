# Runtime Tests
This package contains integration tests that run against the actual Neuroweb runtime, unlike pallet-level tests that use mocks.

## Purpose
Runtime tests verify that:
- Pallets integrate correctly with the runtime
- Configuration types work as expected
- Cross-pallet interactions function properly
- Runtime-specific logic behaves correctly

## Running Tests
```bash
cargo test -p runtime-tests
```
