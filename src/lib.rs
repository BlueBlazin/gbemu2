pub mod cartridge;
mod cpu;
mod dma;
pub mod driver;
pub mod emulator;
mod interrupts;
mod io;
mod mmu;
mod ppu;
mod state;
mod timer;

// New modules for cleaner organization
pub mod emulation_runner;
pub mod renderer;
pub mod rom_loader;
pub mod texture_pool;
