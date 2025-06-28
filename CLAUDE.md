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

**Event System:**
- `Event` enum for host system events: `VBlank`, `AudioBufferFull`, `MaxCycles`
- `emit_event()` function for components to signal events
- `run_for(cycles)` method returns `ExecutionResult` with event and cycles executed
- **Important:** Events are for host system (display, audio), separate from emulated interrupts

**Usage Patterns:**
```rust
// Unlimited execution (current usage)
driver.run();

// Controlled execution for precise timing
let result = driver.run_for(max_cycles);
match result.event {
    Event::VBlank => display_framebuffer(),      // Host handles graphics
    Event::AudioBufferFull => drain_audio(),    // Host handles audio  
    Event::MaxCycles => continue,                // All cycles consumed
}
```

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

#### PPU Implementation (`src/ppu.rs`)
- **Complete cycle-accurate implementation** with proper Game Boy timing
- **Pixel FIFO system:** Background and Object FIFOs with 8-pixel fetching
- **Mode timing:** OAM Scan (80 dots), Drawing (172-289 dots), HBlank, VBlank
- **Object support:** Full sprite handling including 8x16 mode, flipping, priority
- **Window rendering:** Proper window triggers, penalties, and line counter
- **Timing penalties:** SCX scrolling, window rendering (6 dots), object penalties (6-11 dots)
- **Memory access restrictions:** VRAM/OAM access blocked during appropriate modes
- **Event emission:** Emits `VBlank` events for host framebuffer display

#### DMA Implementation (`src/dma.rs`)
- **OAM DMA transfer:** Cycle-accurate 160-byte transfer over 160 M-cycles
- **Bus conflict simulation:** CPU restricted to HRAM during DMA transfer
- **FF46 register:** Proper DMA trigger and source address handling
- **Timing accuracy:** 2 M-cycle delay before transfer starts
- **Independent operation:** Continues during CPU halt/stop states

#### Timer Implementation (`src/timer.rs`)
- **Cycle-accurate timing** with proper DIV and TIMA behavior
- **Interrupt generation:** Timer overflow triggers interrupt requests
- **Register handling:** DIV (FF04), TIMA (FF05), TMA (FF06), TAC (FF07)

#### Memory Management (`src/mmu.rs`, `src/io.rs`)
- **Complete memory map:** ROM, RAM, VRAM, OAM, I/O, HRAM
- **DMA bus restrictions:** Enforces CPU memory access limitations during DMA
- **I/O register system:** Centralized handling of memory-mapped registers
- **Cartridge integration:** ROM/MBC support through cartridge abstraction

#### Interrupt System (`src/interrupts.rs`)
- **Complete interrupt handling:** IF/IE registers, interrupt priorities
- **Interrupt types:** VBlank, LCD, Timer, Serial, Joypad
- **CPU integration:** Proper interrupt servicing and handler addresses

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

**CRITICAL TIMING NOTE:**
- **T-cycle = 1 clock cycle** (the fundamental timing unit)
- **M-cycle = 4 T-cycles** (machine cycle, used for Game Boy timing specs)
- **Always use `sleep(4)` for 1 M-cycle operations**
- Game Boy documentation uses M-cycles, but our sleep() function takes T-cycles
- Examples: Memory access = 1 M-cycle = `sleep(4)`, Instruction timing in M-cycles must be multiplied by 4

The driver automatically handles scheduling and ensures components wake up at the correct cycle count.

## Development Notes

- The project uses Rust edition 2024
- Only dependency is `futures = "0.3.31"`
- Thread-local storage is used for cycle tracking to avoid passing state between async contexts
- The BTreeMap in the driver ensures futures are processed in chronological order

## Current Status & Next Steps

### Completed Components
- ✅ **Driver System:** Event-driven execution with `run_for()` and controlled timing
- ✅ **PPU:** Complete cycle-accurate implementation with FIFO, objects, window, timing penalties
- ✅ **DMA:** OAM DMA transfer with bus conflicts and authentic Game Boy behavior
- ✅ **Timer:** Cycle-accurate DIV/TIMA with interrupt generation
- ✅ **MMU/I/O:** Complete memory map with DMA restrictions and I/O handling
- ✅ **Interrupts:** Full interrupt system with IF/IE registers
- ✅ **CPU Foundations:** 8-bit/16-bit loads, 8-bit arithmetic, stack operations, flag handling

### Major TODOs
1. **CPU Instruction Set Completion:**
   - Logical instructions (AND/OR/XOR/CP)
   - Bit operations (SET/RES/BIT, rotates/shifts)
   - Flow control (JP/JR/CALL/RET with conditions)
   - Special instructions (DAA/CPL/SCF/CCF, HALT/STOP, EI/DI)

2. **Main Emulation Loop:**
   - Frame rate limiting and timing control
   - Input handling (joypad)
   - Host system integration (graphics/audio output)
   - ROM loading and cartridge selection

3. **Additional Features:**
   - Audio Processing Unit (APU) implementation
   - Advanced MBC support (MBC1, MBC3, MBC5)
   - Save state functionality
   - Debugging tools and logging

4. **Host Integration:**
   - Graphics backend for framebuffer display
   - Audio backend for sound output
   - Input system for joypad controls
   - File I/O for ROM loading and saves

### Implementation Notes
- **Pattern:** Register ops use bits 2-0, immediate ops have dedicated opcodes, (HL) variants embedded in register patterns at bit pattern 110
- **Event vs Interrupt:** Events are for host system (VBlank → display), interrupts are for emulated CPU (VBlank interrupt → 0x40)
- **Timing Critical:** Always use T-cycles with sleep(), remember M-cycle = 4 T-cycles