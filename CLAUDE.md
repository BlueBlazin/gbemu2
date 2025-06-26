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

#### State Management (`src/state.rs`, `src/emulator.rs`)
- `EmulatorState` contains CPU registers and 64KB memory array
- `CpuState` has all Game Boy registers with 8/16-bit access methods and flag operations
- `emulator.rs` provides `with_state()` and `with_state_mut()` for shared state access across components

#### CPU Implementation (`src/cpu/mod.rs`)
- Implements fetch-decode-execute cycle with proper Game Boy timing
- **Instruction Sets Implemented:**
  - Complete 8-bit load instructions (LD r,n; LD r,(HL); LD (HL),r; LDH; etc.)
  - Complete 16-bit load instructions (LD rr,nn; PUSH/POP; LD SP,HL; etc.)
  - Complete 8-bit arithmetic (ADD/ADC/SUB/SBC with register/immediate/(HL) variants)
- **Helper Functions:** `add_8bit()` and `sub_8bit()` handle flag calculations (Z,N,H,C)
- Memory operations: `memory_read()` and `memory_write()` take 4 cycles each
- All instructions follow official Game Boy timing specifications

### Async Pattern Usage
Components follow this pattern:
```rust
async fn component_name() {
    sleep(cycles).await;  // Wait for specific number of cycles
    // Component logic here
}
```

Memory access pattern:
```rust
async fn memory_read(address: u16) -> u8 {
    sleep(4).await; // 4 cycles for memory access
    with_state(|state| state.memory[address as usize])
}
```

The driver automatically handles scheduling and ensures components wake up at the correct cycle count.

## Development Notes

- The project uses Rust edition 2024
- Only dependency is `futures = "0.3.31"`
- Thread-local storage is used for cycle tracking to avoid passing state between async contexts
- The BTreeMap in the driver ensures futures are processed in chronological order

## Current Status & Next Steps

**Completed:** 8-bit/16-bit loads, 8-bit arithmetic (ADD/ADC/SUB/SBC), stack operations, flag handling
**Next:** Logical instructions (AND/OR/XOR/CP), bit operations, jumps/calls, interrupts
**Pattern:** Register ops use bits 2-0, immediate ops have dedicated opcodes, (HL) variants embedded in register patterns at bit pattern 110