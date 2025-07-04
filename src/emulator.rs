use crate::driver::Driver;
use crate::state::EmulatorState;
use crate::{cpu, dma, joypad, ppu, timer};
use std::cell::RefCell;

thread_local! {
    static STATE: RefCell<EmulatorState> = RefCell::new(EmulatorState::new());
}

pub fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&EmulatorState) -> R,
{
    STATE.with(|state| f(&state.borrow()))
}

pub fn with_state_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut EmulatorState) -> R,
{
    STATE.with(|state| f(&mut state.borrow_mut()))
}

pub fn init_emulator() {
    STATE.with(|state| {
        *state.borrow_mut() = EmulatorState::new();
    });
}

pub fn init_emulator_with_debug(debug_mode: bool) {
    STATE.with(|state| {
        let mut new_state = EmulatorState::new();
        new_state.debug_mode = debug_mode;
        *state.borrow_mut() = new_state;
    });
}

pub fn create_driver() -> Driver {
    let mut driver = Driver::new();
    driver.spawn(cpu::cpu());
    driver.spawn(ppu::ppu());
    driver.spawn(timer::timer());
    driver.spawn(dma::dma_component());
    driver.spawn(joypad::joypad_handler());
    driver
}

pub fn load_cartridge(rom_data: Vec<u8>) -> Result<(), String> {
    STATE.with(|state| state.borrow_mut().load_cartridge(rom_data))
}
