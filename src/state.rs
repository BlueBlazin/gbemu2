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
pub struct MemoryMap {
    // ROM Banks (0x0000-0x7FFF)
    pub rom_bank_00: [u8; 0x4000], // 0x0000-0x3FFF: Fixed bank
    pub rom_bank_nn: [u8; 0x4000], // 0x4000-0x7FFF: Switchable bank (TODO: implement banking)

    // Video RAM (0x8000-0x9FFF)
    pub vram: [u8; 0x2000], // 8 KiB Video RAM (TODO: connect to PPU)

    // External RAM (0xA000-0xBFFF)
    pub external_ram: [u8; 0x2000], // 8 KiB from cartridge (TODO: implement banking)

    // Work RAM (0xC000-0xDFFF)
    pub wram: [u8; 0x2000], // 8 KiB Work RAM

    // Object Attribute Memory (0xFE00-0xFE9F)
    pub oam: [u8; 0xA0], // 160 bytes OAM (TODO: connect to PPU)

    // I/O Registers (0xFF00-0xFF7F)
    pub io_registers: [u8; 0x80], // 128 bytes I/O (TODO: implement proper I/O handling)

    // High RAM (0xFF80-0xFFFE)
    pub hram: [u8; 0x7F], // 127 bytes High RAM

    // Interrupt Enable Register (0xFFFF)
    pub ie_register: u8, // Interrupt Enable (TODO: implement interrupts)
}

impl MemoryMap {
    pub fn new() -> Self {
        Self {
            rom_bank_00: [0; 0x4000],
            rom_bank_nn: [0; 0x4000],
            vram: [0; 0x2000],
            external_ram: [0; 0x2000],
            wram: [0; 0x2000],
            oam: [0; 0xA0],
            io_registers: [0; 0x80],
            hram: [0; 0x7F],
            ie_register: 0,
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            // ROM Bank 00 (0x0000-0x3FFF)
            0x0000..=0x3FFF => self.rom_bank_00[(address & 0x3FFF) as usize],

            // ROM Bank NN (0x4000-0x7FFF) - TODO: implement banking
            0x4000..=0x7FFF => self.rom_bank_nn[(address & 0x3FFF) as usize],

            // Video RAM (0x8000-0x9FFF) - TODO: connect to PPU
            0x8000..=0x9FFF => self.vram[(address & 0x1FFF) as usize],

            // External RAM (0xA000-0xBFFF) - TODO: implement banking
            0xA000..=0xBFFF => self.external_ram[(address & 0x1FFF) as usize],

            // Work RAM (0xC000-0xDFFF)
            0xC000..=0xDFFF => self.wram[(address & 0x1FFF) as usize],

            // Echo RAM (0xE000-0xFDFF) - mirror of 0xC000-0xDDFF
            0xE000..=0xFDFF => {
                let mirrored_addr = address - 0x2000;
                self.wram[(mirrored_addr & 0x1FFF) as usize]
            }

            // Object Attribute Memory (0xFE00-0xFE9F) - TODO: connect to PPU
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize],

            // Not Usable (0xFEA0-0xFEFF) - return 0xFF
            0xFEA0..=0xFEFF => 0xFF,

            // I/O Registers (0xFF00-0xFF7F) - TODO: implement proper I/O handling
            0xFF00..=0xFF7F => self.io_registers[(address - 0xFF00) as usize],

            // High RAM (0xFF80-0xFFFE)
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize],

            // Interrupt Enable Register (0xFFFF) - TODO: implement interrupts
            0xFFFF => self.ie_register,
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            // ROM Banks (0x0000-0x7FFF) - TODO: implement mapper control
            0x0000..=0x7FFF => {
                // ROM writes typically control banking, not actual ROM data
                // TODO: implement cartridge mapper handling
            }

            // Video RAM (0x8000-0x9FFF) - TODO: connect to PPU
            0x8000..=0x9FFF => {
                self.vram[(address & 0x1FFF) as usize] = value;
                // TODO: notify PPU of VRAM changes
            }

            // External RAM (0xA000-0xBFFF) - TODO: implement banking
            0xA000..=0xBFFF => {
                self.external_ram[(address & 0x1FFF) as usize] = value;
            }

            // Work RAM (0xC000-0xDFFF)
            0xC000..=0xDFFF => {
                self.wram[(address & 0x1FFF) as usize] = value;
            }

            // Echo RAM (0xE000-0xFDFF) - mirror of 0xC000-0xDDFF
            0xE000..=0xFDFF => {
                let mirrored_addr = address - 0x2000;
                self.wram[(mirrored_addr & 0x1FFF) as usize] = value;
            }

            // Object Attribute Memory (0xFE00-0xFE9F) - TODO: connect to PPU
            0xFE00..=0xFE9F => {
                self.oam[(address - 0xFE00) as usize] = value;
                // TODO: notify PPU of OAM changes
            }

            // Not Usable (0xFEA0-0xFEFF) - ignore writes
            0xFEA0..=0xFEFF => {}

            // I/O Registers (0xFF00-0xFF7F) - TODO: implement proper I/O handling
            0xFF00..=0xFF7F => {
                self.io_registers[(address - 0xFF00) as usize] = value;
                // TODO: handle specific I/O register writes (joypad, timer, LCD, etc.)
            }

            // High RAM (0xFF80-0xFFFE)
            0xFF80..=0xFFFE => {
                self.hram[(address - 0xFF80) as usize] = value;
            }

            // Interrupt Enable Register (0xFFFF) - TODO: implement interrupts
            0xFFFF => {
                self.ie_register = value;
                // TODO: handle interrupt enable changes
            }
        }
    }

    // Helper method to load ROM data (TODO: extend for proper cartridge handling)
    pub fn load_rom(&mut self, rom_data: &[u8]) {
        // Load first 16KB into bank 00
        let bank_00_size = std::cmp::min(rom_data.len(), 0x4000);
        self.rom_bank_00[..bank_00_size].copy_from_slice(&rom_data[..bank_00_size]);

        // Load next 16KB into bank NN if available
        if rom_data.len() > 0x4000 {
            let bank_nn_size = std::cmp::min(rom_data.len() - 0x4000, 0x4000);
            self.rom_bank_nn[..bank_nn_size]
                .copy_from_slice(&rom_data[0x4000..0x4000 + bank_nn_size]);
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmulatorState {
    pub cpu: CpuState,
    pub memory_map: MemoryMap,

    // Legacy memory array for backward compatibility (TODO: remove after updating CPU)
    pub memory: [u8; 0x10000],
}

impl EmulatorState {
    pub fn new() -> Self {
        Self {
            cpu: CpuState::new(),
            memory_map: MemoryMap::new(),
            memory: [0; 0x10000],
        }
    }

    // Helper methods to access memory through the memory map
    pub fn read_memory(&self, address: u16) -> u8 {
        self.memory_map.read(address)
    }

    pub fn write_memory(&mut self, address: u16, value: u8) {
        self.memory_map.write(address, value);
        // Also update legacy array for now (TODO: remove after CPU migration)
        self.memory[address as usize] = value;
    }
}
