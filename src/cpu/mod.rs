use crate::driver::sleep;
use crate::emulator::{with_state, with_state_mut};

async fn memory_read(address: u16) -> u8 {
    sleep(4).await; // Memory read takes 4 cycles
    with_state(|state| state.memory[address as usize])
}

async fn memory_write(address: u16, value: u8) {
    sleep(4).await; // Memory write takes 4 cycles
    with_state_mut(|state| {
        state.memory[address as usize] = value;
    });
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

async fn execute_ld_hl_r(opcode: u8) {
    // Get HL address
    let hl_addr = with_state(|state| state.cpu.hl());
    
    // Decode which register based on opcode bits 2-0
    let reg_bits = opcode & 0b111;
    
    let value = with_state(|state| {
        match reg_bits {
            0b000 => state.cpu.b, // LD (HL), B (0x70)
            0b001 => state.cpu.c, // LD (HL), C (0x71)
            0b010 => state.cpu.d, // LD (HL), D (0x72)
            0b011 => state.cpu.e, // LD (HL), E (0x73)
            0b100 => state.cpu.h, // LD (HL), H (0x74)
            0b101 => state.cpu.l, // LD (HL), L (0x75)
            0b111 => state.cpu.a, // LD (HL), A (0x77)
            _ => panic!("Invalid LD (HL),r opcode: 0x{:02X}", opcode),
        }
    });
    
    // Write data to memory at HL address (second M-cycle)
    memory_write(hl_addr, value).await;
}

async fn execute_ld_hl_n() {
    // LD (HL), n - 3 M-cycles total
    // M2: Read immediate value
    let value = fetch().await;
    // M3: Write to memory at HL address
    let hl_addr = with_state(|state| state.cpu.hl());
    memory_write(hl_addr, value).await;
}

async fn execute_ld_a_bc() {
    // LD A, (BC) - 2 M-cycles total
    // M2: Read from memory at BC address
    let bc_addr = with_state(|state| state.cpu.bc());
    let value = memory_read(bc_addr).await;
    with_state_mut(|state| {
        state.cpu.a = value;
    });
}

async fn execute_ld_a_de() {
    // LD A, (DE) - 2 M-cycles total
    // M2: Read from memory at DE address
    let de_addr = with_state(|state| state.cpu.de());
    let value = memory_read(de_addr).await;
    with_state_mut(|state| {
        state.cpu.a = value;
    });
}

async fn execute_ld_bc_a() {
    // LD (BC), A - 2 M-cycles total
    // M2: Write A to memory at BC address
    let bc_addr = with_state(|state| state.cpu.bc());
    let a_value = with_state(|state| state.cpu.a);
    memory_write(bc_addr, a_value).await;
}

async fn execute_ld_de_a() {
    // LD (DE), A - 2 M-cycles total
    // M2: Write A to memory at DE address
    let de_addr = with_state(|state| state.cpu.de());
    let a_value = with_state(|state| state.cpu.a);
    memory_write(de_addr, a_value).await;
}

async fn execute_ld_a_nn() {
    // LD A, (nn) - 4 M-cycles total
    // M2: Read LSB of address
    let nn_lsb = fetch().await;
    // M3: Read MSB of address
    let nn_msb = fetch().await;
    // Construct 16-bit address
    let nn = ((nn_msb as u16) << 8) | (nn_lsb as u16);
    // M4: Read from memory at nn address
    let value = memory_read(nn).await;
    with_state_mut(|state| {
        state.cpu.a = value;
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
        // LD (HL), r instructions (0b01110xxx)
        0x70 | 0x71 | 0x72 | 0x73 | 0x74 | 0x75 | 0x77 => {
            execute_ld_hl_r(opcode).await;
        }
        // New load instructions
        0x36 => execute_ld_hl_n().await,        // LD (HL), n
        0x0A => execute_ld_a_bc().await,        // LD A, (BC)
        0x1A => execute_ld_a_de().await,        // LD A, (DE)
        0x02 => execute_ld_bc_a().await,        // LD (BC), A
        0x12 => execute_ld_de_a().await,        // LD (DE), A
        0xFA => execute_ld_a_nn().await,        // LD A, (nn)
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
