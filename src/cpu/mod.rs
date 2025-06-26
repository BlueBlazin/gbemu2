use crate::driver::sleep;
use crate::emulator::{with_state, with_state_mut};

async fn memory_read(address: u16) -> u8 {
    sleep(4).await; // Memory read takes 4 cycles
    with_state(|state| state.memory[address as usize])
}

async fn fetch() -> u8 {
    let pc = with_state(|state| state.cpu.pc);
    
    let instruction = memory_read(pc).await;
    
    // Increment PC after fetch
    with_state_mut(|state| {
        state.cpu.pc = pc.wrapping_add(1);
    });
    
    instruction
}

async fn execute_ld_r_n(opcode: u8) {
    // Read immediate value (second M-cycle)
    let value = fetch().await;
    
    // Decode which register based on opcode bits 5-3
    let reg_bits = (opcode >> 3) & 0b111;
    
    with_state_mut(|state| {
        match reg_bits {
            0b000 => state.cpu.b = value, // LD B, n (0x06)
            0b001 => state.cpu.c = value, // LD C, n (0x0E)
            0b010 => state.cpu.d = value, // LD D, n (0x16)
            0b011 => state.cpu.e = value, // LD E, n (0x1E)
            0b100 => state.cpu.h = value, // LD H, n (0x26)
            0b101 => state.cpu.l = value, // LD L, n (0x2E)
            0b111 => state.cpu.a = value, // LD A, n (0x3E)
            _ => panic!("Invalid LD r,n opcode: 0x{:02X}", opcode),
        }
    });
}

async fn execute_ld_r_hl(opcode: u8) {
    // Get HL address
    let hl_addr = with_state(|state| state.cpu.hl());
    
    // Read data from memory at HL address (second M-cycle)
    let value = memory_read(hl_addr).await;
    
    // Decode which register based on opcode bits 5-3
    let reg_bits = (opcode >> 3) & 0b111;
    
    with_state_mut(|state| {
        match reg_bits {
            0b000 => state.cpu.b = value, // LD B, (HL) (0x46)
            0b001 => state.cpu.c = value, // LD C, (HL) (0x4E)
            0b010 => state.cpu.d = value, // LD D, (HL) (0x56)
            0b011 => state.cpu.e = value, // LD E, (HL) (0x5E)
            0b100 => state.cpu.h = value, // LD H, (HL) (0x66)
            0b101 => state.cpu.l = value, // LD L, (HL) (0x6E)
            0b111 => state.cpu.a = value, // LD A, (HL) (0x7E)
            _ => panic!("Invalid LD r,(HL) opcode: 0x{:02X}", opcode),
        }
    });
}

async fn decode_and_execute(opcode: u8) {
    match opcode {
        // LD r, n instructions (0b00xxx110)
        0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x3E => {
            execute_ld_r_n(opcode).await;
        }
        // LD r, (HL) instructions (0b01xxx110)
        0x46 | 0x4E | 0x56 | 0x5E | 0x66 | 0x6E | 0x7E => {
            execute_ld_r_hl(opcode).await;
        }
        _ => {
            println!("Unimplemented instruction: 0x{:02X}", opcode);
            // For now, just sleep to avoid infinite loop
            sleep(4).await;
        }
    }
}

pub async fn cpu() {
    loop {
        let instruction = fetch().await;
        decode_and_execute(instruction).await;
    }
}
