use crate::driver::sleep;
use crate::emulator::{with_state, with_state_mut};

// Interrupt types and their bit positions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterruptType {
    VBlank = 0,
    LCD = 1,
    Timer = 2,
    Serial = 3,
    Joypad = 4,
}

// Interrupt handler addresses
const VBLANK_HANDLER: u16 = 0x40;
const LCD_HANDLER: u16 = 0x48;
const TIMER_HANDLER: u16 = 0x50;
const SERIAL_HANDLER: u16 = 0x58;
const JOYPAD_HANDLER: u16 = 0x60;

#[derive(Debug, Clone)]
pub struct InterruptState {
    pub ime: bool,      // Interrupt Master Enable (internal CPU flag)
    pub ie: u8,         // Interrupt Enable register (FF00)
    pub if_reg: u8,     // Interrupt Flag register (FF0F)
    pub ei_delay: bool, // EI instruction delay flag
}

impl InterruptState {
    pub fn new() -> Self {
        Self {
            ime: false,      // IME starts disabled
            ie: 0,           // No interrupts enabled initially
            if_reg: 0,       // No interrupts pending initially
            ei_delay: false, // No EI delay initially
        }
    }

    // Request an interrupt by setting the corresponding IF bit
    pub fn request_interrupt(&mut self, interrupt: InterruptType) {
        self.if_reg |= 1 << (interrupt as u8);
    }

    // Check if an interrupt is enabled
    pub fn is_interrupt_enabled(&self, interrupt: InterruptType) -> bool {
        (self.ie & (1 << (interrupt as u8))) != 0
    }

    // Check if an interrupt is requested
    pub fn is_interrupt_requested(&self, interrupt: InterruptType) -> bool {
        (self.if_reg & (1 << (interrupt as u8))) != 0
    }

    // Get the highest priority pending interrupt
    pub fn get_pending_interrupt(&self) -> Option<InterruptType> {
        if !self.ime {
            return None;
        }

        let pending = self.ie & self.if_reg;
        if pending == 0 {
            return None;
        }

        // Check in priority order (bit 0 has highest priority)
        for bit in 0..5 {
            if (pending & (1 << bit)) != 0 {
                return match bit {
                    0 => Some(InterruptType::VBlank),
                    1 => Some(InterruptType::LCD),
                    2 => Some(InterruptType::Timer),
                    3 => Some(InterruptType::Serial),
                    4 => Some(InterruptType::Joypad),
                    _ => None,
                };
            }
        }

        None
    }

    // Acknowledge an interrupt (clear IF bit and disable IME)
    pub fn acknowledge_interrupt(&mut self, interrupt: InterruptType) {
        self.if_reg &= !(1 << (interrupt as u8));
        self.ime = false;
    }

    // Enable interrupts (EI instruction)
    pub fn enable_interrupts(&mut self) {
        self.ei_delay = true; // EI effect is delayed by one instruction
    }

    // Disable interrupts (DI instruction)
    pub fn disable_interrupts(&mut self) {
        self.ime = false;
        self.ei_delay = false;
    }

    // Enable interrupts and return (RETI instruction)
    pub fn enable_interrupts_and_return(&mut self) {
        self.ime = true;
        self.ei_delay = false;
    }

    // Process EI delay (call this after each instruction)
    pub fn process_ei_delay(&mut self) {
        if self.ei_delay {
            self.ime = true;
            self.ei_delay = false;
        }
    }

    // Check if any interrupt should be serviced
    pub fn should_service_interrupt(&self) -> bool {
        self.ime && (self.ie & self.if_reg) != 0
    }
}

// Get interrupt handler address
pub fn get_interrupt_handler_address(interrupt: InterruptType) -> u16 {
    match interrupt {
        InterruptType::VBlank => VBLANK_HANDLER,
        InterruptType::LCD => LCD_HANDLER,
        InterruptType::Timer => TIMER_HANDLER,
        InterruptType::Serial => SERIAL_HANDLER,
        InterruptType::Joypad => JOYPAD_HANDLER,
    }
}

// Main interrupt handling async function
pub async fn handle_interrupt(interrupt: InterruptType) {
    // Acknowledge the interrupt
    with_state_mut(|state| {
        state.interrupts.acknowledge_interrupt(interrupt);
    });

    // Interrupt service routine takes 5 M-cycles total:

    // 1. Two wait states (2 M-cycles)
    sleep(8).await; // 2 M-cycles = 8 cycles

    // 2. Push current PC onto stack (2 M-cycles)
    let pc = with_state(|state| state.cpu.pc);
    push_pc_to_stack(pc).await;

    // 3. Set PC to interrupt handler address (1 M-cycle)
    let handler_addr = get_interrupt_handler_address(interrupt);
    with_state_mut(|state| state.cpu.pc = handler_addr);
    sleep(4).await; // 1 M-cycle = 4 cycles
}

// Push PC to stack (used by interrupt handling)
async fn push_pc_to_stack(pc: u16) {
    // Decrement SP and write high byte
    with_state_mut(|state| state.cpu.sp = state.cpu.sp.wrapping_sub(1));
    let sp = with_state(|state| state.cpu.sp);
    memory_write(sp, (pc >> 8) as u8).await;

    // Decrement SP and write low byte
    with_state_mut(|state| state.cpu.sp = state.cpu.sp.wrapping_sub(1));
    let sp = with_state(|state| state.cpu.sp);
    memory_write(sp, (pc & 0xFF) as u8).await;
}

// Memory access functions for interrupt handling
async fn memory_write(address: u16, value: u8) {
    sleep(4).await; // Memory write takes 4 cycles
    with_state_mut(|state| {
        state.memory[address as usize] = value;
    });
}

// Request interrupt functions for other components to use
pub fn request_vblank_interrupt() {
    with_state_mut(|state| {
        state.interrupts.request_interrupt(InterruptType::VBlank);
    });
}

pub fn request_lcd_interrupt() {
    with_state_mut(|state| {
        state.interrupts.request_interrupt(InterruptType::LCD);
    });
}

pub fn request_timer_interrupt() {
    with_state_mut(|state| {
        state.interrupts.request_interrupt(InterruptType::Timer);
    });
}

pub fn request_serial_interrupt() {
    with_state_mut(|state| {
        state.interrupts.request_interrupt(InterruptType::Serial);
    });
}

pub fn request_joypad_interrupt() {
    with_state_mut(|state| {
        state.interrupts.request_interrupt(InterruptType::Joypad);
    });
}

// Check if interrupts should be processed (for CPU to call)
pub fn check_interrupts() -> Option<InterruptType> {
    with_state(|state| state.interrupts.get_pending_interrupt())
}

// Process EI delay (for CPU to call after each instruction)
pub fn process_ei_delay() {
    with_state_mut(|state| {
        state.interrupts.process_ei_delay();
    });
}

// CPU instruction implementations
pub fn execute_ei() {
    with_state_mut(|state| {
        state.interrupts.enable_interrupts();
    });
}

pub fn execute_di() {
    with_state_mut(|state| {
        state.interrupts.disable_interrupts();
    });
}

pub fn execute_reti() {
    with_state_mut(|state| {
        state.interrupts.enable_interrupts_and_return();
    });
}
