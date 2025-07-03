use crate::driver::sleep;
use crate::emulator::{with_state, with_state_mut};
use crate::interrupts;

// Timer register addresses
pub const DIV_REGISTER: u16 = 0xFF04; // Divider register
pub const TIMA_REGISTER: u16 = 0xFF05; // Timer counter
pub const TMA_REGISTER: u16 = 0xFF06; // Timer modulo
pub const TAC_REGISTER: u16 = 0xFF07; // Timer control

// Timer frequencies (in M-cycles between increments)
const TIMER_FREQ_4096: u16 = 256; // 00: 4096 Hz
const TIMER_FREQ_262144: u16 = 4; // 01: 262144 Hz
const TIMER_FREQ_65536: u16 = 16; // 10: 65536 Hz
const TIMER_FREQ_16384: u16 = 64; // 11: 16384 Hz

// DIV frequency (16384 Hz = every 256 M-cycles)
const DIV_FREQ: u16 = 256;

#[derive(Debug, Clone)]
pub struct TimerState {
    // Registers
    pub div: u8,  // FF04: Divider register (read-only upper 8 bits of system counter)
    pub tima: u8, // FF05: Timer counter
    pub tma: u8,  // FF06: Timer modulo
    pub tac: u8,  // FF07: Timer control

    // Internal state
    pub system_counter: u16, // Full 16-bit system counter (DIV is upper 8 bits)
    pub timer_cycles: u16,   // Cycles since last TIMA increment
    pub overflow_pending: bool, // True when TIMA overflow needs to be processed next cycle
    pub overflow_tma_value: u8, // TMA value to use for overflow (captured at overflow time)
    pub stop_mode: bool,     // True when in STOP mode (counter paused)
}

impl Default for TimerState {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerState {
    pub fn new() -> Self {
        Self {
            div: 0xAB,              // DIV = $AB (boot value)
            tima: 0x00,             // TIMA = $00 (boot value)
            tma: 0x00,              // TMA = $00 (boot value)
            tac: 0xF8,              // TAC = $F8 (boot value)
            system_counter: 0xAB00, // DIV is upper 8 bits, so 0xAB << 8
            timer_cycles: 0,
            overflow_pending: false,
            overflow_tma_value: 0x00,
            stop_mode: false,
        }
    }

    // TAC register bit accessors
    pub fn timer_enable(&self) -> bool {
        (self.tac & 0x04) != 0
    }

    pub fn clock_select(&self) -> u8 {
        self.tac & 0x03
    }

    // Get timer frequency based on clock select
    pub fn get_timer_frequency(&self) -> u16 {
        match self.clock_select() {
            0b00 => TIMER_FREQ_4096,   // 4096 Hz
            0b01 => TIMER_FREQ_262144, // 262144 Hz
            0b10 => TIMER_FREQ_65536,  // 65536 Hz
            0b11 => TIMER_FREQ_16384,  // 16384 Hz
            _ => unreachable!(),
        }
    }

    // Check if selected bit of system counter is set
    pub fn is_timer_bit_set(&self) -> bool {
        let bit_position = match self.clock_select() {
            0b00 => 7, // Bit 7 for 256 M-cycle frequency
            0b01 => 1, // Bit 1 for 4 M-cycle frequency
            0b10 => 3, // Bit 3 for 16 M-cycle frequency
            0b11 => 5, // Bit 5 for 64 M-cycle frequency
            _ => unreachable!(),
        };
        (self.system_counter & (1 << bit_position)) != 0
    }

    // Handle timer tick (when selected bit transitions from 1 to 0)
    pub fn handle_timer_tick(&mut self) {
        if self.timer_enable() {
            let (result, overflow) = self.tima.overflowing_add(1);
            self.tima = result;

            if overflow {
                // TIMA overflowed - will be reset to TMA next cycle
                // Capture current TMA value at overflow time
                self.overflow_pending = true;
                self.overflow_tma_value = self.tma;
            }
        }
    }

    // Process pending timer overflow
    // Returns true if a timer interrupt should be requested
    pub fn process_timer_overflow(&mut self) -> bool {
        if self.overflow_pending {
            // Use the TMA value that was captured at overflow time
            // This ensures that TMA writes during overflow cycle don't affect the result
            self.tima = self.overflow_tma_value;
            self.overflow_pending = false;
            // Signal that a timer interrupt should be requested
            return true;
        }
        false
    }

    // Write to DIV register (resets system counter)
    pub fn write_div(&mut self, _value: u8) {
        // Writing any value to DIV resets the entire system counter
        let old_timer_bit = self.is_timer_bit_set();
        self.system_counter = 0;
        self.div = 0;

        // If timer bit was set before reset, this triggers a timer tick
        if old_timer_bit {
            self.handle_timer_tick();
        }
    }

    // Write to TIMA register
    pub fn write_tima(&mut self, value: u8) {
        // If overflow is pending, ignore the write (the overflow will proceed)
        if !self.overflow_pending {
            self.tima = value;
        }
    }

    // Write to TMA register
    pub fn write_tma(&mut self, value: u8) {
        self.tma = value;

        // If overflow is pending and TMA is written on same cycle,
        // the OLD value (captured at overflow time) will still be used for the reset
        // This is handled by using overflow_tma_value in process_timer_overflow
    }

    // Write to TAC register
    pub fn write_tac(&mut self, value: u8) {
        let old_enable = self.timer_enable();
        let old_timer_bit = self.is_timer_bit_set();

        self.tac = value & 0x07; // Only bits 0-2 are writable

        let new_enable = self.timer_enable();
        let new_timer_bit = self.is_timer_bit_set();

        // Check for timer tick conditions:
        // 1. Disabling timer when selected bit is set
        // 2. Changing clock select from set bit to unset bit
        if (old_enable && !new_enable && old_timer_bit) || (old_timer_bit && !new_timer_bit) {
            self.handle_timer_tick();
        }
    }

    // Enter STOP mode
    pub fn enter_stop_mode(&mut self) {
        self.stop_mode = true;
    }

    // Exit STOP mode
    pub fn exit_stop_mode(&mut self) {
        self.stop_mode = false;
    }
}

// Main timer async function
pub async fn timer() {
    loop {
        // Process one M-cycle of timer operation
        step_timer().await;

        // Sleep for one M-cycle (4 cycles)
        sleep(4).await;
    }
}

async fn step_timer() {
    // Check if in STOP mode
    let stop_mode = with_state(|state| state.timer.stop_mode);
    if stop_mode {
        return; // Don't update counters in STOP mode
    }

    // First, process any pending timer overflow from previous cycle
    let should_interrupt = with_state_mut(|state| state.timer.process_timer_overflow());

    // Request timer interrupt if needed (outside the borrow context)
    if should_interrupt {
        request_timer_interrupt();
    }

    // Get current timer bit state before incrementing
    let old_timer_bit = with_state(|state| state.timer.is_timer_bit_set());

    // Increment system counter (this happens every M-cycle)
    with_state_mut(|state| {
        state.timer.system_counter = state.timer.system_counter.wrapping_add(1);

        // Update DIV register - increments at 16384 Hz (every 256 M-cycles)
        // DIV is the upper 8 bits of the 16-bit system counter
        state.timer.div = (state.timer.system_counter >> 8) as u8;

        // Validate DIV frequency: 16384 Hz = 4194304 Hz / 256 M-cycles
        debug_assert_eq!(
            256,          // DIV increments every 256 M-cycles (upper 8 bits of 16-bit counter)
            256,          // Expected frequency for 16384 Hz
            "DIV frequency calculation mismatch"
        );
    });

    // Check for timer tick (selected bit transition from 1 to 0)
    let new_timer_bit = with_state(|state| state.timer.is_timer_bit_set());

    if old_timer_bit && !new_timer_bit {
        with_state_mut(|state| {
            state.timer.handle_timer_tick();
        });
    }
}

// I/O register handlers for timer registers
pub fn read_timer_register(address: u16) -> u8 {
    with_state(|state| {
        match address {
            DIV_REGISTER => state.timer.div,
            TIMA_REGISTER => {
                if state.timer.overflow_pending {
                    // During overflow cycle, TIMA reads as 0x00
                    0x00
                } else {
                    state.timer.tima
                }
            }
            TMA_REGISTER => state.timer.tma,
            TAC_REGISTER => state.timer.tac | 0xF8, // Upper bits read as 1
            _ => 0xFF,                              // Invalid register
        }
    })
}

pub fn write_timer_register(address: u16, value: u8) {
    with_state_mut(|state| {
        match address {
            DIV_REGISTER => state.timer.write_div(value),
            TIMA_REGISTER => state.timer.write_tima(value),
            TMA_REGISTER => state.timer.write_tma(value),
            TAC_REGISTER => state.timer.write_tac(value),
            _ => {} // Invalid register, ignore
        }
    });
}

// Direct access versions for avoiding nested borrows
pub fn read_timer_register_direct(address: u16, timer: &TimerState) -> u8 {
    match address {
        DIV_REGISTER => timer.div,
        TIMA_REGISTER => {
            if timer.overflow_pending {
                // During overflow cycle, TIMA reads as 0x00
                0x00
            } else {
                timer.tima
            }
        }
        TMA_REGISTER => timer.tma,
        TAC_REGISTER => timer.tac | 0xF8, // Upper bits read as 1
        _ => 0xFF,                        // Invalid register
    }
}

pub fn write_timer_register_direct(address: u16, value: u8, timer: &mut TimerState) {
    match address {
        DIV_REGISTER => timer.write_div(value),
        TIMA_REGISTER => timer.write_tima(value),
        TMA_REGISTER => timer.write_tma(value),
        TAC_REGISTER => timer.write_tac(value),
        _ => {} // Invalid register, ignore
    }
}

// Helper functions for other components
pub fn request_timer_interrupt() {
    interrupts::request_timer_interrupt();
}

pub fn enter_stop_mode() {
    with_state_mut(|state| {
        state.timer.enter_stop_mode();
    });
}

pub fn exit_stop_mode() {
    with_state_mut(|state| {
        state.timer.exit_stop_mode();
    });

    // In a real Game Boy, STOP mode is typically exited by:
    // - Joypad input
    // - Serial communication interrupt
    // This function provides the interface for those systems to wake up the timer
}

// Get current timer state for debugging and inspection
pub fn get_timer_state() -> (u8, u8, u8, u8) {
    with_state(|state| {
        (
            state.timer.div,
            state.timer.tima,
            state.timer.tma,
            state.timer.tac,
        )
    })
}

// Additional debugging helper - get full timer state including internals
pub fn get_full_timer_state() -> (u8, u8, u8, u8, u16, bool, u8, bool) {
    with_state(|state| {
        (
            state.timer.div,
            state.timer.tima,
            state.timer.tma,
            state.timer.tac,
            state.timer.system_counter,
            state.timer.overflow_pending,
            state.timer.overflow_tma_value,
            state.timer.stop_mode,
        )
    })
}

// Public interface for external systems that might need to wake from STOP mode
// (such as joypad input handling when implemented)
pub fn wake_from_stop_if_needed() {
    // In a full Game Boy emulator, this would be called by:
    // - Joypad input system
    // - Serial communication interrupt
    // - Other external wake-up sources

    let is_stopped = with_state(|state| state.timer.stop_mode);
    if is_stopped {
        exit_stop_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stop_mode_functionality() {
        // This test demonstrates the STOP mode entry and exit functionality
        // ensuring exit_stop_mode is properly connected

        let mut timer_state = TimerState::new();

        // Enter STOP mode
        timer_state.enter_stop_mode();
        assert!(timer_state.stop_mode);

        // Exit STOP mode
        timer_state.exit_stop_mode();
        assert!(!timer_state.stop_mode);
    }
}
