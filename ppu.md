# PPU Notes

I am breaking the PPU down into two parts. The PPU data and the PPU operation.

## PPU Data

### Tile Data

- Stored in VRAM at $8000-$97FF
- Each tile takes 16 bytes total 384 tiles
- Each tile (or character) has 8×8 pixels and has a color depth of 2 bits per pixel, allowing each pixel to use one of 4 colors or gray shades
- Tiles can be part of bg/window maps and/or as objects (movable sprites)
- Color 0 has special meaning in objects - it's transparent allowing bg or other objs behind to show through

Three "blocks" of 128 tiles:
Tile IDs for... | Block 0 | Block 1 | Block 2
--- | --- | --- | ---
$8000–87FF | $8800–8FFF | $9000–97FF
Objects 0–127|128–255|—
BG/Win, if LCDC.4=1|0–127|128–255|—
BG/Win, if LCDC.4=0|—|128–255|0–127
3

#### Tile Indexing

Always indexed using an 8-bit integer but addressing method may differ:

- $8000 method: uses $8000 as its base pointer and uses an unsigned addressing, meaning that tiles 0-127 are in block 0, and tiles 128-255 are in block 1
- $8800 method: uses $9000 as its base pointer and uses a signed addressing, meaning that tiles 0-127 are in block 2, and tiles -128 to -1 are in block 1; or, to put it differently, “$8800 addressing” takes tiles 0-127 from block 2 and tiles 128-255 from block 1

block 1 is shared by both addressing methods

#### Data format

- Each tile occupies 16 bytes, where each line is represented by 2 bytes
- For each line, the first byte specifies the least significant bit of the color ID of each pixel, and the second byte specifies the most significant bit
- In both bytes, bit 7 represents the leftmost pixel, and bit 0 the rightmost
- Each pixel has a color index of 0 to 3 which are translated into gray shades depending on the current palettes

### Tile Maps

- Two 32x32 tile maps in VRAM at $9800-$9BFF and $9C00-$9FFF

#### Tile Indexes

- Each tile map contains the 1-byte indexes of the tiles to be displayed
- Tiles are obtained from the Tile Data Table using either of the two addressing modes
- Since one tile has 8×8 pixels, each map holds a 256×256 pixels picture Only 160×144 of those pixels are displayed on the LCD at any given time

#### Background (BG)

- The SCY and SCX registers can be used to scroll the Background, specifying the origin of the visible 160×144 pixel area within the total 256×256 pixel Background map The visible area of the Background wraps around the Background map (that is, when part of the visible area goes beyond the map edge, it starts displaying the opposite side of the map)

#### Window

- Window overlays bg
- Content of the Window is not scrollable
- only way to adjust the Window is by modifying its position on the screen, which is done via the WX and WY registers
- screen coordinates of the top left corner of the Window are (WX-7,WY)
- tiles for the Window are stored in the Tile Data Table. both bg and window share same tile data table
- Whether the Window is displayed can be toggled using LCDC bit 5 but this bit is only functional when LCDC bit 0 is set
- Enabling the Window makes Mode 3 slightly longer on scanlines where it’s visible
- window keeps an internal line counter that’s functionally similar to LY, and increments alongside it. However, it only gets incremented when the window is visible, as describe. This line counter determines what window line is to be rendered on the current scanline

### Object Attribute Memory (OAM)

- PPU can display up to 40 movable objects/sprites, each 8×8 or 8×16 pixels
- Because of a limitation of hardware, only ten objects can be displayed per scanline
- Object tiles have the same format as BG tiles, but they are taken from tile blocks 0 and 1 located at $8000-8FFF and have unsigned numbering
- attributes reside in the object attribute memory (OAM) at $FE00-FE9F

Each of the 40 entries consists of four bytes:

#### Byte 0 — Y Position

Y = Object’s vertical position on the screen + 16. So for example:
Y=0 hides an object,
Y=2 hides an 8×8 object but displays the last two rows of an 8×16 object,
Y=16 displays an object at the top of the screen,
Y=144 displays an 8×16 object aligned with the bottom of the screen,
Y=152 displays an 8×8 object aligned with the bottom of the screen,
Y=154 displays the first six rows of an object at the bottom of the screen,
Y=160 hides an object.

#### Byte 1 — X Position

- X = Object’s horizontal position on the screen + 8
- An off-screen value (X=0 or X>=168) hides the object, but the object still contributes to the limit of ten objects per scanline

#### Byte 2 — Tile Index

- In 8×8 mode (LCDC bit 2 = 0), this byte specifies the object’s only tile index ($00-$FF). This unsigned value selects a tile from the memory area at $8000-$8FFF

#### Byte 3 — Attributes/Flags

bit 7: Priority
bit 6: Y flip
bit 5: X flip
bit 4: DMG palette
bits 3-0: unused

#### Object Priority and Conflicts

Selection priority:

- each scanline’s OAM scan, the PPU compares LY (using LCDC bit 2 to determine their size) to each object’s Y position to select up to 10 objects to be drawn on that line. The PPU scans OAM sequentially (from $FE00 to $FE9F), selecting the first (up to) 10 suitably-positioned objects
- Since the PPU only checks the Y coordinate to select objects, even off-screen objects count towards the 10-objects-per-scanline limit

Drawing priority:

- When opaque pixels from two different objects overlap, which pixel ends up being displayed is determined by another kind of priority: the pixel belonging to the higher-priority object wins

### LCD Control

#### FF40 — LCDC: LCD control

LCDC is the main LCD Control register. Its bits toggle what elements are displayed on the screen, and how
LCD & PPU enable: 0 = Off; 1 = On
Window tile map area: 0 = 9800–9BFF; 1 = 9C00–9FFF
Window enable: 0 = Off; 1 = On
BG & Window tile data area: 0 = 8800–97FF; 1 = 8000–8FFF
BG tile map area: 0 = 9800–9BFF; 1 = 9C00–9FFF
OBJ size: 0 = 8×8; 1 = 8×16
OBJ enable: 0 = Off; 1 = On
BG & Window enable / priority [Different meaning in CGB Mode]: 0 = Off; 1 = On

#### LCDC.7 — LCD enable

This bit controls whether the LCD is on and the PPU is active. Setting it to 0 turns both off, which grants immediate and full access to VRAM, OAM, etc
When the display is disabled the screen is blank, which on DMG is displayed as a white “whiter” than color #0
When re-enabling the LCD, the PPU will immediately start drawing again, but the screen will stay blank during the first frame

#### LCDC.6 — Window tile map area

controls which background map the Window uses for rendering. When it’s clear (0), the $9800 tilemap is used, otherwise it’s the $9C00 one

#### LCDC.5 — Window enable

controls whether the window shall be displayed or not. This bit is overridden on DMG by bit 0 if that bit is clear.
Changing the value of this register mid-frame triggers a more complex behaviour

#### LCDC.4 — BG and Window tile data area

controls which addressing mode the BG and Window use to pick tiles
objects/sprites aren't affected by this and always use the $8000 addressing mode

#### LCDC.3 — BG tile map area

works similarly to LCDC bit 6: if the bit is clear (0), the BG uses tilemap $9800, otherwise tilemap $9C00

#### LCDC.2 — OBJ size

controls the size of all objects (1 tile or 2 stacked vertically)

#### LCDC.1 — OBJ enable

This bit toggles whether objects are displayed or not

#### LCDC.0 — BG and Window enable/priority

When Bit 0 is cleared, both background and window become blank (white), and the Window Display Bit is ignored in that case. Only objects may still be displayed (if enabled in Bit 1)

### LCD Status Registers

The Game Boy constantly compares the value of the LYC and LY registers. When both values are identical, the “LYC=LY” flag in the STAT register is set, and (if enabled) a STAT interrupt is requested

#### FF41 — STAT: LCD status

7|6|5|4|3|2|1|0
|LYC int select|Mode 2 int select|Mode 1 int select|Mode 0 int select|LYC == LY|PPU mode

LYC int select (Read/Write): If set, selects the LYC == LY condition for the STAT interrupt.
Mode 2 int select (Read/Write): If set, selects the Mode 2 condition for the STAT interrupt.
Mode 1 int select (Read/Write): If set, selects the Mode 1 condition for the STAT interrupt.
Mode 0 int select (Read/Write): If set, selects the Mode 0 condition for the STAT interrupt.
LYC == LY (Read-only): Set when LY contains the same value as LYC; it is constantly updated.
PPU mode (Read-only): Indicates the PPU’s current status. Reports 0 instead when the PPU is disabled.

### LCD Position and Scrolling

These registers can be accessed even during Mode 3, but modifications may not take effect immediately (see further below)

#### FF42–FF43 — SCY, SCX: Background viewport Y position, X position

- These two registers specify the top-left coordinates of the visible 160×144 pixel area within the 256×256 pixels BG map. Values in the range 0–255 may be used
- The PPU calculates the bottom-right coordinates of the viewport with those formulas: bottom := (SCY + 143) % 256 and right := (SCX + 159) % 256

#### FF4A–FF4B — WY, WX: Window Y position, X position plus 7

- These two registers specify the on-screen coordinates of the Window’s top-left pixel
- The Window is visible (if enabled) when both coordinates are in the ranges WX=0..166, WY=0..143 respectively Values WX=7, WY=0 place the Window at the top left of the screen, completely covering the background

### Palettes

FF47 — BGP (Non-CGB Mode only): BG palette data
This register assigns gray shades to the color indices of the BG and Window tiles
Each of the two-bit values map to a color thusly:
Value|Color
---|---
0|White
1|Light gray
2|Dark gray
3|Black

#### FF48–FF49 — OBP0, OBP1 (Non-CGB Mode only): OBJ palette 0, 1 data

These registers assigns gray shades to the color indexes of the OBJs that use the corresponding palette. They work exactly like BGP, except that the lower two bits are ignored because color index 0 is transparent for OBJs
