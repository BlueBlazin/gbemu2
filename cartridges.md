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
