use crate::driver::sleep;
use crate::emulator::{with_state, with_state_mut};
use crate::interrupts::InterruptType;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU8, Ordering};

/// Joypad button states
#[derive(Debug, Clone, Copy)]
pub struct JoypadState {
    // D-pad buttons
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    // Action buttons
    pub a: bool,
    pub b: bool,
    pub start: bool,
    pub select: bool,
    // Previous P1 value for edge detection
    previous_p1: u8,
}

impl JoypadState {
    pub fn new() -> Self {
        Self {
            up: false,
            down: false,
            left: false,
            right: false,
            a: false,
            b: false,
            start: false,
            select: false,
            previous_p1: 0xFF,
        }
    }

    /// Calculate P1 register value based on current button states and selection
    pub fn calculate_p1(&self, p1_select: u8) -> u8 {
        let mut result = 0xCF; // Bits 6-7 always 1, bits 4-5 from selection

        // Keep the selection bits (4-5) from p1_select
        result = (result & 0xCF) | (p1_select & 0x30);

        let select_dpad = (p1_select & 0x10) == 0; // Bit 4 low = select d-pad
        let select_buttons = (p1_select & 0x20) == 0; // Bit 5 low = select buttons

        if !select_dpad && !select_buttons {
            // Neither selected, lower nibble reads as 0xF
            result |= 0x0F;
        } else {
            // Calculate lower nibble based on selected mode
            let mut lower_nibble = 0x0F;

            if select_dpad {
                if self.down {
                    lower_nibble &= !0x08;
                } // Bit 3
                if self.up {
                    lower_nibble &= !0x04;
                } // Bit 2
                if self.left {
                    lower_nibble &= !0x02;
                } // Bit 1
                if self.right {
                    lower_nibble &= !0x01;
                } // Bit 0
            }

            if select_buttons {
                if self.start {
                    lower_nibble &= !0x08;
                } // Bit 3
                if self.select {
                    lower_nibble &= !0x04;
                } // Bit 2
                if self.b {
                    lower_nibble &= !0x02;
                } // Bit 1
                if self.a {
                    lower_nibble &= !0x01;
                } // Bit 0
            }

            result = (result & 0xF0) | lower_nibble;
        }

        result
    }

    /// Check for joypad interrupt (High to Low transition on any button line)
    pub fn check_interrupt(&mut self, new_p1: u8) -> bool {
        // Check if any bit went from 1 to 0 in the lower nibble
        let previous_lower = self.previous_p1 & 0x0F;
        let new_lower = new_p1 & 0x0F;
        let high_to_low = previous_lower & !new_lower;

        self.previous_p1 = new_p1;

        high_to_low != 0
    }

    /// Set button state from key press/release
    pub fn set_button(&mut self, button: JoypadButton, pressed: bool) {
        match button {
            JoypadButton::Up => self.up = pressed,
            JoypadButton::Down => self.down = pressed,
            JoypadButton::Left => self.left = pressed,
            JoypadButton::Right => self.right = pressed,
            JoypadButton::A => self.a = pressed,
            JoypadButton::B => self.b = pressed,
            JoypadButton::Start => self.start = pressed,
            JoypadButton::Select => self.select = pressed,
        }
    }
}

/// Joypad button enumeration for external input mapping
#[derive(Debug, Clone, Copy)]
pub enum JoypadButton {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Start,
    Select,
}

// Global joypad state accessible from main thread
// Using atomic u8 for each button to avoid locks
static BUTTON_STATES: [AtomicU8; 8] = [
    AtomicU8::new(0), // Up
    AtomicU8::new(0), // Down
    AtomicU8::new(0), // Left
    AtomicU8::new(0), // Right
    AtomicU8::new(0), // A
    AtomicU8::new(0), // B
    AtomicU8::new(0), // Start
    AtomicU8::new(0), // Select
];

thread_local! {
    static JOYPAD_STATE: RefCell<JoypadState> = RefCell::new(JoypadState::new());
}

/// Async joypad handler that polls for state changes
pub async fn joypad_handler() {
    loop {
        // Poll every 256 T-cycles (64 M-cycles) for responsiveness
        sleep(256).await;

        // Update local joypad state from global atomic states
        JOYPAD_STATE.with(|state| {
            let mut joypad = state.borrow_mut();
            joypad.up = BUTTON_STATES[0].load(Ordering::Relaxed) != 0;
            joypad.down = BUTTON_STATES[1].load(Ordering::Relaxed) != 0;
            joypad.left = BUTTON_STATES[2].load(Ordering::Relaxed) != 0;
            joypad.right = BUTTON_STATES[3].load(Ordering::Relaxed) != 0;
            joypad.a = BUTTON_STATES[4].load(Ordering::Relaxed) != 0;
            joypad.b = BUTTON_STATES[5].load(Ordering::Relaxed) != 0;
            joypad.start = BUTTON_STATES[6].load(Ordering::Relaxed) != 0;
            joypad.select = BUTTON_STATES[7].load(Ordering::Relaxed) != 0;

            // Read current P1 register value
            let p1_select = with_state(|state| state.read_memory(0xFF00));

            // Calculate new P1 value based on current button states
            let new_p1 = joypad.calculate_p1(p1_select);

            // Check for interrupt condition
            if joypad.check_interrupt(new_p1) {
                // Request joypad interrupt
                with_state_mut(|state| {
                    state.interrupts.request_interrupt(InterruptType::Joypad);
                });
            }

            // Update P1 register with new value
            with_state_mut(|state| {
                // Update the full register value (lower nibble with button states)
                state.mmu.io_registers[0x00] = new_p1;
            });
        });
    }
}

/// Helper function to update joypad state from external input
/// This would be called from the main loop when processing SDL events
pub fn update_joypad_button(button: JoypadButton, pressed: bool) {
    let index = match button {
        JoypadButton::Up => 0,
        JoypadButton::Down => 1,
        JoypadButton::Left => 2,
        JoypadButton::Right => 3,
        JoypadButton::A => 4,
        JoypadButton::B => 5,
        JoypadButton::Start => 6,
        JoypadButton::Select => 7,
    };

    BUTTON_STATES[index].store(if pressed { 1 } else { 0 }, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p1_calculation_no_selection() {
        let joypad = JoypadState::new();
        // Neither d-pad nor buttons selected
        let p1 = joypad.calculate_p1(0xFF);
        assert_eq!(p1, 0xFF); // All bits high
    }

    #[test]
    fn test_p1_calculation_dpad_selected() {
        let mut joypad = JoypadState::new();
        joypad.down = true;
        joypad.left = true;

        // Select d-pad (bit 4 = 0)
        let p1 = joypad.calculate_p1(0xEF);
        assert_eq!(p1 & 0x0F, 0x05); // Bits 3 and 1 should be low
    }

    #[test]
    fn test_p1_calculation_buttons_selected() {
        let mut joypad = JoypadState::new();
        joypad.a = true;
        joypad.start = true;

        // Select buttons (bit 5 = 0)
        let p1 = joypad.calculate_p1(0xDF);
        assert_eq!(p1 & 0x0F, 0x06); // Bits 3 and 0 should be low
    }

    #[test]
    fn test_interrupt_detection() {
        let mut joypad = JoypadState::new();
        joypad.previous_p1 = 0xFF;

        // Simulate button press (bit goes from 1 to 0)
        let interrupt = joypad.check_interrupt(0xFE);
        assert!(interrupt);

        // No change should not trigger interrupt
        let interrupt = joypad.check_interrupt(0xFE);
        assert!(!interrupt);
    }
}
