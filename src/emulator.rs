use crate::driver::{Driver, Event, ExecutionResult};
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

pub fn create_driver() -> Driver {
    let mut driver = Driver::new();
    driver.spawn(cpu::cpu());
    driver.spawn(ppu::ppu());
    driver.spawn(timer::timer());
    driver.spawn(dma::dma_component());
    driver
}

pub fn run_emulator_step(driver: &mut Driver) -> ExecutionResult {
    // Run emulator for approximately one frame's worth of cycles
    // Game Boy runs at ~4.194304 MHz, 60 FPS = ~69905 cycles per frame
    let cycles_per_frame = 69905;

    loop {
        let result = driver.run_for(cycles_per_frame);

        match result.event {
            Event::VBlank => {
                // VBlank occurred - frame complete
                return result;
            }
            Event::AudioBufferFull => {
                // TODO: Handle audio when APU is implemented
                continue;
            }
            Event::MaxCycles => {
                // Ran out of cycles without VBlank - return result and continue next time
                return result;
            }
        }
    }
}

pub fn run_emulator() {
    init_emulator();
    crate::ui::run_ui();
}
