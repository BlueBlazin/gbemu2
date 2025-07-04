use crate::interrupts::InterruptType;

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

#[derive(Debug, Clone)]
pub struct JoypadState {
    // Direction buttons
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,

    // Action buttons
    pub a: bool,
    pub b: bool,
    pub start: bool,
    pub select: bool,

    // Previous P1 value for interrupt detection
    pub previous_p1_lower: u8,
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
            previous_p1_lower: 0x0F, // All buttons released initially
        }
    }

    /// Calculate P1 register value based on selection bits and button states
    pub fn calculate_p1(&self, p1_select: u8) -> u8 {
        let mut result = 0xC0; // Bits 6-7 always 1

        // Keep the selection bits (4-5) from input
        result |= p1_select & 0x30;

        let select_dpad = (p1_select & 0x10) == 0; // Bit 4 low = select d-pad
        let select_buttons = (p1_select & 0x20) == 0; // Bit 5 low = select buttons

        let mut lower_nibble = 0x0F; // Start with all buttons released (bits high)

        if select_dpad {
            // D-pad selected - map direction keys to lower nibble
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
            // Action buttons selected - map action keys to lower nibble
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

        result | lower_nibble
    }

    /// Set a button state
    pub fn set_button(&mut self, button: JoypadButton, pressed: bool) {
        // Update button state
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

    /// Check for interrupt condition and trigger if needed
    /// This should be called from the main state update, not from within set_button
    pub fn check_and_trigger_interrupt(&mut self, p1_select: u8, interrupt_state: &mut crate::interrupts::InterruptState) -> bool {
        // Calculate new P1 value
        let new_p1 = self.calculate_p1(p1_select);
        let new_lower = new_p1 & 0x0F;

        // Check if any bit went from 1 to 0 (button pressed)
        let high_to_low = self.previous_p1_lower & !new_lower;

        let should_interrupt = high_to_low != 0;
        if should_interrupt {
            // Request joypad interrupt
            interrupt_state.request_interrupt(InterruptType::Joypad);
        }

        self.previous_p1_lower = new_lower;
        should_interrupt
    }
}
