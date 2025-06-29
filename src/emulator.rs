use crate::driver::Driver;
use crate::state::EmulatorState;
use crate::{cpu, dma, ppu, timer};
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

pub fn create_driver() -> Driver {
    let mut driver = Driver::new();
    driver.spawn(cpu::cpu());
    driver.spawn(ppu::ppu());
    driver.spawn(timer::timer());
    driver.spawn(dma::dma_component());
    driver
}

pub fn run_emulator(rom_data: Option<Vec<u8>>) {
    init_emulator();

    // Load cartridge if ROM data is provided
    if let Some(rom_bytes) = rom_data {
        match load_cartridge(rom_bytes) {
            Ok(()) => println!("Cartridge loaded successfully"),
            Err(e) => {
                eprintln!("Failed to load cartridge: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        println!("No ROM loaded - running without cartridge");
    }

    crate::ui::run_ui();
}

pub fn load_cartridge(rom_data: Vec<u8>) -> Result<(), String> {
    STATE.with(|state| state.borrow_mut().load_cartridge(rom_data))
}
