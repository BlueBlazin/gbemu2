# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

GBEmu2 is a cycle-accurate Game Boy (DMG) emulator written in Rust that uses a bespoke async-await state machine implementation. The project demonstrates an innovative approach to emulator architecture by leveraging Rust's async/await system for precise timing control.

## Common Commands

```bash
# Build the project
cargo build

# Run the emulator
cargo run

# Run with release optimizations
cargo run --release

# Check code without building
cargo check

# Run tests (when available)
cargo test
```

## Architecture

### Core Design Philosophy
- **Function-based approach**: Uses async functions rather than struct implementations
- **Centralized state**: All emulator state (CPU registers, RAM, ROM) is managed through a shared state struct
- **Async timing**: Components use `sleep(cycles).await` for cycle-accurate timing

### Key Components

#### Driver System (`src/driver.rs`)
The heart of the emulator's timing system:
- `Driver` struct manages a futures queue organized by clock cycles
- `Sleep` future provides cycle-accurate delays
- Thread-local storage tracks current cycle and wake times
- Components are spawned as async tasks and scheduled based on their timing requirements

#### State Management (`src/state.rs`)
Currently empty but intended to hold the central state structure that all components will read from and write to.

#### CPU Implementation (`src/cpu/mod.rs`)
Placeholder for the CPU emulation logic that will use async functions to implement instruction execution with proper timing.

### Async Pattern Usage
Components follow this pattern:
```rust
async fn component_name() {
    sleep(cycles).await;  // Wait for specific number of cycles
    // Component logic here
}
```

The driver automatically handles scheduling and ensures components wake up at the correct cycle count.

## Development Notes

- The project uses Rust edition 2024
- Only dependency is `futures = "0.3.31"`
- Thread-local storage is used for cycle tracking to avoid passing state between async contexts
- The BTreeMap in the driver ensures futures are processed in chronological order