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

| Tile IDs for...     | Block 0    | Block 1    | Block 2 |
| ------------------- | ---------- | ---------- | ------- |
| $8000–87FF          | $8800–8FFF | $9000–97FF |
| Objects 0–127       | 128–255    | —          |
| BG/Win, if LCDC.4=1 | 0–127      | 128–255    | —       |
| BG/Win, if LCDC.4=0 | —          | 128–255    | 0–127   |

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

## PPU Operation

- PPU draws frame progressively and directly to the screen
- Frame consists of 154 scanlines. During first 144, screen is drawn top to bottom, left to right
- 4 "dots" per M-cycle

### PPU Modes

PPU cycles through 4 modes during a frame:

1. Mode 2: OAM scan. 80 dots
2. Mode 3: Drawing pixels. 172-289 dots
3. Mode 0: Horizontal Blank. 87-204 dots
4. Mode 1: Vertical blank. 10 "scanlines" 4560 dots

Each line: 456 dots
One frame: 70224 dots

While PPU is accessing video-related memory, that memory is inaccessible to CPU (writes are ignored and reads return $FF)

| Mode | Action                                     | Duration                            | Accessible video memory |
| ---- | ------------------------------------------ | ----------------------------------- | ----------------------- |
| 2    | Searching for OBJs which overlap this line | 80 dots                             | VRAM, CGB palettes      |
| 3    | Sending pixels to the LCD                  | Between 172 and 289 dots, see below | None                    |
| 0    | Waiting until the end of the scanline      | 376 - mode 3’s duration             | VRAM, OAM, CGB palettes |
| 1    | Waiting until the next frame               | 4560 dots (10 scanlines)            | VRAM, OAM, CGB palettes |

#### Mode 3 length

- During Mode 3, by default the PPU outputs one pixel to the screen per dot, from left to right; the screen is 160 pixels wide, so the minimum Mode 3 length is 160 + 12 = 172 dots.
- Game Boy does not always output pixels steadily. Some features cause the rendering process to stall for a couple dots
- Any extra time spent stalling lengthens Mode 3; but since scanlines last for a fixed number of dots, Mode 0 is therefore shortened by that same amount of time

Three things can cause Mode 3 “penalties”:

1. Background scrolling: At the very beginning of Mode 3, rendering is paused for SCX % 8 dots while the same number of pixels are discarded from the leftmost tile.
2. Window: After the last non-window pixel is emitted, a 6-dot penalty is incurred while the BG fetcher is being set up for the window.
3. Objects: Each object drawn during the scanline (even partially) incurs a 6- to 11-dot penalty

mid-scanline writes to BGP allow observing this behavior precisely, as any delay shifts the write’s effect to the left by that many dots

#### OBJ penalty algorithm

Only the OBJ’s leftmost pixel matters here, transparent or not

1. Determine the tile (bg or window) that The Pixel is within. (This is affected by horizontal scrolling and/or the window)
2. If that tile has not been considered by a previous OBJ yet:
   1. Count how many of that tile’s pixels are strictly to the right of The Pixel
   2. Subtract 2
   3. Incur this many dots of penalty, or zero if negative (from waiting for the BG fetch to finish)
3. Incur a flat, 6-dot penalty (from fetching the OBJ’s tile)
   Exception: an OBJ with an OAM X position of 0 (thus, completely off the left side of the screen) always incurs a 11-dot penalty, regardless of SCX

### Pixel FIFO

- There are two pixel FIFOs. One for bg pixels one for object/sprite pixels
- They are independent of each other and values from two are only mixed when popping items
- Objects take priority unless transparent (color 0)
- Each FIFO can hold up to 16 pixels
- FIFO and Pixel Fetcher work together to ensure FIFO always contains at least 8 pixels at any given time
- Each FIFO is only manipulated during Mode 3

Each pixel FIFO has 3 properties:

- Color: a value between 0 and 3
- Palette: only applies to objects
- Background Priority: holds the value of the OBJ-to-BG Priority bit

#### FIFO Pixel Fetcher

Fetches a row of 8 background or window pixels and queues them up to be mixed with object pixels
Pixel fetcher has 5 steps. The first four steps take 2 dots each and the fifth step is attempted every dot until it succeeds. Order of steps:

1. Get tile
2. Get tile data low
3. Get tile data high
4. Sleep
5. Push

##### Get Tile

- Determines which background/window tile to fetch pixels from. By default the tilemap used is the one at $9800 but certain conditions can change that
- When LCDC.3 is enabled and the X coordinate of the current scanline is not inside the window then tilemap $9C00 is used
- When LCDC.6 is enabled and the X coordinate of the current scanline is inside the window then tilemap $9C00 is used
- The fetcher keeps track of which X and Y coordinate of the tile it’s on
- If the current tile is a window tile, the X coordinate for the window tile is used, otherwise the following formula is used to calculate the X coordinate: ((SCX / 8) + fetcher’s X coordinate) & $1F. Because of this formula, fetcherX can be between 0 and 31
- If the current tile is a window tile, the Y coordinate for the window tile is used, otherwise the following formula is used to calculate the Y coordinate: (currentScanline + SCY) & 255. Because of this formula, fetcherY can be between 0 and 255
- fetcher’s X and Y coordinate can then be used to get the tile from VRAM. However, if the PPU’s access to VRAM is blocked then the value for the tile is read as $FF

##### Get Tile Data Low

- Check LCDC.4 for which tilemap to use
- Tile data is retrieved from VRAM. However, if the PPU’s access to VRAM is blocked then the tile data is read as $FF

##### Get Tile Data High

- Same as Get Tile Data Low except the tile address is incremented by 1
- This also pushes a row of background/window pixels to the FIFO. This extra push is not part of the 8 steps, meaning there’s 3 total chances to push pixels to the background FIFO every time the complete fetcher steps are performed

##### Push

- Pushes a row of background/window pixels to the FIFO. Since tiles are 8 pixels wide, a “row” of pixels is 8 pixels from the tile to be rendered based on the X and Y coordinates calculated in the previous steps
- Pixels are only pushed to the background FIFO if it’s empty
- This is where the tile data retrieved in the two Tile Data steps will come in handy. Depending on if the tile is flipped horizontally the pixels will be pushed to the background FIFO differently. If the tile is flipped horizontally the pixels will be pushed LSB first. Otherwise they will be pushed MSB first

##### Sleep

- Do nothing

#### Mode 3 Operation

- Pixel FIFO only operates during mode 3. At the beginning of mode 3 both the background and OAM FIFOs are cleared

##### The Window

- When rendering the window the background FIFO is cleared and the fetcher is reset to step 1
- When WX is 0 and the SCX & 7 > 0 mode 3 is shortened by 1 dot
- When the window has already started rendering there is a bug that occurs when WX is changed mid-scanline. When the value of WX changes after the window has started rendering and the new value of WX is reached again, a pixel with color value of 0 and the lowest priority is pushed onto the background FIFO

##### Sprites

- The following is performed for each object on the current scanline if LCDC.1 is enabled and the X coordinate of the current scanline has an object on it. If those conditions are not met then object fetching is canceled
- At this point the fetcher is advanced one step until it’s at step 5 or until the background FIFO is not empty. Advancing the fetcher one step here lengthens mode 3 by 1 dot. This process may be canceled after the fetcher has advanced a step
- When SCX & 7 > 0 and there is an object at X coordinate 0 of the current scanline then mode 3 is lengthened. The amount of dots this lengthens mode 3 by is whatever the lower 3 bits of SCX are. After this penalty is applied object fetching may be canceled
- After checking for objects at X coordinate 0 the fetcher is advanced two steps. First advancement lengthens mode 3 by 1 dot and the second advancement lengthens mode 3 by 3 dots
- The lower address for the row of pixels of the target object tile is now retrieved and lengthens mode 3 by 1 dot. Exiting object fetch lengthens mode 3 by 1 dot. The upper address for the target object tile is now retrieved and does not shorten mode 3
- At this point VRAM Access is checked for the lower and upper addresses for the target object. Before any mixing is done, if the OAM FIFO doesn’t have at least 8 pixels in it then transparent pixels with the lowest priority are pushed onto the OAM FIFO. Once this is done each pixel of the target object row is checked
- If the target object pixel is not white and the pixel in the OAM FIFO is white, or if the pixel in the OAM FIFO has higher priority than the target object’s pixel, then the pixel in the OAM FIFO is replaced with the target object’s properties
- Now it’s time to render a pixel! The same process described in Object Fetch Canceling is performed: a pixel is rendered and the fetcher is advanced one step. This advancement lengthens mode 3 by 1 dot if the X coordinate of the current scanline is not 160. If the X coordinate is 160 the PPU stops processing objects (because they won’t be visible)
- Everything in this section is repeated for every object on the current scanline unless it was decided that fetching should be canceled or the X coordinate is 160

##### Pixel Rendering

- This is where the background FIFO and OAM FIFO are mixed
- If there are pixels in the background and OAM FIFOs then a pixel is popped off each. If the OAM pixel is not transparent and LCDC.1 is enabled then the OAM pixel’s background priority property is used if it’s the same or higher priority as the background pixel’s background priority
- Pixels won’t be pushed to the LCD if there is nothing in the background FIFO or the current pixel is pixel 160 or greater
- If LCDC.0 is disabled then the background is disabled. When the background pixel is disabled the pixel color value will be 0, otherwise the color value will be whatever color pixel was popped off the background FIFO. When the pixel popped off the background FIFO has a color value other than 0 and it has priority then the object pixel will be discarded
- At this point the color of the pixel is retrieved from the BGP register and pushed to the LCD
- When an object pixel has priority, the color value is retrieved from the popped pixel from the OAM FIFO. The color for the pixel is retrieved from either the OBP1 or OBP0 register depending on the pixel’s palette property. If the palette property is 1 then OBP1 is used, otherwise OBP0 is used. The pixel is then pushed to the LCD

##### Object Fetch Canceling

Object fetching may be canceled if LCDC.1 is disabled while the PPU is fetching an object from OAM. This canceling lengthens mode 3 by the amount of dots the previous instruction took plus the residual dots left for the PPU to process. When OAM fetching is canceled, a pixel is rendered, and the fetcher is advanced one step. This advancement lengthens mode 3 by 1 dot if the current pixel is not 160. If the current pixel is 160 the PPU stops processing objects because they won’t be visible

## OAM DMA Transfer

### FF46 — DMA: OAM DMA source address & start

- Writing to this register starts a DMA transfer from ROM or RAM to OAM
- The written value specifies the transfer source address divided by $100, that is, source and destination are:
Source:      $XX00-$XX9F ;XX = $00 to $DF
Destination: $FE00-$FE9F
- The transfer takes 160 M-cycles: 640 dot

### OAM DMA Details

- Can copy one byte per machine cycle without involving the CPU at all
- The upper 8 bits of the OAM DMA source address are stored in the DMA register, while the lower 8 bits used by both the source and target address are stored in the DMA controller and are not accessible directly
- A transfer always begins with 0x00 in the lower bits and copies exactly 160 bytes, so the lower bits are never in the 0xA0-0xFF range
- Writing to the DMA register updates the upper bits of the DMA source address and also triggers an OAM DMA transfer request, although the DMA transfer does not begin immediately

bit 7-0 DMA: OAM DMA source address

- Writing to this register requests an OAM DMA transfer, but it’s just a request and the actual DMA transfer starts with a delay
- Reading this register returns the value that was previously written to the register. The stored value is not cleared on reset, so the initial value before the first write is unknown

### OAM DMA bus conflicts

- During OAM DMA, the CPU can access only HRAM (memory at $FF80-$FFFE)
- While an OAM DMA is in progress, the PPU cannot read OAM properly either
