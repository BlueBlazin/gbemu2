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

## MBC3

(max 2MByte ROM and/or 32KByte RAM and Timer)

- Beside for the ability to access up to 2MB ROM (128 banks), and 32KB RAM (4 banks), the MBC3 also includes a built-in Real Time Clock (RTC)

### Memory

#### 0000-3FFF - ROM Bank 00 (Read Only)

- Contains the first 16 KiB of the ROM

#### 4000-7FFF - ROM Bank 01-7F (Read Only)

- This area may contain any of the further 16 KiB banks of the ROM
- If the main 5-bit ROM banking register is 0, it reads the bank as if it was set to 1
- Same as for MBC1, except that accessing banks $20, $40, and $60 is supported now

#### A000-BFFF - RAM Bank 00-07 or RTC register (Read/Write)

- Depending on the current Bank Number/RTC Register selection, this memory space is used to access an 8 KiB external RAM Bank, or a single RTC Register

### Registers

#### A000-BFFF - RTC Register 08-0C (Read/Write)

- Depending on the current Bank Number/RTC Register selection (see below), this memory space is used to access an 8KByte external RAM Bank, or a single RTC Register
- The mapped RTC register can be read/written by accessing any address in that area, typically using address A000

#### 0000-1FFF - RAM and Timer Enable (Write Only)

- Mostly the same as for MBC1, a value of $0A will enable reading and writing to external RAM - and to the RTC Registers! A value of $00 will disable either

#### 2000-3FFF - ROM Bank Number (Write Only)

- Same as for MBC1, except that the whole 7 bits of the ROM Bank Number are written directly to this address
- As for the MBC1, writing a value of $00 will select Bank $01 instead. All other values $01-$7F select the corresponding ROM Banks

#### 4000-5FFF - RAM Bank Number - or - RTC Register Select (Write Only)

- Controls what is mapped into memory at A000-BFFF

| Value   | Selection                                   |
| ------- | ------------------------------------------- |
| $00-$07 | The corresponding RAM Bank.                 |
| $08-$0C | The corresponding RTC Register (see below). |

#### 6000-7FFF - Latch Clock Data (Write Only)

- When writing $00, and then $01 to this register, the current time becomes latched into the RTC registers
- The latched data will not change until it becomes latched again, by repeating the write $00->$01 procedure
- This provides a way to read the RTC registers while the clock keeps ticking

#### Clock Counter Registers

| Register | Name   | Description                                                                                                                                                                                   | Range          |
| -------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| $08      | RTC S  | Seconds                                                                                                                                                                                       | 0-59 ($00-$3B) |
| $09      | RTC M  | Minutes                                                                                                                                                                                       | 0-59 ($00-$3B) |
| $0A      | RTC H  | Hours                                                                                                                                                                                         | 0-23 ($00-$17) |
| $0B      | RTC DL | Lower 8 bits of Day Counter                                                                                                                                                                   | ($00-$FF)      |
| $0C      | RTC DH | Upper 1 bit of Day Counter, Carry Bit, Halt Flag. Bit 0: Most significant bit (Bit 8) of Day Counter. Bit 6: Halt (0=Active, 1=Stop Timer). Bit 7: Day Counter Carry Bit (1=Counter Overflow) | -              |

- The Halt Flag is supposed to be set before writing to the RTC Registers

#### The Day Counter

- The total 9 bits of the Day Counter allow counting days in range from 0-511 ($000-$1FF)
- The Day Counter Carry Bit becomes set when this value overflows. In that case the Carry Bit remains set until the program does reset it. Note that you can store an offset to the Day Counter in battery RAM
- For example, every time you read a non-zero Day Counter, add this Counter to the offset in RAM, and reset the Counter to zero
- This method allows counting any number of days, making your program Year-10000-Proof, provided that the cartridge gets used at least every 511 days
