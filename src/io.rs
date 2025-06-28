use crate::emulator::{with_state, with_state_mut};
use crate::timer;

// I/O register addresses
pub const IF_REGISTER: u16 = 0xFF0F; // Interrupt Flag
pub const IE_REGISTER: u16 = 0xFFFF; // Interrupt Enable

// Read from I/O register
pub fn read_io_register(address: u16) -> u8 {
    match address {
        IF_REGISTER => {
            // Read Interrupt Flag register from interrupt state
            with_state(|state| state.interrupts.if_reg)
        }
        IE_REGISTER => {
            // Read Interrupt Enable register from interrupt state
            with_state(|state| state.interrupts.ie)
        }
        // Timer registers (FF04-FF07)
        timer::DIV_REGISTER | timer::TIMA_REGISTER | timer::TMA_REGISTER | timer::TAC_REGISTER => {
            timer::read_timer_register(address)
        }
        _ => {
            // Default I/O register read from MMU
            with_state(|state| state.mmu.io_registers[(address - 0xFF00) as usize])
        }
    }
}

// Write to I/O register
pub fn write_io_register(address: u16, value: u8) {
    match address {
        IF_REGISTER => {
            // Write Interrupt Flag register to interrupt state
            with_state_mut(|state| state.interrupts.if_reg = value);
        }
        IE_REGISTER => {
            // Write Interrupt Enable register to interrupt state
            with_state_mut(|state| state.interrupts.ie = value);
        }
        // Timer registers (FF04-FF07)
        timer::DIV_REGISTER | timer::TIMA_REGISTER | timer::TMA_REGISTER | timer::TAC_REGISTER => {
            timer::write_timer_register(address, value);
        }
        _ => {
            // Default I/O register write to MMU
            with_state_mut(|state| state.mmu.io_registers[(address - 0xFF00) as usize] = value);
        }
    }
}

// Helper functions for specific interrupt register operations
pub fn get_interrupt_enable() -> u8 {
    with_state(|state| state.interrupts.ie)
}

pub fn set_interrupt_enable(value: u8) {
    with_state_mut(|state| state.interrupts.ie = value);
}

pub fn get_interrupt_flag() -> u8 {
    with_state(|state| state.interrupts.if_reg)
}

pub fn set_interrupt_flag(value: u8) {
    with_state_mut(|state| state.interrupts.if_reg = value);
}

// Set specific interrupt flag bits
pub fn set_interrupt_flag_bit(bit: u8, value: bool) {
    with_state_mut(|state| {
        if value {
            state.interrupts.if_reg |= 1 << bit;
        } else {
            state.interrupts.if_reg &= !(1 << bit);
        }
    });
}

// Set specific interrupt enable bits
pub fn set_interrupt_enable_bit(bit: u8, value: bool) {
    with_state_mut(|state| {
        if value {
            state.interrupts.ie |= 1 << bit;
        } else {
            state.interrupts.ie &= !(1 << bit);
        }
    });
}
