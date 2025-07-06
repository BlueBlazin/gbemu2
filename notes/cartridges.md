## The Cartridge Header

Each cartridge contains a header, located at the address range $0100—$014F. The cartridge header provides the following information about the game itself and the hardware it expects to run on
The bytes which are important:

### 0x147 — Cartridge type

- This byte indicates what kind of hardware is present on the cartridge and its mapper
  - 0x00 | 0x08 | 0x09 => Mbc 0
  - 0x01..=0x03 => Mbc 1
  - 0x0F..=0x13 => Mbc 3
  - 0x19..=0x1E => Mbc 5

### 0x148 — ROM size

- This byte indicates how much ROM is present on the cartridge. In most cases, the ROM size is given by 32 KiB × (1 << value):

| Value | ROM size | Number of ROM banks |
| ----- | -------- | ------------------- |
| $00   | 32 KiB   | 2 (no banking)      |
| $01   | 64 KiB   | 4                   |
| $02   | 128 KiB  | 8                   |
| $03   | 256 KiB  | 16                  |
| $04   | 512 KiB  | 32                  |
| $05   | 1 MiB    | 64                  |
| $06   | 2 MiB    | 128                 |
| $07   | 4 MiB    | 256                 |
| $08   | 8 MiB    | 512                 |

### 0149 — RAM size

- This byte indicates how much RAM is present on the cartridge, if any
- If the cartridge type does not include “RAM” in its name, this should be set to 0

| Code | SRAM size | Comment                |
| ---- | --------- | ---------------------- |
| $00  | 0         | No RAM                 |
| $01  | –         | Unused 12              |
| $02  | 8 KiB     | 1 bank                 |
| $03  | 32 KiB    | 4 banks of 8 KiB each  |
| $04  | 128 KiB   | 16 banks of 8 KiB each |
| $05  | 64 KiB    | 8 banks of 8 KiB each  |

## MBC 0

- Small games of not more than 32 KiB ROM do not require a MBC chip for ROM banking
- The ROM is directly mapped to memory at $0000-7FFF

## MBC 1

- MBC1 supports up to 512 KiB ROM with up to 32 KiB of banked RAM

### Memory

#### 0000–3FFF — ROM Bank X0 \[read-only\]

- This area normally contains the first 16 KiB (bank 00) of the cartridge ROM

#### 4000–7FFF — ROM Bank 01-7F \[read-only\]

- This area may contain any of the further 16 KiB banks of the ROM
- If the main 5-bit ROM banking register is 0, it reads the bank as if it was set to 1

#### A000–BFFF — RAM Bank 00–03, if any

- This area is used to address external RAM in the cartridge (if any). The RAM is only accessible if RAM is enabled, otherwise reads return open bus values (often $FF, but not guaranteed) and writes are ignored
- Available RAM sizes are 8 KiB (at $A000–BFFF) and 32 KiB (in form of four 8K banks at $A000–BFFF). 32 KiB is only available in cartridges with ROM <= 512 KiB

### Registers

- All of the MBC1 registers default to $00 on power-up, which for the “ROM Bank Number” register is treated as $01

#### 0000–1FFF — RAM Enable (Write Only)

#### 2000–3FFF — ROM Bank Number (Write Only)

- This 5-bit register (range $01-$1F) selects the ROM bank number for the 4000–7FFF region. Higher bits are discarded
- If the ROM Bank Number is set to a higher value than the number of banks in the cart, the bank number is masked to the required number of bits. e.g. a 256 KiB cart only needs a 4-bit bank number to address all of its 16 banks, so this register is masked to 4 bits. The upper bit would be ignored for bank selection

#### 4000–5FFF — RAM Bank Number — or — Upper Bits of ROM Bank Number (Write Only)

- This second 2-bit register can be used to select a RAM Bank in range from $00–$03 (32 KiB ram carts only), or to specify the upper two bits (bits 5-6) of the ROM Bank number (1 MiB ROM or larger carts only). If neither ROM nor RAM is large enough, setting this register does nothing

#### 6000–7FFF — Banking Mode Select (Write Only)

- This 1-bit register selects between the two MBC1 banking modes, controlling the behaviour of the secondary 2-bit banking register (above)

Example implementation from another Rust emulator:

```rs
const RAM_OFFSET: usize = 0xA000;
const ROM_OFFSET: usize = 0x4000;
const RAM_BANK_SIZE: usize = 0x2000;
const ROM_BANK_SIZE: usize = 0x4000;

#[derive(PartialEq)]
enum Mode {
    Mode0,
    Mode1,
}

pub struct Mbc1 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    ram_enabled: bool,
    mode: Mode,

    bank1: u8,
    bank2: u8,

    max_banks: u8,
    size: usize,
}

impl Mbc1 {
    pub fn new(data: Vec<u8>) -> Self {
        let size = data.len();

        let max_banks = 2u32.pow(data[0x148] as u32) as u8;

        let ram_size = match data[0x0149] {
            1 => 0x800,
            2 => 0x2000,
            3 => 0x8000,
            4 => 0x20000,
            _ => 0x800,
        };

        Mbc1 {
            rom: data,
            ram: vec![0; ram_size],
            ram_enabled: false,
            mode: Mode::Mode0,

            bank1: 1,
            bank2: 0,

            max_banks,
            size,
        }
    }
}

impl Mbc for Mbc1 {
    fn get_byte(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => match self.mode {
                Mode::Mode0 => self.rom[addr as usize],
                Mode::Mode1 => {
                    let bank = self.bank2 << 5;
                    let addr = bank as usize * ROM_BANK_SIZE + addr as usize;
                    self.rom[addr]
                }
            },
            0x4000..=0x7FFF => {
                let bank = (self.bank2 << 5) | self.bank1;

                let addr = bank as usize * ROM_BANK_SIZE + (addr as usize - ROM_OFFSET);

                self.rom[addr % self.size]
            }
            0xA000..=0xBFFF => {
                if !self.ram_enabled {
                    return 0xFF;
                }

                let bank = match self.mode {
                    Mode::Mode0 => 0x0,
                    Mode::Mode1 => self.bank2,
                };
                let addr = bank as usize * RAM_BANK_SIZE + (addr as usize - RAM_OFFSET);
                self.ram[addr]
            }
            _ => panic!("Address out of bounds."),
        }
    }

    fn set_byte(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enabled = (value & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                self.bank1 = match value & 0x1F {
                    0 => 1,
                    n => n,
                };
            }
            0x4000..=0x5FFF => {
                self.bank2 = value & 0x3;
            }
            0x6000..=0x7FFF => {
                self.mode = match value & 0x1 {
                    0 => Mode::Mode0,
                    _ => Mode::Mode1,
                };
            }
            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    let bank = match self.mode {
                        Mode::Mode0 => 0x0,
                        Mode::Mode1 => self.bank2,
                    };
                    let addr = bank as usize * RAM_BANK_SIZE + (addr as usize - RAM_OFFSET);
                    self.ram[addr] = value;
                }
            }
            _ => panic!("Address out of bounds."),
        }
    }
}
```

## MBC 1

### MBC1 registers

- The MBC1 chip includes four registers that affect the behaviour of the chip
- All registers are smaller than 8 bits, and unused bits are simply ignored during writes
- The registers are not directly readable

#### Register 12.1: 0x0000-0x1FFF - RAMG - MBC1 RAM gate register

- bit 7-4 Unimplemented: Ignored during writes
- bit 3-0 RAMG<3:0>: RAM gate register

  - 0b1010 = enable access to chip RAM
  - All other values disable access to chip RAM

- The RAMG register is used to enable access to the cartridge SRAM if one exists
- RAM access is disabled by default but can be enabled by writing to the 0x0000-0x1FFF address range a value with the bit pattern 0b1010 in the lower nibble
- Upper bits don’t matter, but any other bit pattern in the lower nibble disables access to RAM
- When RAM access is disabled, all writes to the external RAM area 0xA000-0xBFFF are ignored, and reads return 0xFF

#### Register 12.2: 0x2000-0x3FFF - BANK1 - MBC1 bank register 1

- bit 7-5 Unimplemented: Ignored during writes
- bit 4-0 BANK1<4:0>: Bank register 1

  - Never contains the value 0b00000
  - If 0b00000 is written, the resulting value will be 0b00001 instead

- The 5-bit BANK1 register is used as the lower 5 bits of the ROM bank number when the CPU accesses the 0x4000-0x7FFF memory area
- MBC1 doesn’t allow the BANK1 register to contain zero (bit pattern 0b00000), so the initial value at reset is 0b00001 and attempting to write 0b00000 will write 0b00001 instead
- This makes it impossible to read banks 0x00, 0x20, 0x40 and 0x60 from the 0x4000-0x7FFF memory area, because those bank numbers have 0b00000 in the lower bits
- Due to the zero value adjustment, requesting any of these banks actually requests the next bank (e.g. 0x21 instead of 0x20)

#### Register 12.3: 0x2000-0x3FFF - BANK1 - MBC1 bank register 2

- bit 7-2 Unimplemented: Ignored during writes
- bit 1-0 BANK2<1:0>: Bank register 2
- The 2-bit BANK2 register can be used as the upper bits of the ROM bank number, or as the 2-bit RAM bank number
- Unlike BANK1, BANK2 doesn’t disallow zero, so all 2-bit values are possible

#### Register 12.4: 0x6000-0x7FFF - MODE - MBC1 mode register

- bit 7-1 Unimplemented: Ignored during writes
- bit 0 MODE: Mode register
  - 0b1 = BANK2 affects accesses to 0x0000-0x3FFF, 0x4000-0x7FFF, 0xA000-0xBFFF
  - 0b0 = BANK2 affects only accesses to 0x4000-0x7FFF
- The MODE register determines how the BANK2 register value is used during memory accesses.

#### 12.2 ROM in the 0x0000-0x7FFF area

- When the 0x0000-0x3FFF address range is accessed, the effective bank number depends on the MODE register
- In MODE 0b0 the bank number is always 0, but in MODE 0b1 it’s formed by shifting the BANK2 register value left by 5 bits.
- When the 0x4000-0x7FFF addess range is accessed, the effective bank number is always a combination of BANK1 and BANK2 register values
- If the cartridge ROM is smaller than 16 Mbit, there are less ROM address pins to connect to and therefore some bank number bits are ignored

#### 12.3 RAM in the 0xA000-0xBFFF area

- Some MBC1 carts include SRAM, which is mapped to the 0xA000-0xBFFF area
- If no RAM is present, or RAM is not enabled with the RAMG register, all reads return undefined values and writes have no effect
- In MODE 0b0 the BANK2 register value is not used, so the first RAM bank is always mapped to the 0xA000-0xBFFF area
- In MODE 0b1 the BANK2 register value is used as the bank number
