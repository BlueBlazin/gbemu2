use crate::driver::sleep;
use crate::emulator::{with_state, with_state_mut};
use crate::interrupts;

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

// Helper function for 8-bit addition with flag calculation
fn add_8bit(a: u8, b: u8, carry: bool) -> (u8, bool, bool, bool) {
    let carry_val = if carry { 1 } else { 0 };
    let result = a.wrapping_add(b).wrapping_add(carry_val);
    let full_result = (a as u16) + (b as u16) + (carry_val as u16);

    let zero = result == 0;
    let half_carry = ((a & 0xF) + (b & 0xF) + carry_val) > 0xF;
    let carry_flag = full_result > 0xFF;

    (result, zero, half_carry, carry_flag)
}

// Helper function for 8-bit subtraction with flag calculation
fn sub_8bit(a: u8, b: u8, carry: bool) -> (u8, bool, bool, bool) {
    let carry_val = if carry { 1 } else { 0 };
    let result = a.wrapping_sub(b).wrapping_sub(carry_val);

    let zero = result == 0;
    let half_carry = (a & 0xF) < ((b & 0xF) + carry_val);
    let carry_flag = (a as u16) < ((b as u16) + (carry_val as u16));

    (result, zero, half_carry, carry_flag)
}

// Helper function for 8-bit increment with flag calculation (no carry flag affected)
fn inc_8bit(a: u8) -> (u8, bool, bool) {
    let result = a.wrapping_add(1);
    let zero = result == 0;
    let half_carry = (a & 0xF) == 0xF; // Half-carry occurs when lower nibble is 0xF
    (result, zero, half_carry)
}

// Helper function for 8-bit decrement with flag calculation (no carry flag affected)
fn dec_8bit(a: u8) -> (u8, bool, bool) {
    let result = a.wrapping_sub(1);
    let zero = result == 0;
    let half_carry = (a & 0xF) == 0; // Half-carry occurs when lower nibble is 0
    (result, zero, half_carry)
}

async fn execute_add_r(opcode: u8) {
    // ADD r - 1 or 2 M-cycles total depending on operand
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // ADD (HL) - 2 M-cycles total
        execute_add_hl().await;
        return;
    }

    let (a_val, operand) = with_state(|state| {
        let a = state.cpu.a;
        let op = match reg_bits {
            0b000 => state.cpu.b, // ADD B (0x80)
            0b001 => state.cpu.c, // ADD C (0x81)
            0b010 => state.cpu.d, // ADD D (0x82)
            0b011 => state.cpu.e, // ADD E (0x83)
            0b100 => state.cpu.h, // ADD H (0x84)
            0b101 => state.cpu.l, // ADD L (0x85)
            0b111 => state.cpu.a, // ADD A (0x87)
            _ => unreachable!(),
        };
        (a, op)
    });

    let (result, zero, half_carry, carry) = add_8bit(a_val, operand, false);

    with_state_mut(|state| {
        state.cpu.a = result;
        state.cpu.set_flags(zero, false, half_carry, carry);
    });
}

async fn execute_add_hl() {
    // ADD (HL) - 2 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let operand = memory_read(hl_addr).await;

    let a_val = with_state(|state| state.cpu.a);
    let (result, zero, half_carry, carry) = add_8bit(a_val, operand, false);

    with_state_mut(|state| {
        state.cpu.a = result;
        state.cpu.set_flags(zero, false, half_carry, carry);
    });
}

async fn execute_add_n() {
    // ADD n - 2 M-cycles total
    // M2: Read immediate value
    let operand = fetch().await;

    let a_val = with_state(|state| state.cpu.a);
    let (result, zero, half_carry, carry) = add_8bit(a_val, operand, false);

    with_state_mut(|state| {
        state.cpu.a = result;
        state.cpu.set_flags(zero, false, half_carry, carry);
    });
}

async fn execute_adc_r(opcode: u8) {
    // ADC r - 1 or 2 M-cycles total depending on operand
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // ADC (HL) - 2 M-cycles total
        execute_adc_hl().await;
        return;
    }

    let (a_val, operand, carry_flag) = with_state(|state| {
        let a = state.cpu.a;
        let op = match reg_bits {
            0b000 => state.cpu.b, // ADC B (0x88)
            0b001 => state.cpu.c, // ADC C (0x89)
            0b010 => state.cpu.d, // ADC D (0x8A)
            0b011 => state.cpu.e, // ADC E (0x8B)
            0b100 => state.cpu.h, // ADC H (0x8C)
            0b101 => state.cpu.l, // ADC L (0x8D)
            0b111 => state.cpu.a, // ADC A (0x8F)
            _ => unreachable!(),
        };
        (a, op, state.cpu.flag_c())
    });

    let (result, zero, half_carry, carry) = add_8bit(a_val, operand, carry_flag);

    with_state_mut(|state| {
        state.cpu.a = result;
        state.cpu.set_flags(zero, false, half_carry, carry);
    });
}

async fn execute_adc_hl() {
    // ADC (HL) - 2 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let operand = memory_read(hl_addr).await;

    let (a_val, carry_flag) = with_state(|state| (state.cpu.a, state.cpu.flag_c()));
    let (result, zero, half_carry, carry) = add_8bit(a_val, operand, carry_flag);

    with_state_mut(|state| {
        state.cpu.a = result;
        state.cpu.set_flags(zero, false, half_carry, carry);
    });
}

async fn execute_adc_n() {
    // ADC n - 2 M-cycles total
    // M2: Read immediate value
    let operand = fetch().await;

    let (a_val, carry_flag) = with_state(|state| (state.cpu.a, state.cpu.flag_c()));
    let (result, zero, half_carry, carry) = add_8bit(a_val, operand, carry_flag);

    with_state_mut(|state| {
        state.cpu.a = result;
        state.cpu.set_flags(zero, false, half_carry, carry);
    });
}

async fn execute_sub_r(opcode: u8) {
    // SUB r - 1 or 2 M-cycles total depending on operand
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // SUB (HL) - 2 M-cycles total
        execute_sub_hl().await;
        return;
    }

    let (a_val, operand) = with_state(|state| {
        let a = state.cpu.a;
        let op = match reg_bits {
            0b000 => state.cpu.b, // SUB B (0x90)
            0b001 => state.cpu.c, // SUB C (0x91)
            0b010 => state.cpu.d, // SUB D (0x92)
            0b011 => state.cpu.e, // SUB E (0x93)
            0b100 => state.cpu.h, // SUB H (0x94)
            0b101 => state.cpu.l, // SUB L (0x95)
            0b111 => state.cpu.a, // SUB A (0x97)
            _ => unreachable!(),
        };
        (a, op)
    });

    let (result, zero, half_carry, carry) = sub_8bit(a_val, operand, false);

    with_state_mut(|state| {
        state.cpu.a = result;
        state.cpu.set_flags(zero, true, half_carry, carry); // N = 1 for subtraction
    });
}

async fn execute_sub_hl() {
    // SUB (HL) - 2 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let operand = memory_read(hl_addr).await;

    let a_val = with_state(|state| state.cpu.a);
    let (result, zero, half_carry, carry) = sub_8bit(a_val, operand, false);

    with_state_mut(|state| {
        state.cpu.a = result;
        state.cpu.set_flags(zero, true, half_carry, carry); // N = 1 for subtraction
    });
}

async fn execute_sub_n() {
    // SUB n - 2 M-cycles total
    // M2: Read immediate value
    let operand = fetch().await;

    let a_val = with_state(|state| state.cpu.a);
    let (result, zero, half_carry, carry) = sub_8bit(a_val, operand, false);

    with_state_mut(|state| {
        state.cpu.a = result;
        state.cpu.set_flags(zero, true, half_carry, carry); // N = 1 for subtraction
    });
}

async fn execute_sbc_r(opcode: u8) {
    // SBC r - 1 or 2 M-cycles total depending on operand
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // SBC (HL) - 2 M-cycles total
        execute_sbc_hl().await;
        return;
    }

    let (a_val, operand, carry_flag) = with_state(|state| {
        let a = state.cpu.a;
        let op = match reg_bits {
            0b000 => state.cpu.b, // SBC B (0x98)
            0b001 => state.cpu.c, // SBC C (0x99)
            0b010 => state.cpu.d, // SBC D (0x9A)
            0b011 => state.cpu.e, // SBC E (0x9B)
            0b100 => state.cpu.h, // SBC H (0x9C)
            0b101 => state.cpu.l, // SBC L (0x9D)
            0b111 => state.cpu.a, // SBC A (0x9F)
            _ => unreachable!(),
        };
        (a, op, state.cpu.flag_c())
    });

    let (result, zero, half_carry, carry) = sub_8bit(a_val, operand, carry_flag);

    with_state_mut(|state| {
        state.cpu.a = result;
        state.cpu.set_flags(zero, true, half_carry, carry); // N = 1 for subtraction
    });
}

async fn execute_sbc_hl() {
    // SBC (HL) - 2 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let operand = memory_read(hl_addr).await;

    let (a_val, carry_flag) = with_state(|state| (state.cpu.a, state.cpu.flag_c()));
    let (result, zero, half_carry, carry) = sub_8bit(a_val, operand, carry_flag);

    with_state_mut(|state| {
        state.cpu.a = result;
        state.cpu.set_flags(zero, true, half_carry, carry); // N = 1 for subtraction
    });
}

async fn execute_sbc_n() {
    // SBC n - 2 M-cycles total
    // M2: Read immediate value
    let operand = fetch().await;

    let (a_val, carry_flag) = with_state(|state| (state.cpu.a, state.cpu.flag_c()));
    let (result, zero, half_carry, carry) = sub_8bit(a_val, operand, carry_flag);

    with_state_mut(|state| {
        state.cpu.a = result;
        state.cpu.set_flags(zero, true, half_carry, carry); // N = 1 for subtraction
    });
}

async fn execute_cp_r(opcode: u8) {
    // CP r - 1 or 2 M-cycles total depending on operand
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // CP (HL) - 2 M-cycles total
        execute_cp_hl().await;
        return;
    }

    let (a_val, operand) = with_state(|state| {
        let a = state.cpu.a;
        let op = match reg_bits {
            0b000 => state.cpu.b, // CP B (0xB8)
            0b001 => state.cpu.c, // CP C (0xB9)
            0b010 => state.cpu.d, // CP D (0xBA)
            0b011 => state.cpu.e, // CP E (0xBB)
            0b100 => state.cpu.h, // CP H (0xBC)
            0b101 => state.cpu.l, // CP L (0xBD)
            0b111 => state.cpu.a, // CP A (0xBF)
            _ => unreachable!(),
        };
        (a, op)
    });

    let (_, zero, half_carry, carry) = sub_8bit(a_val, operand, false);

    with_state_mut(|state| {
        // CP doesn't update A register, only flags
        state.cpu.set_flags(zero, true, half_carry, carry); // N = 1 for subtraction
    });
}

async fn execute_cp_hl() {
    // CP (HL) - 2 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let operand = memory_read(hl_addr).await;

    let a_val = with_state(|state| state.cpu.a);
    let (_, zero, half_carry, carry) = sub_8bit(a_val, operand, false);

    with_state_mut(|state| {
        // CP doesn't update A register, only flags
        state.cpu.set_flags(zero, true, half_carry, carry); // N = 1 for subtraction
    });
}

async fn execute_cp_n() {
    // CP n - 2 M-cycles total
    // M2: Read immediate value
    let operand = fetch().await;

    let a_val = with_state(|state| state.cpu.a);
    let (_, zero, half_carry, carry) = sub_8bit(a_val, operand, false);

    with_state_mut(|state| {
        // CP doesn't update A register, only flags
        state.cpu.set_flags(zero, true, half_carry, carry); // N = 1 for subtraction
    });
}

async fn execute_inc_r(opcode: u8) {
    // INC r - 1 M-cycle total
    // Decode which register based on opcode bits 5-3
    let reg_bits = (opcode >> 3) & 0b111;

    let (old_val, carry_flag) = with_state(|state| {
        let val = match reg_bits {
            0b000 => state.cpu.b, // INC B (0x04)
            0b001 => state.cpu.c, // INC C (0x0C)
            0b010 => state.cpu.d, // INC D (0x14)
            0b011 => state.cpu.e, // INC E (0x1C)
            0b100 => state.cpu.h, // INC H (0x24)
            0b101 => state.cpu.l, // INC L (0x2C)
            0b111 => state.cpu.a, // INC A (0x3C)
            _ => panic!("Invalid INC r opcode: 0x{:02X}", opcode),
        };
        (val, state.cpu.flag_c()) // Preserve carry flag
    });

    let (result, zero, half_carry) = inc_8bit(old_val);

    with_state_mut(|state| {
        match reg_bits {
            0b000 => state.cpu.b = result,
            0b001 => state.cpu.c = result,
            0b010 => state.cpu.d = result,
            0b011 => state.cpu.e = result,
            0b100 => state.cpu.h = result,
            0b101 => state.cpu.l = result,
            0b111 => state.cpu.a = result,
            _ => unreachable!(),
        }
        // INC preserves carry flag, sets Z=result==0, N=0, H=half_carry
        state.cpu.set_flags(zero, false, half_carry, carry_flag);
    });
}

async fn execute_inc_hl() {
    // INC (HL) - 3 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let old_val = memory_read(hl_addr).await;

    let (result, zero, half_carry) = inc_8bit(old_val);
    let carry_flag = with_state(|state| state.cpu.flag_c()); // Preserve carry flag

    // M3: Write result back to memory
    memory_write(hl_addr, result).await;

    with_state_mut(|state| {
        // INC preserves carry flag, sets Z=result==0, N=0, H=half_carry
        state.cpu.set_flags(zero, false, half_carry, carry_flag);
    });
}

async fn execute_dec_r(opcode: u8) {
    // DEC r - 1 M-cycle total
    // Decode which register based on opcode bits 5-3
    let reg_bits = (opcode >> 3) & 0b111;

    let (old_val, carry_flag) = with_state(|state| {
        let val = match reg_bits {
            0b000 => state.cpu.b, // DEC B (0x05)
            0b001 => state.cpu.c, // DEC C (0x0D)
            0b010 => state.cpu.d, // DEC D (0x15)
            0b011 => state.cpu.e, // DEC E (0x1D)
            0b100 => state.cpu.h, // DEC H (0x25)
            0b101 => state.cpu.l, // DEC L (0x2D)
            0b111 => state.cpu.a, // DEC A (0x3D)
            _ => panic!("Invalid DEC r opcode: 0x{:02X}", opcode),
        };
        (val, state.cpu.flag_c()) // Preserve carry flag
    });

    let (result, zero, half_carry) = dec_8bit(old_val);

    with_state_mut(|state| {
        match reg_bits {
            0b000 => state.cpu.b = result,
            0b001 => state.cpu.c = result,
            0b010 => state.cpu.d = result,
            0b011 => state.cpu.e = result,
            0b100 => state.cpu.h = result,
            0b101 => state.cpu.l = result,
            0b111 => state.cpu.a = result,
            _ => unreachable!(),
        }
        // DEC preserves carry flag, sets Z=result==0, N=1, H=half_carry
        state.cpu.set_flags(zero, true, half_carry, carry_flag);
    });
}

async fn execute_dec_hl() {
    // DEC (HL) - 3 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let old_val = memory_read(hl_addr).await;

    let (result, zero, half_carry) = dec_8bit(old_val);
    let carry_flag = with_state(|state| state.cpu.flag_c()); // Preserve carry flag

    // M3: Write result back to memory
    memory_write(hl_addr, result).await;

    with_state_mut(|state| {
        // DEC preserves carry flag, sets Z=result==0, N=1, H=half_carry
        state.cpu.set_flags(zero, true, half_carry, carry_flag);
    });
}

async fn execute_and_r(opcode: u8) {
    // AND r - 1 or 2 M-cycles total depending on operand
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // AND (HL) - 2 M-cycles total
        execute_and_hl().await;
        return;
    }

    let (a_val, operand) = with_state(|state| {
        let a = state.cpu.a;
        let op = match reg_bits {
            0b000 => state.cpu.b, // AND B (0xA0)
            0b001 => state.cpu.c, // AND C (0xA1)
            0b010 => state.cpu.d, // AND D (0xA2)
            0b011 => state.cpu.e, // AND E (0xA3)
            0b100 => state.cpu.h, // AND H (0xA4)
            0b101 => state.cpu.l, // AND L (0xA5)
            0b111 => state.cpu.a, // AND A (0xA7)
            _ => unreachable!(),
        };
        (a, op)
    });

    let result = a_val & operand;
    let zero = result == 0;

    with_state_mut(|state| {
        state.cpu.a = result;
        // AND sets Z=result==0, N=0, H=1, C=0
        state.cpu.set_flags(zero, false, true, false);
    });
}

async fn execute_and_hl() {
    // AND (HL) - 2 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let operand = memory_read(hl_addr).await;

    let a_val = with_state(|state| state.cpu.a);
    let result = a_val & operand;
    let zero = result == 0;

    with_state_mut(|state| {
        state.cpu.a = result;
        // AND sets Z=result==0, N=0, H=1, C=0
        state.cpu.set_flags(zero, false, true, false);
    });
}

async fn execute_and_n() {
    // AND n - 2 M-cycles total
    // M2: Read immediate value
    let operand = fetch().await;

    let a_val = with_state(|state| state.cpu.a);
    let result = a_val & operand;
    let zero = result == 0;

    with_state_mut(|state| {
        state.cpu.a = result;
        // AND sets Z=result==0, N=0, H=1, C=0
        state.cpu.set_flags(zero, false, true, false);
    });
}

async fn execute_or_r(opcode: u8) {
    // OR r - 1 or 2 M-cycles total depending on operand
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // OR (HL) - 2 M-cycles total
        execute_or_hl().await;
        return;
    }

    let (a_val, operand) = with_state(|state| {
        let a = state.cpu.a;
        let op = match reg_bits {
            0b000 => state.cpu.b, // OR B (0xB0)
            0b001 => state.cpu.c, // OR C (0xB1)
            0b010 => state.cpu.d, // OR D (0xB2)
            0b011 => state.cpu.e, // OR E (0xB3)
            0b100 => state.cpu.h, // OR H (0xB4)
            0b101 => state.cpu.l, // OR L (0xB5)
            0b111 => state.cpu.a, // OR A (0xB7)
            _ => unreachable!(),
        };
        (a, op)
    });

    let result = a_val | operand;
    let zero = result == 0;

    with_state_mut(|state| {
        state.cpu.a = result;
        // OR sets Z=result==0, N=0, H=0, C=0
        state.cpu.set_flags(zero, false, false, false);
    });
}

async fn execute_or_hl() {
    // OR (HL) - 2 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let operand = memory_read(hl_addr).await;

    let a_val = with_state(|state| state.cpu.a);
    let result = a_val | operand;
    let zero = result == 0;

    with_state_mut(|state| {
        state.cpu.a = result;
        // OR sets Z=result==0, N=0, H=0, C=0
        state.cpu.set_flags(zero, false, false, false);
    });
}

async fn execute_or_n() {
    // OR n - 2 M-cycles total
    // M2: Read immediate value
    let operand = fetch().await;

    let a_val = with_state(|state| state.cpu.a);
    let result = a_val | operand;
    let zero = result == 0;

    with_state_mut(|state| {
        state.cpu.a = result;
        // OR sets Z=result==0, N=0, H=0, C=0
        state.cpu.set_flags(zero, false, false, false);
    });
}

async fn execute_xor_r(opcode: u8) {
    // XOR r - 1 or 2 M-cycles total depending on operand
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // XOR (HL) - 2 M-cycles total
        execute_xor_hl().await;
        return;
    }

    let (a_val, operand) = with_state(|state| {
        let a = state.cpu.a;
        let op = match reg_bits {
            0b000 => state.cpu.b, // XOR B (0xA8)
            0b001 => state.cpu.c, // XOR C (0xA9)
            0b010 => state.cpu.d, // XOR D (0xAA)
            0b011 => state.cpu.e, // XOR E (0xAB)
            0b100 => state.cpu.h, // XOR H (0xAC)
            0b101 => state.cpu.l, // XOR L (0xAD)
            0b111 => state.cpu.a, // XOR A (0xAF)
            _ => unreachable!(),
        };
        (a, op)
    });

    let result = a_val ^ operand;
    let zero = result == 0;

    with_state_mut(|state| {
        state.cpu.a = result;
        // XOR sets Z=result==0, N=0, H=0, C=0
        state.cpu.set_flags(zero, false, false, false);
    });
}

async fn execute_xor_hl() {
    // XOR (HL) - 2 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let operand = memory_read(hl_addr).await;

    let a_val = with_state(|state| state.cpu.a);
    let result = a_val ^ operand;
    let zero = result == 0;

    with_state_mut(|state| {
        state.cpu.a = result;
        // XOR sets Z=result==0, N=0, H=0, C=0
        state.cpu.set_flags(zero, false, false, false);
    });
}

async fn execute_xor_n() {
    // XOR n - 2 M-cycles total
    // M2: Read immediate value
    let operand = fetch().await;

    let a_val = with_state(|state| state.cpu.a);
    let result = a_val ^ operand;
    let zero = result == 0;

    with_state_mut(|state| {
        state.cpu.a = result;
        // XOR sets Z=result==0, N=0, H=0, C=0
        state.cpu.set_flags(zero, false, false, false);
    });
}

async fn execute_ccf() {
    // CCF - 1 M-cycle total
    // Complement carry flag, clear N and H flags
    with_state_mut(|state| {
        let current_z = state.cpu.flag_z();
        let current_c = state.cpu.flag_c();
        // Flip carry flag, clear N and H, preserve Z
        state.cpu.set_flags(current_z, false, false, !current_c);
    });
}

async fn execute_scf() {
    // SCF - 1 M-cycle total
    // Set carry flag, clear N and H flags
    with_state_mut(|state| {
        let current_z = state.cpu.flag_z();
        // Set carry flag, clear N and H, preserve Z
        state.cpu.set_flags(current_z, false, false, true);
    });
}

async fn execute_daa() {
    // DAA - 1 M-cycle total
    // Decimal adjust accumulator
    with_state_mut(|state| {
        let mut a = state.cpu.a as u16;

        if !state.cpu.flag_n() {
            // Previous operation was addition
            if state.cpu.flag_h() || (a & 0xF) > 0x9 {
                a = a.wrapping_add(0x6);
            }
            if state.cpu.flag_c() || a > 0x9F {
                a = a.wrapping_add(0x60);
                state.cpu.set_flag_c(true);
            } else {
                state.cpu.set_flag_c(false);
            }
        } else {
            // Previous operation was subtraction
            if state.cpu.flag_h() {
                a = a.wrapping_sub(0x6);
            }
            if state.cpu.flag_c() {
                a = a.wrapping_sub(0x60);
            }
        }

        let result = (a & 0xFF) as u8;
        state.cpu.a = result;
        state.cpu.set_flag_z(result == 0);
        state.cpu.set_flag_h(false);
        // Carry flag was already set above if needed
    });
}

async fn execute_cpl() {
    // CPL - 1 M-cycle total
    // Complement accumulator, set N and H flags
    with_state_mut(|state| {
        state.cpu.a = !state.cpu.a;
        let current_z = state.cpu.flag_z();
        let current_c = state.cpu.flag_c();
        // Set N and H flags, preserve Z and C
        state.cpu.set_flags(current_z, true, true, current_c);
    });
}

async fn execute_inc_rr(opcode: u8) {
    // INC rr - 2 M-cycles total
    // M2: Internal operation (no memory access)
    sleep(4).await;

    // Decode which register pair based on opcode bits 5-4
    let reg_bits = (opcode >> 4) & 0b11;

    with_state_mut(|state| {
        match reg_bits {
            0b00 => {
                let bc = state.cpu.bc().wrapping_add(1);
                state.cpu.set_bc(bc);
            } // INC BC (0x03)
            0b01 => {
                let de = state.cpu.de().wrapping_add(1);
                state.cpu.set_de(de);
            } // INC DE (0x13)
            0b10 => {
                let hl = state.cpu.hl().wrapping_add(1);
                state.cpu.set_hl(hl);
            } // INC HL (0x23)
            0b11 => {
                state.cpu.sp = state.cpu.sp.wrapping_add(1);
            } // INC SP (0x33)
            _ => unreachable!(),
        }
        // INC rr doesn't affect any flags
    });
}

async fn execute_dec_rr(opcode: u8) {
    // DEC rr - 2 M-cycles total
    // M2: Internal operation (no memory access)
    sleep(4).await;

    // Decode which register pair based on opcode bits 5-4
    let reg_bits = (opcode >> 4) & 0b11;

    with_state_mut(|state| {
        match reg_bits {
            0b00 => {
                let bc = state.cpu.bc().wrapping_sub(1);
                state.cpu.set_bc(bc);
            } // DEC BC (0x0B)
            0b01 => {
                let de = state.cpu.de().wrapping_sub(1);
                state.cpu.set_de(de);
            } // DEC DE (0x1B)
            0b10 => {
                let hl = state.cpu.hl().wrapping_sub(1);
                state.cpu.set_hl(hl);
            } // DEC HL (0x2B)
            0b11 => {
                state.cpu.sp = state.cpu.sp.wrapping_sub(1);
            } // DEC SP (0x3B)
            _ => unreachable!(),
        }
        // DEC rr doesn't affect any flags
    });
}

async fn execute_add_hl_rr(opcode: u8) {
    // ADD HL, rr - 2 M-cycles total
    // M2: Internal operation (no memory access)
    sleep(4).await;

    // Decode which register pair based on opcode bits 5-4
    let reg_bits = (opcode >> 4) & 0b11;

    let (hl_val, operand, current_z) = with_state(|state| {
        let hl = state.cpu.hl();
        let op = match reg_bits {
            0b00 => state.cpu.bc(), // ADD HL, BC (0x09)
            0b01 => state.cpu.de(), // ADD HL, DE (0x19)
            0b10 => state.cpu.hl(), // ADD HL, HL (0x29)
            0b11 => state.cpu.sp,   // ADD HL, SP (0x39)
            _ => unreachable!(),
        };
        (hl, op, state.cpu.flag_z())
    });

    // Calculate result and flags
    let result = hl_val.wrapping_add(operand);
    let full_result = (hl_val as u32) + (operand as u32);

    // Half-carry occurs when there's a carry from bit 11 to bit 12
    let half_carry = ((hl_val & 0xFFF) + (operand & 0xFFF)) > 0xFFF;
    let carry = full_result > 0xFFFF;

    with_state_mut(|state| {
        state.cpu.set_hl(result);
        // ADD HL,rr sets N=0, H=half_carry, C=carry, preserves Z
        state.cpu.set_flags(current_z, false, half_carry, carry);
    });
}

async fn execute_add_sp_e() {
    // ADD SP, e - 4 M-cycles total
    // M2: Read signed 8-bit offset
    let e = fetch().await;

    // M3: Internal operation
    sleep(4).await;

    // M4: Internal operation
    sleep(4).await;

    let sp = with_state(|state| state.cpu.sp);
    let e_signed = e as i8 as i16 as u16;

    // Calculate result and check for half-carry and carry on lower byte
    let result = sp.wrapping_add(e_signed);
    let half_carry = ((sp & 0xF) + (e_signed & 0xF)) > 0xF;
    let carry = ((sp & 0xFF) + (e_signed & 0xFF)) > 0xFF;

    with_state_mut(|state| {
        state.cpu.sp = result;
        // ADD SP,e sets Z=0, N=0, H=half_carry, C=carry
        state.cpu.set_flags(false, false, half_carry, carry);
    });
}

async fn execute_rlca() {
    // RLCA - 1 M-cycle total
    // Rotate left circular accumulator
    with_state_mut(|state| {
        let a = state.cpu.a;
        let bit7 = (a & 0x80) != 0;
        let result = (a << 1) | if bit7 { 1 } else { 0 };

        state.cpu.a = result;
        // RLCA sets Z=0, N=0, H=0, C=old_bit7
        state.cpu.set_flags(false, false, false, bit7);
    });
}

async fn execute_rrca() {
    // RRCA - 1 M-cycle total
    // Rotate right circular accumulator
    with_state_mut(|state| {
        let a = state.cpu.a;
        let bit0 = (a & 0x01) != 0;
        let result = (a >> 1) | if bit0 { 0x80 } else { 0 };

        state.cpu.a = result;
        // RRCA sets Z=0, N=0, H=0, C=old_bit0
        state.cpu.set_flags(false, false, false, bit0);
    });
}

async fn execute_rla() {
    // RLA - 1 M-cycle total
    // Rotate left accumulator through carry
    with_state_mut(|state| {
        let a = state.cpu.a;
        let old_carry = state.cpu.flag_c();
        let bit7 = (a & 0x80) != 0;
        let result = (a << 1) | if old_carry { 1 } else { 0 };

        state.cpu.a = result;
        // RLA sets Z=0, N=0, H=0, C=old_bit7
        state.cpu.set_flags(false, false, false, bit7);
    });
}

async fn execute_rra() {
    // RRA - 1 M-cycle total
    // Rotate right accumulator through carry
    with_state_mut(|state| {
        let a = state.cpu.a;
        let old_carry = state.cpu.flag_c();
        let bit0 = (a & 0x01) != 0;
        let result = (a >> 1) | if old_carry { 0x80 } else { 0 };

        state.cpu.a = result;
        // RRA sets Z=0, N=0, H=0, C=old_bit0
        state.cpu.set_flags(false, false, false, bit0);
    });
}

// Helper functions for CB-prefixed rotate/shift operations
fn rlc_8bit(value: u8) -> (u8, bool, bool) {
    let bit7 = (value & 0x80) != 0;
    let result = (value << 1) | if bit7 { 1 } else { 0 };
    let zero = result == 0;
    (result, zero, bit7)
}

fn rrc_8bit(value: u8) -> (u8, bool, bool) {
    let bit0 = (value & 0x01) != 0;
    let result = (value >> 1) | if bit0 { 0x80 } else { 0 };
    let zero = result == 0;
    (result, zero, bit0)
}

fn rl_8bit(value: u8, carry_in: bool) -> (u8, bool, bool) {
    let bit7 = (value & 0x80) != 0;
    let result = (value << 1) | if carry_in { 1 } else { 0 };
    let zero = result == 0;
    (result, zero, bit7)
}

fn rr_8bit(value: u8, carry_in: bool) -> (u8, bool, bool) {
    let bit0 = (value & 0x01) != 0;
    let result = (value >> 1) | if carry_in { 0x80 } else { 0 };
    let zero = result == 0;
    (result, zero, bit0)
}

fn sla_8bit(value: u8) -> (u8, bool, bool) {
    let bit7 = (value & 0x80) != 0;
    let result = value << 1;
    let zero = result == 0;
    (result, zero, bit7)
}

fn sra_8bit(value: u8) -> (u8, bool, bool) {
    let bit0 = (value & 0x01) != 0;
    let bit7 = value & 0x80; // Preserve sign bit
    let result = (value >> 1) | bit7;
    let zero = result == 0;
    (result, zero, bit0)
}

fn swap_8bit(value: u8) -> (u8, bool) {
    let result = ((value & 0x0F) << 4) | ((value & 0xF0) >> 4);
    let zero = result == 0;
    (result, zero)
}

fn srl_8bit(value: u8) -> (u8, bool, bool) {
    let bit0 = (value & 0x01) != 0;
    let result = value >> 1; // Logical shift (no sign preservation)
    let zero = result == 0;
    (result, zero, bit0)
}

async fn execute_cb_rlc_r(opcode: u8) {
    // RLC r - 2 M-cycles total
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // RLC (HL) - 4 M-cycles total
        execute_cb_rlc_hl().await;
        return;
    }

    let old_val = with_state(|state| match reg_bits {
        0b000 => state.cpu.b,
        0b001 => state.cpu.c,
        0b010 => state.cpu.d,
        0b011 => state.cpu.e,
        0b100 => state.cpu.h,
        0b101 => state.cpu.l,
        0b111 => state.cpu.a,
        _ => unreachable!(),
    });

    let (result, zero, carry) = rlc_8bit(old_val);

    with_state_mut(|state| {
        match reg_bits {
            0b000 => state.cpu.b = result,
            0b001 => state.cpu.c = result,
            0b010 => state.cpu.d = result,
            0b011 => state.cpu.e = result,
            0b100 => state.cpu.h = result,
            0b101 => state.cpu.l = result,
            0b111 => state.cpu.a = result,
            _ => unreachable!(),
        }
        // RLC sets Z=result==0, N=0, H=0, C=old_bit7
        state.cpu.set_flags(zero, false, false, carry);
    });
}

async fn execute_cb_rlc_hl() {
    // RLC (HL) - 4 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let old_val = memory_read(hl_addr).await;

    let (result, zero, carry) = rlc_8bit(old_val);

    // M3: Write result back to memory
    memory_write(hl_addr, result).await;

    with_state_mut(|state| {
        // RLC sets Z=result==0, N=0, H=0, C=old_bit7
        state.cpu.set_flags(zero, false, false, carry);
    });
}

async fn execute_cb_rrc_r(opcode: u8) {
    // RRC r - 2 M-cycles total
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // RRC (HL) - 4 M-cycles total
        execute_cb_rrc_hl().await;
        return;
    }

    let old_val = with_state(|state| match reg_bits {
        0b000 => state.cpu.b,
        0b001 => state.cpu.c,
        0b010 => state.cpu.d,
        0b011 => state.cpu.e,
        0b100 => state.cpu.h,
        0b101 => state.cpu.l,
        0b111 => state.cpu.a,
        _ => unreachable!(),
    });

    let (result, zero, carry) = rrc_8bit(old_val);

    with_state_mut(|state| {
        match reg_bits {
            0b000 => state.cpu.b = result,
            0b001 => state.cpu.c = result,
            0b010 => state.cpu.d = result,
            0b011 => state.cpu.e = result,
            0b100 => state.cpu.h = result,
            0b101 => state.cpu.l = result,
            0b111 => state.cpu.a = result,
            _ => unreachable!(),
        }
        // RRC sets Z=result==0, N=0, H=0, C=old_bit0
        state.cpu.set_flags(zero, false, false, carry);
    });
}

async fn execute_cb_rrc_hl() {
    // RRC (HL) - 4 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let old_val = memory_read(hl_addr).await;

    let (result, zero, carry) = rrc_8bit(old_val);

    // M3: Write result back to memory
    memory_write(hl_addr, result).await;

    with_state_mut(|state| {
        // RRC sets Z=result==0, N=0, H=0, C=old_bit0
        state.cpu.set_flags(zero, false, false, carry);
    });
}

async fn execute_cb_rl_r(opcode: u8) {
    // RL r - 2 M-cycles total
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // RL (HL) - 4 M-cycles total
        execute_cb_rl_hl().await;
        return;
    }

    let (old_val, carry_in) = with_state(|state| {
        let val = match reg_bits {
            0b000 => state.cpu.b,
            0b001 => state.cpu.c,
            0b010 => state.cpu.d,
            0b011 => state.cpu.e,
            0b100 => state.cpu.h,
            0b101 => state.cpu.l,
            0b111 => state.cpu.a,
            _ => unreachable!(),
        };
        (val, state.cpu.flag_c())
    });

    let (result, zero, carry) = rl_8bit(old_val, carry_in);

    with_state_mut(|state| {
        match reg_bits {
            0b000 => state.cpu.b = result,
            0b001 => state.cpu.c = result,
            0b010 => state.cpu.d = result,
            0b011 => state.cpu.e = result,
            0b100 => state.cpu.h = result,
            0b101 => state.cpu.l = result,
            0b111 => state.cpu.a = result,
            _ => unreachable!(),
        }
        // RL sets Z=result==0, N=0, H=0, C=old_bit7
        state.cpu.set_flags(zero, false, false, carry);
    });
}

async fn execute_cb_rl_hl() {
    // RL (HL) - 4 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let old_val = memory_read(hl_addr).await;

    let carry_in = with_state(|state| state.cpu.flag_c());
    let (result, zero, carry) = rl_8bit(old_val, carry_in);

    // M3: Write result back to memory
    memory_write(hl_addr, result).await;

    with_state_mut(|state| {
        // RL sets Z=result==0, N=0, H=0, C=old_bit7
        state.cpu.set_flags(zero, false, false, carry);
    });
}

async fn execute_cb_rr_r(opcode: u8) {
    // RR r - 2 M-cycles total
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // RR (HL) - 4 M-cycles total
        execute_cb_rr_hl().await;
        return;
    }

    let (old_val, carry_in) = with_state(|state| {
        let val = match reg_bits {
            0b000 => state.cpu.b,
            0b001 => state.cpu.c,
            0b010 => state.cpu.d,
            0b011 => state.cpu.e,
            0b100 => state.cpu.h,
            0b101 => state.cpu.l,
            0b111 => state.cpu.a,
            _ => unreachable!(),
        };
        (val, state.cpu.flag_c())
    });

    let (result, zero, carry) = rr_8bit(old_val, carry_in);

    with_state_mut(|state| {
        match reg_bits {
            0b000 => state.cpu.b = result,
            0b001 => state.cpu.c = result,
            0b010 => state.cpu.d = result,
            0b011 => state.cpu.e = result,
            0b100 => state.cpu.h = result,
            0b101 => state.cpu.l = result,
            0b111 => state.cpu.a = result,
            _ => unreachable!(),
        }
        // RR sets Z=result==0, N=0, H=0, C=old_bit0
        state.cpu.set_flags(zero, false, false, carry);
    });
}

async fn execute_cb_rr_hl() {
    // RR (HL) - 4 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let old_val = memory_read(hl_addr).await;

    let carry_in = with_state(|state| state.cpu.flag_c());
    let (result, zero, carry) = rr_8bit(old_val, carry_in);

    // M3: Write result back to memory
    memory_write(hl_addr, result).await;

    with_state_mut(|state| {
        // RR sets Z=result==0, N=0, H=0, C=old_bit0
        state.cpu.set_flags(zero, false, false, carry);
    });
}

async fn execute_cb_sla_r(opcode: u8) {
    // SLA r - 2 M-cycles total
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // SLA (HL) - 4 M-cycles total
        execute_cb_sla_hl().await;
        return;
    }

    let old_val = with_state(|state| match reg_bits {
        0b000 => state.cpu.b,
        0b001 => state.cpu.c,
        0b010 => state.cpu.d,
        0b011 => state.cpu.e,
        0b100 => state.cpu.h,
        0b101 => state.cpu.l,
        0b111 => state.cpu.a,
        _ => unreachable!(),
    });

    let (result, zero, carry) = sla_8bit(old_val);

    with_state_mut(|state| {
        match reg_bits {
            0b000 => state.cpu.b = result,
            0b001 => state.cpu.c = result,
            0b010 => state.cpu.d = result,
            0b011 => state.cpu.e = result,
            0b100 => state.cpu.h = result,
            0b101 => state.cpu.l = result,
            0b111 => state.cpu.a = result,
            _ => unreachable!(),
        }
        // SLA sets Z=result==0, N=0, H=0, C=old_bit7
        state.cpu.set_flags(zero, false, false, carry);
    });
}

async fn execute_cb_sla_hl() {
    // SLA (HL) - 4 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let old_val = memory_read(hl_addr).await;

    let (result, zero, carry) = sla_8bit(old_val);

    // M3: Write result back to memory
    memory_write(hl_addr, result).await;

    with_state_mut(|state| {
        // SLA sets Z=result==0, N=0, H=0, C=old_bit7
        state.cpu.set_flags(zero, false, false, carry);
    });
}

async fn execute_cb_sra_r(opcode: u8) {
    // SRA r - 2 M-cycles total
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // SRA (HL) - 4 M-cycles total
        execute_cb_sra_hl().await;
        return;
    }

    let old_val = with_state(|state| match reg_bits {
        0b000 => state.cpu.b,
        0b001 => state.cpu.c,
        0b010 => state.cpu.d,
        0b011 => state.cpu.e,
        0b100 => state.cpu.h,
        0b101 => state.cpu.l,
        0b111 => state.cpu.a,
        _ => unreachable!(),
    });

    let (result, zero, carry) = sra_8bit(old_val);

    with_state_mut(|state| {
        match reg_bits {
            0b000 => state.cpu.b = result,
            0b001 => state.cpu.c = result,
            0b010 => state.cpu.d = result,
            0b011 => state.cpu.e = result,
            0b100 => state.cpu.h = result,
            0b101 => state.cpu.l = result,
            0b111 => state.cpu.a = result,
            _ => unreachable!(),
        }
        // SRA sets Z=result==0, N=0, H=0, C=old_bit0
        state.cpu.set_flags(zero, false, false, carry);
    });
}

async fn execute_cb_sra_hl() {
    // SRA (HL) - 4 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let old_val = memory_read(hl_addr).await;

    let (result, zero, carry) = sra_8bit(old_val);

    // M3: Write result back to memory
    memory_write(hl_addr, result).await;

    with_state_mut(|state| {
        // SRA sets Z=result==0, N=0, H=0, C=old_bit0
        state.cpu.set_flags(zero, false, false, carry);
    });
}

async fn execute_cb_swap_r(opcode: u8) {
    // SWAP r - 2 M-cycles total
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // SWAP (HL) - 4 M-cycles total
        execute_cb_swap_hl().await;
        return;
    }

    let old_val = with_state(|state| match reg_bits {
        0b000 => state.cpu.b,
        0b001 => state.cpu.c,
        0b010 => state.cpu.d,
        0b011 => state.cpu.e,
        0b100 => state.cpu.h,
        0b101 => state.cpu.l,
        0b111 => state.cpu.a,
        _ => unreachable!(),
    });

    let (result, zero) = swap_8bit(old_val);

    with_state_mut(|state| {
        match reg_bits {
            0b000 => state.cpu.b = result,
            0b001 => state.cpu.c = result,
            0b010 => state.cpu.d = result,
            0b011 => state.cpu.e = result,
            0b100 => state.cpu.h = result,
            0b101 => state.cpu.l = result,
            0b111 => state.cpu.a = result,
            _ => unreachable!(),
        }
        // SWAP sets Z=result==0, N=0, H=0, C=0
        state.cpu.set_flags(zero, false, false, false);
    });
}

async fn execute_cb_swap_hl() {
    // SWAP (HL) - 4 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let old_val = memory_read(hl_addr).await;

    let (result, zero) = swap_8bit(old_val);

    // M3: Write result back to memory
    memory_write(hl_addr, result).await;

    with_state_mut(|state| {
        // SWAP sets Z=result==0, N=0, H=0, C=0
        state.cpu.set_flags(zero, false, false, false);
    });
}

async fn execute_cb_srl_r(opcode: u8) {
    // SRL r - 2 M-cycles total
    let reg_bits = opcode & 0b111;

    if reg_bits == 0b110 {
        // SRL (HL) - 4 M-cycles total
        execute_cb_srl_hl().await;
        return;
    }

    let old_val = with_state(|state| match reg_bits {
        0b000 => state.cpu.b,
        0b001 => state.cpu.c,
        0b010 => state.cpu.d,
        0b011 => state.cpu.e,
        0b100 => state.cpu.h,
        0b101 => state.cpu.l,
        0b111 => state.cpu.a,
        _ => unreachable!(),
    });

    let (result, zero, carry) = srl_8bit(old_val);

    with_state_mut(|state| {
        match reg_bits {
            0b000 => state.cpu.b = result,
            0b001 => state.cpu.c = result,
            0b010 => state.cpu.d = result,
            0b011 => state.cpu.e = result,
            0b100 => state.cpu.h = result,
            0b101 => state.cpu.l = result,
            0b111 => state.cpu.a = result,
            _ => unreachable!(),
        }
        // SRL sets Z=result==0, N=0, H=0, C=old_bit0
        state.cpu.set_flags(zero, false, false, carry);
    });
}

async fn execute_cb_srl_hl() {
    // SRL (HL) - 4 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let old_val = memory_read(hl_addr).await;

    let (result, zero, carry) = srl_8bit(old_val);

    // M3: Write result back to memory
    memory_write(hl_addr, result).await;

    with_state_mut(|state| {
        // SRL sets Z=result==0, N=0, H=0, C=old_bit0
        state.cpu.set_flags(zero, false, false, carry);
    });
}

async fn execute_cb_bit_r(opcode: u8) {
    // BIT b, r - 2 M-cycles total
    let reg_bits = opcode & 0b111;
    let bit_pos = (opcode >> 3) & 0b111;

    if reg_bits == 0b110 {
        // BIT b, (HL) - 3 M-cycles total
        execute_cb_bit_hl(bit_pos).await;
        return;
    }

    let (reg_val, current_c) = with_state(|state| {
        let val = match reg_bits {
            0b000 => state.cpu.b,
            0b001 => state.cpu.c,
            0b010 => state.cpu.d,
            0b011 => state.cpu.e,
            0b100 => state.cpu.h,
            0b101 => state.cpu.l,
            0b111 => state.cpu.a,
            _ => unreachable!(),
        };
        (val, state.cpu.flag_c())
    });

    let bit_set = (reg_val & (1 << bit_pos)) != 0;

    with_state_mut(|state| {
        // BIT sets Z=!bit_set, N=0, H=1, preserves C
        state.cpu.set_flags(!bit_set, false, true, current_c);
    });
}

async fn execute_cb_bit_hl(bit_pos: u8) {
    // BIT b, (HL) - 3 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let reg_val = memory_read(hl_addr).await;

    let current_c = with_state(|state| state.cpu.flag_c());
    let bit_set = (reg_val & (1 << bit_pos)) != 0;

    with_state_mut(|state| {
        // BIT sets Z=!bit_set, N=0, H=1, preserves C
        state.cpu.set_flags(!bit_set, false, true, current_c);
    });
}

async fn execute_cb_res_r(opcode: u8) {
    // RES b, r - 2 M-cycles total
    let reg_bits = opcode & 0b111;
    let bit_pos = (opcode >> 3) & 0b111;

    if reg_bits == 0b110 {
        // RES b, (HL) - 4 M-cycles total
        execute_cb_res_hl(bit_pos).await;
        return;
    }

    let old_val = with_state(|state| match reg_bits {
        0b000 => state.cpu.b,
        0b001 => state.cpu.c,
        0b010 => state.cpu.d,
        0b011 => state.cpu.e,
        0b100 => state.cpu.h,
        0b101 => state.cpu.l,
        0b111 => state.cpu.a,
        _ => unreachable!(),
    });

    let result = old_val & !(1 << bit_pos); // Clear the bit

    with_state_mut(|state| {
        match reg_bits {
            0b000 => state.cpu.b = result,
            0b001 => state.cpu.c = result,
            0b010 => state.cpu.d = result,
            0b011 => state.cpu.e = result,
            0b100 => state.cpu.h = result,
            0b101 => state.cpu.l = result,
            0b111 => state.cpu.a = result,
            _ => unreachable!(),
        }
        // RES doesn't affect any flags
    });
}

async fn execute_cb_res_hl(bit_pos: u8) {
    // RES b, (HL) - 4 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let old_val = memory_read(hl_addr).await;

    let result = old_val & !(1 << bit_pos); // Clear the bit

    // M3: Write result back to memory
    memory_write(hl_addr, result).await;

    // RES doesn't affect any flags
}

async fn execute_cb_set_r(opcode: u8) {
    // SET b, r - 2 M-cycles total
    let reg_bits = opcode & 0b111;
    let bit_pos = (opcode >> 3) & 0b111;

    if reg_bits == 0b110 {
        // SET b, (HL) - 4 M-cycles total
        execute_cb_set_hl(bit_pos).await;
        return;
    }

    let old_val = with_state(|state| match reg_bits {
        0b000 => state.cpu.b,
        0b001 => state.cpu.c,
        0b010 => state.cpu.d,
        0b011 => state.cpu.e,
        0b100 => state.cpu.h,
        0b101 => state.cpu.l,
        0b111 => state.cpu.a,
        _ => unreachable!(),
    });

    let result = old_val | (1 << bit_pos); // Set the bit

    with_state_mut(|state| {
        match reg_bits {
            0b000 => state.cpu.b = result,
            0b001 => state.cpu.c = result,
            0b010 => state.cpu.d = result,
            0b011 => state.cpu.e = result,
            0b100 => state.cpu.h = result,
            0b101 => state.cpu.l = result,
            0b111 => state.cpu.a = result,
            _ => unreachable!(),
        }
        // SET doesn't affect any flags
    });
}

async fn execute_cb_set_hl(bit_pos: u8) {
    // SET b, (HL) - 4 M-cycles total
    // M2: Read from memory at HL
    let hl_addr = with_state(|state| state.cpu.hl());
    let old_val = memory_read(hl_addr).await;

    let result = old_val | (1 << bit_pos); // Set the bit

    // M3: Write result back to memory
    memory_write(hl_addr, result).await;

    // SET doesn't affect any flags
}

async fn execute_jp_nn() {
    // JP nn - 4 M-cycles total
    // M2: Read LSB of address
    let nn_lsb = fetch().await;
    // M3: Read MSB of address
    let nn_msb = fetch().await;
    // M4: Internal operation
    sleep(4).await;

    // Construct 16-bit address
    let nn = ((nn_msb as u16) << 8) | (nn_lsb as u16);

    with_state_mut(|state| {
        state.cpu.pc = nn;
    });
}

async fn execute_jp_hl() {
    // JP HL - 1 M-cycle total
    // Just set PC to HL value
    let hl = with_state(|state| state.cpu.hl());
    with_state_mut(|state| {
        state.cpu.pc = hl;
    });
}

async fn execute_jp_cc_nn(opcode: u8) {
    // JP cc, nn - 3 or 4 M-cycles total
    // M2: Read LSB of address
    let nn_lsb = fetch().await;
    // M3: Read MSB of address
    let nn_msb = fetch().await;

    // Construct 16-bit address
    let nn = ((nn_msb as u16) << 8) | (nn_lsb as u16);

    // Decode condition from opcode bits 4-3
    let condition_met = with_state(|state| {
        let cc_bits = (opcode >> 3) & 0b11;
        match cc_bits {
            0b00 => !state.cpu.flag_z(), // JP NZ, nn (0xC2)
            0b01 => state.cpu.flag_z(),  // JP Z, nn (0xCA)
            0b10 => !state.cpu.flag_c(), // JP NC, nn (0xD2)
            0b11 => state.cpu.flag_c(),  // JP C, nn (0xDA)
            _ => unreachable!(),
        }
    });

    if condition_met {
        // M4: Internal operation (extra cycle when jump taken)
        sleep(4).await;
        with_state_mut(|state| {
            state.cpu.pc = nn;
        });
    }
    // If condition not met, no extra cycle and PC stays as is
}

async fn execute_jr_e() {
    // JR e - 3 M-cycles total
    // M2: Read signed 8-bit offset
    let e = fetch().await;

    // M3: Internal operation
    sleep(4).await;

    let e_signed = e as i8 as i16; // Sign extend to 16-bit
    let current_pc = with_state(|state| state.cpu.pc);
    let new_pc = current_pc.wrapping_add(e_signed as u16);

    with_state_mut(|state| {
        state.cpu.pc = new_pc;
    });
}

async fn execute_jr_cc_e(opcode: u8) {
    // JR cc, e - 2 or 3 M-cycles total
    // M2: Read signed 8-bit offset
    let e = fetch().await;

    // Decode condition from opcode bits 4-3
    let condition_met = with_state(|state| {
        let cc_bits = (opcode >> 3) & 0b11;
        match cc_bits {
            0b00 => !state.cpu.flag_z(), // JR NZ, e (0x20)
            0b01 => state.cpu.flag_z(),  // JR Z, e (0x28)
            0b10 => !state.cpu.flag_c(), // JR NC, e (0x30)
            0b11 => state.cpu.flag_c(),  // JR C, e (0x38)
            _ => unreachable!(),
        }
    });

    if condition_met {
        // M3: Internal operation (extra cycle when jump taken)
        sleep(4).await;

        let e_signed = e as i8 as i16; // Sign extend to 16-bit
        let current_pc = with_state(|state| state.cpu.pc);
        let new_pc = current_pc.wrapping_add(e_signed as u16);

        with_state_mut(|state| {
            state.cpu.pc = new_pc;
        });
    }
    // If condition not met, no extra cycle and PC stays as is
}

async fn execute_call_nn() {
    // CALL nn - 6 M-cycles total
    // M2: Read LSB of address
    let nn_lsb = fetch().await;
    // M3: Read MSB of address
    let nn_msb = fetch().await;

    // Construct 16-bit address
    let nn = ((nn_msb as u16) << 8) | (nn_lsb as u16);

    // M4: Internal operation
    sleep(4).await;

    // M5: Push high byte of PC to stack
    let (pc, mut sp) = with_state(|state| (state.cpu.pc, state.cpu.sp));
    sp = sp.wrapping_sub(1);
    memory_write(sp, (pc >> 8) as u8).await;

    // M6: Push low byte of PC to stack
    sp = sp.wrapping_sub(1);
    memory_write(sp, (pc & 0xFF) as u8).await;

    with_state_mut(|state| {
        state.cpu.sp = sp;
        state.cpu.pc = nn;
    });
}

async fn execute_call_cc_nn(opcode: u8) {
    // CALL cc, nn - 3 or 6 M-cycles total
    // M2: Read LSB of address
    let nn_lsb = fetch().await;
    // M3: Read MSB of address
    let nn_msb = fetch().await;

    // Construct 16-bit address
    let nn = ((nn_msb as u16) << 8) | (nn_lsb as u16);

    // Decode condition from opcode bits 4-3
    let condition_met = with_state(|state| {
        let cc_bits = (opcode >> 3) & 0b11;
        match cc_bits {
            0b00 => !state.cpu.flag_z(), // CALL NZ, nn (0xC4)
            0b01 => state.cpu.flag_z(),  // CALL Z, nn (0xCC)
            0b10 => !state.cpu.flag_c(), // CALL NC, nn (0xD4)
            0b11 => state.cpu.flag_c(),  // CALL C, nn (0xDC)
            _ => unreachable!(),
        }
    });

    if condition_met {
        // M4: Internal operation
        sleep(4).await;

        // M5: Push high byte of PC to stack
        let (pc, mut sp) = with_state(|state| (state.cpu.pc, state.cpu.sp));
        sp = sp.wrapping_sub(1);
        memory_write(sp, (pc >> 8) as u8).await;

        // M6: Push low byte of PC to stack
        sp = sp.wrapping_sub(1);
        memory_write(sp, (pc & 0xFF) as u8).await;

        with_state_mut(|state| {
            state.cpu.sp = sp;
            state.cpu.pc = nn;
        });
    }
    // If condition not met, no extra cycles and PC stays as is
}

async fn execute_ret() {
    // RET - 4 M-cycles total
    // M2: Read low byte from stack
    let mut sp = with_state(|state| state.cpu.sp);
    let pc_lsb = memory_read(sp).await;
    sp = sp.wrapping_add(1);

    // M3: Read high byte from stack
    let pc_msb = memory_read(sp).await;
    sp = sp.wrapping_add(1);

    // M4: Internal operation
    sleep(4).await;

    // Construct 16-bit PC
    let pc = ((pc_msb as u16) << 8) | (pc_lsb as u16);

    with_state_mut(|state| {
        state.cpu.sp = sp;
        state.cpu.pc = pc;
    });

    // Enable interrupts (RETI specific behavior)
    interrupts::execute_reti();
}

async fn execute_ret_cc(opcode: u8) {
    // RET cc - 2 or 5 M-cycles total
    // M2: Internal operation
    sleep(4).await;

    // Decode condition from opcode bits 4-3
    let condition_met = with_state(|state| {
        let cc_bits = (opcode >> 3) & 0b11;
        match cc_bits {
            0b00 => !state.cpu.flag_z(), // RET NZ (0xC0)
            0b01 => state.cpu.flag_z(),  // RET Z (0xC8)
            0b10 => !state.cpu.flag_c(), // RET NC (0xD0)
            0b11 => state.cpu.flag_c(),  // RET C (0xD8)
            _ => unreachable!(),
        }
    });

    if condition_met {
        // M3: Read low byte from stack
        let mut sp = with_state(|state| state.cpu.sp);
        let pc_lsb = memory_read(sp).await;
        sp = sp.wrapping_add(1);

        // M4: Read high byte from stack
        let pc_msb = memory_read(sp).await;
        sp = sp.wrapping_add(1);

        // M5: Internal operation
        sleep(4).await;

        // Construct 16-bit PC
        let pc = ((pc_msb as u16) << 8) | (pc_lsb as u16);

        with_state_mut(|state| {
            state.cpu.sp = sp;
            state.cpu.pc = pc;
        });
    }
    // If condition not met, no extra cycles and PC stays as is
}

async fn execute_reti() {
    // RETI - 4 M-cycles total
    // M2: Read low byte from stack
    let mut sp = with_state(|state| state.cpu.sp);
    let pc_lsb = memory_read(sp).await;
    sp = sp.wrapping_add(1);

    // M3: Read high byte from stack
    let pc_msb = memory_read(sp).await;
    sp = sp.wrapping_add(1);

    // M4: Internal operation
    sleep(4).await;

    // Construct 16-bit PC
    let pc = ((pc_msb as u16) << 8) | (pc_lsb as u16);

    with_state_mut(|state| {
        state.cpu.sp = sp;
        state.cpu.pc = pc;
    });

    // Enable interrupts (RETI specific behavior)
    interrupts::execute_reti();
}

async fn execute_ei() {
    // EI - 1 M-cycle
    // Enable interrupts with delay
    interrupts::execute_ei();
}

async fn execute_di() {
    // DI - 1 M-cycle
    // Disable interrupts immediately
    interrupts::execute_di();
}

async fn execute_rst_n(opcode: u8) {
    // RST n - 4 M-cycles total
    // M2: Internal operation
    sleep(4).await;

    // M3: Push high byte of PC to stack
    let (pc, mut sp) = with_state(|state| (state.cpu.pc, state.cpu.sp));
    sp = sp.wrapping_sub(1);
    memory_write(sp, (pc >> 8) as u8).await;

    // M4: Push low byte of PC to stack
    sp = sp.wrapping_sub(1);
    memory_write(sp, (pc & 0xFF) as u8).await;

    // Decode restart address from opcode bits 5-3
    let rst_addr = (opcode >> 3) & 0b111;
    let target_addr = (rst_addr as u16) * 8; // RST addresses are 0x00, 0x08, 0x10, ..., 0x38

    with_state_mut(|state| {
        state.cpu.sp = sp;
        state.cpu.pc = target_addr;
    });
}

async fn execute_cb_instruction() {
    // CB prefix - fetch the actual CB instruction
    let cb_opcode = fetch().await;

    match cb_opcode {
        // RLC r (0x00-0x07)
        0x00..=0x07 => execute_cb_rlc_r(cb_opcode).await,
        // RRC r (0x08-0x0F)
        0x08..=0x0F => execute_cb_rrc_r(cb_opcode).await,
        // RL r (0x10-0x17)
        0x10..=0x17 => execute_cb_rl_r(cb_opcode).await,
        // RR r (0x18-0x1F)
        0x18..=0x1F => execute_cb_rr_r(cb_opcode).await,
        // SLA r (0x20-0x27)
        0x20..=0x27 => execute_cb_sla_r(cb_opcode).await,
        // SRA r (0x28-0x2F)
        0x28..=0x2F => execute_cb_sra_r(cb_opcode).await,
        // SWAP r (0x30-0x37)
        0x30..=0x37 => execute_cb_swap_r(cb_opcode).await,
        // SRL r (0x38-0x3F)
        0x38..=0x3F => execute_cb_srl_r(cb_opcode).await,
        // BIT b, r (0x40-0x7F)
        0x40..=0x7F => execute_cb_bit_r(cb_opcode).await,
        // RES b, r (0x80-0xBF)
        0x80..=0xBF => execute_cb_res_r(cb_opcode).await,
        // SET b, r (0xC0-0xFF)
        0xC0..=0xFF => execute_cb_set_r(cb_opcode).await,
        _ => {
            println!("Unimplemented CB instruction: 0x{:02X}", cb_opcode);
            sleep(4).await;
        }
    }
}

// System control instructions
async fn execute_nop() {
    // NOP - 1 M-cycle total
    // Does nothing but consume one cycle (already consumed by fetch)
}

async fn execute_halt() {
    // HALT - Variable timing based on interrupt state
    let (ime, pending_interrupts) = with_state(|state| {
        let ie = state.interrupts.ie;
        let if_reg = state.interrupts.if_reg;
        (state.interrupts.ime, ie & if_reg)
    });

    if ime {
        // IME = 1: Normal halt behavior
        // Wait until an interrupt is pending, then it will be handled normally
        loop {
            sleep(4).await; // Wait one cycle
            let pending = with_state(|state| state.interrupts.ie & state.interrupts.if_reg);
            if pending != 0 {
                break; // Exit halt when interrupt is pending
            }
        }
    } else {
        // IME = 0: Special halt behavior
        if pending_interrupts != 0 {
            // HALT Bug: If interrupt is already pending when HALT executes with IME=0,
            // the halt exits immediately but PC fails to increment properly.
            // This causes the next byte to be read twice.

            // Decrement PC to simulate the halt bug (next instruction will be read twice)
            with_state_mut(|state| {
                state.cpu.pc = state.cpu.pc.wrapping_sub(1);
            });
        } else {
            // No interrupts pending: wait until one becomes pending, but don't handle it
            loop {
                sleep(4).await; // Wait one cycle
                let pending = with_state(|state| state.interrupts.ie & state.interrupts.if_reg);
                if pending != 0 {
                    break; // Exit halt when interrupt becomes pending
                }
            }
        }
    }
}

async fn execute_stop() {
    // STOP - Stops the system
    // For our emulator, we'll just halt indefinitely
    // In a real Game Boy, this stops the main clock and puts the system in low power mode
    loop {
        sleep(4).await; // Sleep forever
    }
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
        0x36 => execute_ld_hl_n().await,     // LD (HL), n
        0x0A => execute_ld_a_bc().await,     // LD A, (BC)
        0x1A => execute_ld_a_de().await,     // LD A, (DE)
        0x02 => execute_ld_bc_a().await,     // LD (BC), A
        0x12 => execute_ld_de_a().await,     // LD (DE), A
        0xFA => execute_ld_a_nn().await,     // LD A, (nn)
        0xEA => execute_ld_nn_a().await,     // LD (nn), A
        0xF2 => execute_ldh_a_c().await,     // LDH A, (C)
        0xE2 => execute_ldh_c_a().await,     // LDH (C), A
        0xF0 => execute_ldh_a_n().await,     // LDH A, (n)
        0xE0 => execute_ldh_n_a().await,     // LDH (n), A
        0x3A => execute_ld_a_hl_dec().await, // LD A, (HL-)
        0x32 => execute_ld_hl_dec_a().await, // LD (HL-), A
        0x2A => execute_ld_a_hl_inc().await, // LD A, (HL+)
        0x22 => execute_ld_hl_inc_a().await, // LD (HL+), A
        // 16-bit load instructions
        0x01 | 0x11 | 0x21 | 0x31 => execute_ld_rr_nn(opcode).await, // LD rr, nn
        0x08 => execute_ld_nn_sp().await,                            // LD (nn), SP
        0xF9 => execute_ld_sp_hl().await,                            // LD SP, HL
        0xC5 | 0xD5 | 0xE5 | 0xF5 => execute_push_rr(opcode).await,  // PUSH rr
        0xC1 | 0xD1 | 0xE1 | 0xF1 => execute_pop_rr(opcode).await,   // POP rr
        0xF8 => execute_ld_hl_sp_e().await,                          // LD HL, SP+e
        // 8-bit arithmetic instructions
        0x80..=0x87 => execute_add_r(opcode).await, // ADD r (includes ADD (HL) at 0x86)
        0xC6 => execute_add_n().await,              // ADD n
        0x88..=0x8F => execute_adc_r(opcode).await, // ADC r (includes ADC (HL) at 0x8E)
        0xCE => execute_adc_n().await,              // ADC n
        0x90..=0x97 => execute_sub_r(opcode).await, // SUB r (includes SUB (HL) at 0x96)
        0xD6 => execute_sub_n().await,              // SUB n
        0x98..=0x9F => execute_sbc_r(opcode).await, // SBC r (includes SBC (HL) at 0x9E)
        0xDE => execute_sbc_n().await,              // SBC n
        // Logical instructions (need to handle overlaps carefully)
        0xA0..=0xA7 => execute_and_r(opcode).await, // AND r (includes AND (HL) at 0xA6)
        0xE6 => execute_and_n().await,              // AND n
        0xA8..=0xAF => execute_xor_r(opcode).await, // XOR r (includes XOR (HL) at 0xAE)
        0xEE => execute_xor_n().await,              // XOR n
        0xB0..=0xB7 => execute_or_r(opcode).await,  // OR r (includes OR (HL) at 0xB6)
        0xF6 => execute_or_n().await,               // OR n
        0xB8..=0xBF => execute_cp_r(opcode).await,  // CP r (includes CP (HL) at 0xBE)
        0xFE => execute_cp_n().await,               // CP n
        // Increment/Decrement instructions
        0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x3C => execute_inc_r(opcode).await, // INC r
        0x34 => execute_inc_hl().await,                                                // INC (HL)
        0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x3D => execute_dec_r(opcode).await, // DEC r
        0x35 => execute_dec_hl().await,                                                // DEC (HL)
        // Flag manipulation instructions
        0x3F => execute_ccf().await, // CCF
        0x37 => execute_scf().await, // SCF
        0x27 => execute_daa().await, // DAA
        0x2F => execute_cpl().await, // CPL
        // 16-bit arithmetic instructions
        0x03 | 0x13 | 0x23 | 0x33 => execute_inc_rr(opcode).await, // INC rr
        0x0B | 0x1B | 0x2B | 0x3B => execute_dec_rr(opcode).await, // DEC rr
        0x09 | 0x19 | 0x29 | 0x39 => execute_add_hl_rr(opcode).await, // ADD HL,rr
        0xE8 => execute_add_sp_e().await,                          // ADD SP,e
        // Rotate/shift instructions (accumulator)
        0x07 => execute_rlca().await, // RLCA
        0x0F => execute_rrca().await, // RRCA
        0x17 => execute_rla().await,  // RLA
        0x1F => execute_rra().await,  // RRA
        // CB-prefixed instructions
        0xCB => execute_cb_instruction().await, // CB prefix
        // Control flow instructions
        0xC3 => execute_jp_nn().await, // JP nn
        0xE9 => execute_jp_hl().await, // JP HL
        0xC2 | 0xCA | 0xD2 | 0xDA => execute_jp_cc_nn(opcode).await, // JP cc, nn
        0x18 => execute_jr_e().await,  // JR e
        0x20 | 0x28 | 0x30 | 0x38 => execute_jr_cc_e(opcode).await, // JR cc, e
        0xCD => execute_call_nn().await, // CALL nn
        0xC4 | 0xCC | 0xD4 | 0xDC => execute_call_cc_nn(opcode).await, // CALL cc, nn
        0xC9 => execute_ret().await,   // RET
        0xC0 | 0xC8 | 0xD0 | 0xD8 => execute_ret_cc(opcode).await, // RET cc
        0xD9 => execute_reti().await,  // RETI
        0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => execute_rst_n(opcode).await, // RST n
        // Interrupt control
        0xF3 => execute_di().await, // DI
        0xFB => execute_ei().await, // EI
        // System control
        0x00 => execute_nop().await,  // NOP
        0x76 => execute_halt().await, // HALT
        0x10 => execute_stop().await, // STOP
        _ => {
            println!("Unimplemented instruction: 0x{:02X}", opcode);
            // For now, just sleep to avoid infinite loop
            sleep(4).await;
        }
    }
}

pub async fn cpu() {
    loop {
        // Check for pending interrupts before executing instruction
        if let Some(interrupt) = interrupts::check_interrupts() {
            interrupts::handle_interrupt(interrupt).await;
            continue;
        }

        // Fetch and execute instruction
        let instruction = fetch().await;
        decode_and_execute(instruction).await;

        // Process EI delay after instruction execution
        interrupts::process_ei_delay();
    }
}
