use crate::cpu;
use crate::dma;
use crate::driver::Driver;
use crate::ppu;
use crate::state::EmulatorState;
use crate::timer;
use std::cell::RefCell;

thread_local! {
    static STATE: RefCell<EmulatorState> = RefCell::new(EmulatorState::new());
}

pub fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&EmulatorState) -> R,
{
    STATE.with(|state| f(&*state.borrow()))
}

pub fn with_state_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut EmulatorState) -> R,
{
    STATE.with(|state| f(&mut *state.borrow_mut()))
}

pub fn init_emulator() {
    STATE.with(|state| {
        *state.borrow_mut() = EmulatorState::new();
    });
}

pub fn run_emulator() {
    init_emulator();

    let mut driver = Driver::new();

    // Spawn CPU component
    driver.spawn(cpu::cpu());

    // Spawn PPU component
    driver.spawn(ppu::ppu());

    // Spawn Timer component
    driver.spawn(timer::timer());

    // Spawn DMA component
    driver.spawn(dma::dma_component());

    // TODO: Add other components (APU, etc.)

    driver.run();
}
