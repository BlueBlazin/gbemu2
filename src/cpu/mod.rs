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

async fn execute_ld_nn_a() {
    // LD (nn), A - 4 M-cycles total
    // M2: Read LSB of address
    let nn_lsb = fetch().await;
    // M3: Read MSB of address
    let nn_msb = fetch().await;
    // Construct 16-bit address
    let nn = ((nn_msb as u16) << 8) | (nn_lsb as u16);
    // M4: Write A to memory at nn address
    let a_value = with_state(|state| state.cpu.a);
    memory_write(nn, a_value).await;
}

async fn execute_ldh_a_c() {
    // LDH A, (C) - 2 M-cycles total
    // M2: Read from memory at 0xFF00+C address
    let c_value = with_state(|state| state.cpu.c);
    let address = 0xFF00 | (c_value as u16);
    let value = memory_read(address).await;
    with_state_mut(|state| {
        state.cpu.a = value;
    });
}

async fn execute_ldh_c_a() {
    // LDH (C), A - 2 M-cycles total
    // M2: Write A to memory at 0xFF00+C address
    let c_value = with_state(|state| state.cpu.c);
    let address = 0xFF00 | (c_value as u16);
    let a_value = with_state(|state| state.cpu.a);
    memory_write(address, a_value).await;
}

async fn execute_ldh_a_n() {
    // LDH A, (n) - 3 M-cycles total
    // M2: Read immediate value n
    let n = fetch().await;
    // M3: Read from memory at 0xFF00+n address
    let address = 0xFF00 | (n as u16);
    let value = memory_read(address).await;
    with_state_mut(|state| {
        state.cpu.a = value;
    });
}

async fn execute_ldh_n_a() {
    // LDH (n), A - 3 M-cycles total
    // M2: Read immediate value n
    let n = fetch().await;
    // M3: Write A to memory at 0xFF00+n address
    let address = 0xFF00 | (n as u16);
    let a_value = with_state(|state| state.cpu.a);
    memory_write(address, a_value).await;
}

async fn execute_ld_a_hl_dec() {
    // LD A, (HL-) - 2 M-cycles total
    // M2: Read from memory at HL address, then decrement HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let value = memory_read(hl_addr).await;
    with_state_mut(|state| {
        state.cpu.a = value;
        state.cpu.set_hl(hl_addr.wrapping_sub(1));
    });
}

async fn execute_ld_hl_dec_a() {
    // LD (HL-), A - 2 M-cycles total
    // M2: Write A to memory at HL address, then decrement HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let a_value = with_state(|state| state.cpu.a);
    memory_write(hl_addr, a_value).await;
    with_state_mut(|state| {
        state.cpu.set_hl(hl_addr.wrapping_sub(1));
    });
}

async fn execute_ld_a_hl_inc() {
    // LD A, (HL+) - 2 M-cycles total
    // M2: Read from memory at HL address, then increment HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let value = memory_read(hl_addr).await;
    with_state_mut(|state| {
        state.cpu.a = value;
        state.cpu.set_hl(hl_addr.wrapping_add(1));
    });
}

async fn execute_ld_hl_inc_a() {
    // LD (HL+), A - 2 M-cycles total
    // M2: Write A to memory at HL address, then increment HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let a_value = with_state(|state| state.cpu.a);
    memory_write(hl_addr, a_value).await;
    with_state_mut(|state| {
        state.cpu.set_hl(hl_addr.wrapping_add(1));
    });
}

async fn execute_ld_rr_nn(opcode: u8) {
    // LD rr, nn - 3 M-cycles total
    // M2: Read LSB
    let nn_lsb = fetch().await;
    // M3: Read MSB
    let nn_msb = fetch().await;
    let nn = ((nn_msb as u16) << 8) | (nn_lsb as u16);
    
    // Decode which register pair based on opcode bits 5-4
    let reg_bits = (opcode >> 4) & 0b11;
    
    with_state_mut(|state| {
        match reg_bits {
            0b00 => state.cpu.set_bc(nn), // LD BC, nn (0x01)
            0b01 => state.cpu.set_de(nn), // LD DE, nn (0x11)
            0b10 => state.cpu.set_hl(nn), // LD HL, nn (0x21)
            0b11 => state.cpu.sp = nn,    // LD SP, nn (0x31)
            _ => unreachable!(),
        }
    });
}

async fn execute_ld_nn_sp() {
    // LD (nn), SP - 5 M-cycles total
    // M2: Read LSB of address
    let nn_lsb = fetch().await;
    // M3: Read MSB of address
    let nn_msb = fetch().await;
    let nn = ((nn_msb as u16) << 8) | (nn_lsb as u16);
    
    let sp = with_state(|state| state.cpu.sp);
    // M4: Write SP low byte
    memory_write(nn, (sp & 0xFF) as u8).await;
    // M5: Write SP high byte
    memory_write(nn.wrapping_add(1), (sp >> 8) as u8).await;
}

async fn execute_ld_sp_hl() {
    // LD SP, HL - 2 M-cycles total
    // M2: Internal operation (no memory access)
    sleep(4).await;
    let hl = with_state(|state| state.cpu.hl());
    with_state_mut(|state| {
        state.cpu.sp = hl;
    });
}

async fn execute_push_rr(opcode: u8) {
    // PUSH rr - 4 M-cycles total
    // M2: Internal operation
    sleep(4).await;
    
    // Decode which register pair based on opcode bits 5-4
    let reg_bits = (opcode >> 4) & 0b11;
    
    let value = with_state(|state| {
        match reg_bits {
            0b00 => state.cpu.bc(), // PUSH BC (0xC5)
            0b01 => state.cpu.de(), // PUSH DE (0xD5)
            0b10 => state.cpu.hl(), // PUSH HL (0xE5)
            0b11 => state.cpu.af(), // PUSH AF (0xF5)
            _ => unreachable!(),
        }
    });
    
    // M3: Write high byte, decrement SP
    let mut sp = with_state(|state| state.cpu.sp);
    sp = sp.wrapping_sub(1);
    memory_write(sp, (value >> 8) as u8).await;
    
    // M4: Write low byte, decrement SP
    sp = sp.wrapping_sub(1);
    memory_write(sp, (value & 0xFF) as u8).await;
    
    with_state_mut(|state| {
        state.cpu.sp = sp;
    });
}

async fn execute_pop_rr(opcode: u8) {
    // POP rr - 3 M-cycles total
    // M2: Read low byte, increment SP
    let mut sp = with_state(|state| state.cpu.sp);
    let lsb = memory_read(sp).await;
    sp = sp.wrapping_add(1);
    
    // M3: Read high byte, increment SP
    let msb = memory_read(sp).await;
    sp = sp.wrapping_add(1);
    
    let value = ((msb as u16) << 8) | (lsb as u16);
    
    // Decode which register pair based on opcode bits 5-4
    let reg_bits = (opcode >> 4) & 0b11;
    
    with_state_mut(|state| {
        state.cpu.sp = sp;
        match reg_bits {
            0b00 => state.cpu.set_bc(value), // POP BC (0xC1)
            0b01 => state.cpu.set_de(value), // POP DE (0xD1)
            0b10 => state.cpu.set_hl(value), // POP HL (0xE1)
            0b11 => state.cpu.set_af(value), // POP AF (0xF1) - sets flags too
            _ => unreachable!(),
        }
    });
}

async fn execute_ld_hl_sp_e() {
    // LD HL, SP+e - 3 M-cycles total
    // M2: Read signed 8-bit offset
    let e = fetch().await;
    
    // M3: Internal operation
    sleep(4).await;
    
    let sp = with_state(|state| state.cpu.sp);
    let e_signed = e as i8 as i16 as u16;
    
    // Calculate result and check for half-carry and carry on lower byte
    let result = sp.wrapping_add(e_signed);
    let half_carry = ((sp & 0xF) + (e_signed & 0xF)) > 0xF;
    let carry = ((sp & 0xFF) + (e_signed & 0xFF)) > 0xFF;
    
    with_state_mut(|state| {
        state.cpu.set_hl(result);
        state.cpu.set_flags(false, false, half_carry, carry); // Z=0, N=0
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
        // 8-bit load instructions
        0x36 => execute_ld_hl_n().await,        // LD (HL), n
        0x0A => execute_ld_a_bc().await,        // LD A, (BC)
        0x1A => execute_ld_a_de().await,        // LD A, (DE)
        0x02 => execute_ld_bc_a().await,        // LD (BC), A
        0x12 => execute_ld_de_a().await,        // LD (DE), A
        0xFA => execute_ld_a_nn().await,        // LD A, (nn)
        0xEA => execute_ld_nn_a().await,        // LD (nn), A
        0xF2 => execute_ldh_a_c().await,        // LDH A, (C)
        0xE2 => execute_ldh_c_a().await,        // LDH (C), A
        0xF0 => execute_ldh_a_n().await,        // LDH A, (n)
        0xE0 => execute_ldh_n_a().await,        // LDH (n), A
        0x3A => execute_ld_a_hl_dec().await,    // LD A, (HL-)
        0x32 => execute_ld_hl_dec_a().await,    // LD (HL-), A
        0x2A => execute_ld_a_hl_inc().await,    // LD A, (HL+)
        0x22 => execute_ld_hl_inc_a().await,    // LD (HL+), A
        // 16-bit load instructions
        0x01 | 0x11 | 0x21 | 0x31 => execute_ld_rr_nn(opcode).await, // LD rr, nn
        0x08 => execute_ld_nn_sp().await,       // LD (nn), SP
        0xF9 => execute_ld_sp_hl().await,       // LD SP, HL
        0xC5 | 0xD5 | 0xE5 | 0xF5 => execute_push_rr(opcode).await, // PUSH rr
        0xC1 | 0xD1 | 0xE1 | 0xF1 => execute_pop_rr(opcode).await,  // POP rr
        0xF8 => execute_ld_hl_sp_e().await,     // LD HL, SP+e
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
