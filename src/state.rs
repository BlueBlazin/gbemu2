use crate::interrupts::InterruptState;
use crate::mmu::MMU;
use crate::ppu::PpuState;

#[derive(Debug, Clone)]
pub struct CpuState {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
}

impl CpuState {
    pub fn new() -> Self {
        Self {
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
        }
    }

    // 16-bit register getters
    pub fn af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16)
    }

    pub fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    pub fn de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    pub fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    // 16-bit register setters
    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = (value & 0xF0) as u8; // Lower 4 bits of F are always 0
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = (value & 0xFF) as u8;
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = (value & 0xFF) as u8;
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = (value & 0xFF) as u8;
    }

    // Flag operations
    pub fn flag_z(&self) -> bool {
        (self.f & 0x80) != 0
    }

    pub fn flag_n(&self) -> bool {
        (self.f & 0x40) != 0
    }

    pub fn flag_h(&self) -> bool {
        (self.f & 0x20) != 0
    }

    pub fn flag_c(&self) -> bool {
        (self.f & 0x10) != 0
    }

    pub fn set_flag_z(&mut self, value: bool) {
        if value {
            self.f |= 0x80;
        } else {
            self.f &= !0x80;
        }
    }

    pub fn set_flag_n(&mut self, value: bool) {
        if value {
            self.f |= 0x40;
        } else {
            self.f &= !0x40;
        }
    }

    pub fn set_flag_h(&mut self, value: bool) {
        if value {
            self.f |= 0x20;
        } else {
            self.f &= !0x20;
        }
    }

    pub fn set_flag_c(&mut self, value: bool) {
        if value {
            self.f |= 0x10;
        } else {
            self.f &= !0x10;
        }
    }

    pub fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.f = 0;
        if z {
            self.f |= 0x80;
        }
        if n {
            self.f |= 0x40;
        }
        if h {
            self.f |= 0x20;
        }
        if c {
            self.f |= 0x10;
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmulatorState {
    pub cpu: CpuState,
    pub mmu: MMU,
    pub ppu: PpuState,
    pub interrupts: InterruptState,

    // Legacy memory array for backward compatibility (TODO: remove after updating CPU)
    pub memory: [u8; 0x10000],
}

impl EmulatorState {
    pub fn new() -> Self {
        Self {
            cpu: CpuState::new(),
            mmu: MMU::new(),
            ppu: PpuState::new(),
            interrupts: InterruptState::new(),
            memory: [0; 0x10000],
        }
    }

    // Helper methods to access memory through the MMU
    pub fn read_memory(&self, address: u16) -> u8 {
        self.mmu.read(address)
    }

    pub fn write_memory(&mut self, address: u16, value: u8) {
        self.mmu.write(address, value);
        // Also update legacy array for now (TODO: remove after CPU migration)
        self.memory[address as usize] = value;
    }
}
