# PPU Details

- PPU draws a frame progressively. A frame consists of 154 scanlines; during the first 144 the screen is drawn top to bottom, left to right.
- The PPU unit of time is the "dot". One dot = 1 T-Cycle. So 4 dots = 1 M-cycle. In our emulator 1 dot of time = `sleep(1)`.

## PPU Modes

- During a frame, PPU cycles between 4 modes:
- Mode 2 - OAM scan (80 dots)
- Mode 3 - Drawing pixels (172 - 289 dots)
- Mode 0 - Horizontal Blank (87 - 204 dots)
  Each scanline = 456 dots.
- Mode 1 - Vertical blank (10 "scanlines" = 4560 dots)

### Mode 3 length

- During mode 3, the PPU outputs one pixel to the screen per dot, from left to right.
- The minimum Mode 3 length is 160 + 12 = 172 dots. he 12 extra dots of penalty come from two tile fetches at the beginning of Mode 3. One is the first tile in the scanline (the one that gets shifted by SCX % 8 pixels), the other is simply discarded.
- The Game Boy does not always output pixels steadily: some features cause the rendering process to stall for a couple dots. Any extra time spent stalling lengthens Mode 3; but since scanlines last for a fixed number of dots, Mode 0 is therefore shortened by that same amount of time.

Three things can cause Mode 3 "penalties":

- Background scrolling: At the very beginning of Mode 3, rendering is paused for SCX % 8 dots while the same number of pixels are discarded from the leftmost tile.
- Window: After the last non-window pixel is emitted, a 6-dot penalty is incurred while the BG fetcher is being set up for the window.
- Objects: Each object drawn during the scanline (even partially) incurs a 6 to 11 dot penalty.

### OBJ penalty algorithm

Only the OBJ’s leftmost pixel matters here, transparent or not:

1. Determine the tile (background or window) that The Pixel is within. (This is affected by horizontal scrolling and/or the window)
2. If that tile has not been considered by a previous OBJ yet (Since pixels are emitted from left to right, OBJs overlapping the scanline are considered from leftmost to rightmost, with ties broken by their index / OAM address (lowest first)):
   1. Count how many of that tile’s pixels are strictly to the right of The Pixel.
   2. Subtract 2.
   3. Incur this many dots of penalty, or zero if negative (from waiting for the BG fetch to finish).
3. Incur a flat, 6 dot penalty (from fetching the OBJ’s tile).

**Exception:** an OBJ with an OAM X position of 0 (thus, completely off the left side of the screen) always incurs a 11-dot penalty, regardless of SCX.

## Object Attribute Memory (OAM)

- The Game Boy PPU can display up to 40 movable objects (or sprites), each 8×8 or 8×16 pixels.
- Only ten objects can be displayed per scanline.
- Object tiles have the same format as BG tiles, but they are taken from tile blocks 0 and 1 located at $8000-8FFF and have unsigned numbering.
- Object attributes reside in the object attribute memory (OAM) at $FE00-FE9F.

Each of the 40 entries consists of four bytes with the following meanings:

Byte 0 — Y Position: Y = Object’s vertical position on the screen + 16
Byte 1 — X Position: X = Object’s horizontal position on the screen + 8. An off-screen value (X=0 or X>=168) hides the object, but the object still contributes to the limit of ten objects per scanline.
Byte 2 — Tile Index: In 8×8 mode (LCDC bit 2 = 0), this byte specifies the object’s only tile index ($00-$FF). This unsigned value selects a tile from the memory area at $8000-$8FFF. In 8×16 mode (LCDC bit 2 = 1), the memory area at $8000-$8FFF is still interpreted as a series of 8×8 tiles, where every 2 tiles form an object. In this mode, this byte specifies the index of the first (top) tile of the object.
Byte 3 — Attributes/Flags:
Bit 7: Priority: 0 = No, 1 = BG and Window color indices 1–3 are drawn over this OBJ
Bit 6: Y flip: 0 = Normal, 1 = Entire OBJ is vertically mirrored
Bit 5: X flip: 0 = Normal, 1 = Entire OBJ is horizontally mirrored
Bit 4: Palette: 0 = OBP0, 1 = OBP1

### Object Priority and Conflicts

- There are two kinds of “priorities” as far as objects are concerned. The first one defines which objects are ignored when there are more than 10 on a given scanline.
- The second one decides which object is displayed on top when some overlap.

Selection priority

- During each scanline’s OAM scan, the PPU compares LY (using LCDC bit 2 to determine their size) to each object’s Y position to select up to 10 objects to be drawn on that line. The PPU scans OAM sequentially (from $FE00 to $FE9F), selecting the first (up to) 10 suitably-positioned objects.

Drawing priority

- When opaque pixels from two different objects overlap, which pixel ends up being displayed is determined by another kind of priority: the pixel belonging to the higher-priority object wins. The smaller the X coordinate, the higher the priority. When X coordinates are identical, the object located first in OAM has higher priority.

### Interaction with "BG over OBJ" flag

- The PPU first resolves priority between objects to pick an “object pixel”, which is the first non-transparent pixel encountered when iterating over objects sorted by their drawing priority. The “BG over OBJ” attribute is never considered in this process.
- Only after object priority is resolved, the “object pixel” has the “BG over OBJ” attribute of its object checked to determine whether it should be drawn over the background.

## Pixel FIFO and Fetcher

- There are two pixel FIFOs. One for background pixels and one for object (sprite) pixels. These two FIFOs are not shared and are independent.
- The two FIFOs are mixed only when popping items.
- Objects take priority unless they’re transparent (color 0).
- Each FIFO can hold up to 8 pixels.
- The FIFO and Pixel Fetcher work together to ensure that the FIFO always contains at least 8 pixels at any given time, as 8 pixels are required for the Pixel Rendering operation to take place.
- Each FIFO is manipulated only during mode 3 (pixel transfer).

FIFO properties:

- Color: a value between 0 and 3
- Palette: only applies to objects
- Background priority: holds the value of the OBJ-to-BG Priority bit

### BG Pixel Fetcher

- The fetcher fetches a row of 8 background or window pixels and queues them up to be mixed with object pixels.
  The fetcher steps:

1. Get tile map
2. Get tile low
3. Get tile high
4. Push

VRAM reads take 2 T-cycles (2 dots) so the first 3 steps take 2 dots each. The push keeps happening every dot until it succeeds.

#### VRAM Access

At various times during PPU operation read access to VRAM is blocked and the value read is $FF:

- LCD turning off
- When switching from mode 3 to mode 0
  At various times during PPU operation read access to VRAM is restored:
- At scanline 0
- When searching OAM and index 37 is reached
- After switching from mode 2 (oam search) to mode 3 (pixel transfer)
  NOTE: These conditions are checked only when entering STOP mode and the PPU’s access to VRAM is always restored upon leaving STOP mode.

#### Get Tile Map

- This step determines which background/window tile to fetch pixels from. By default the tilemap used is the one at $9800 but certain conditions can change that.
- When LCDC.3 is enabled and the X coordinate of the current scanline is not inside the window then tilemap $9C00 is used.
- When LCDC.6 is enabled and the X coordinate of the current scanline is inside the window then tilemap $9C00 is used.
  The fetcher keeps track of which X and Y coordinate of the tile it’s on:
- If the current tile is a window tile, the X coordinate for the window tile is used, otherwise the following formula is used to calculate the X coordinate: ((SCX / 8) + fetcher’s X coordinate) & $1F. Because of this formula, fetcherX can be between 0 and 31.
- If the current tile is a window tile, the Y coordinate for the window tile is used, otherwise the following formula is used to calculate the Y coordinate: (currentScanline + SCY) & 255. Because of this formula, fetcherY can be between 0 and 255.
- The fetcher’s X and Y coordinate can then be used to get the tile from VRAM. However, if the PPU’s access to VRAM is blocked then the value for the tile is read as $FF.

#### Get Tile Data Low

- Check LCDC.4 for which tilemap to use.
- Once the tilemap, VRAM and vertical flip is calculated the tile data is retrieved from VRAM. However, if the PPU’s access to VRAM is blocked then the tile data is read as $FF.

#### Get Tile Data High

- Same as Get Tile Data Low except the tile address is incremented by 1.
- This also pushes a row of background/window pixels to the FIFO. This is an extra push attempt before state transitions to 'Push'.

#### Push

- Pushes a row of background/window pixels to the FIFO.
- Since tiles are 8 pixels wide, a “row” of pixels is 8 pixels from the tile to be rendered based on the X and Y coordinates calculated in the previous steps.
- Pixels are only pushed to the background FIFO if it’s empty.
- This is where the tile data retrieved in the two Tile Data steps will come in handy. Depending on if the tile is flipped horizontally the pixels will be pushed to the background FIFO differently. If the tile is flipped horizontally the pixels will be pushed LSB first. Otherwise they will be pushed MSB first.

IMPORTANT: Remember, the BG/window fetcher and the 2 pixel FIFOs operate concurrently. While the fetcher is fetching or trying to push, at the same time the FIFOs are pushing pixels out.

### Mode 3 Operation

- As stated before the pixel FIFO only operates during mode 3 (pixel transfer). At the beginning of mode 3 both the background and OAM FIFOs are cleared.

#### The Window

- When rendering the window the background FIFO is cleared and the fetcher is reset to step 1 (Get tile map).
- When WX is 0 and the SCX & 7 > 0 mode 3 is shortened by 1 dot.
- When the window has already started rendering there is a bug that occurs when WX is changed mid-scanline. When the value of WX changes after the window has started rendering and the new value of WX is reached again, a pixel with color value of 0 and the lowest priority is pushed onto the background FIFO.

#### Object Fetch Canceling

- Object fetching may be canceled if LCDC.1 is disabled while the PPU is fetching an object from OAM.
- This canceling lengthens mode 3 by the amount of dots the previous instruction took plus the residual dots left for the PPU to process.
- When OAM fetching is canceled, a pixel is rendered, and the fetcher is advanced one step.
- This advancement lengthens mode 3 by 1 dot if the current pixel is not 160.
- If the current pixel is 160 the PPU stops processing objects because they won’t be visible.

#### Sprites

- The following is performed for each object on the current scanline if LCDC.1 is enabled and the X coordinate of the current scanline has an object on it. If those conditions are not met then object fetching is canceled.
- At this point the fetcher is advanced one step until it's at the Push step or until the background FIFO is not empty. Advancing the fetcher one step here lengthens mode 3 by 1 dot. This process may be canceled after the fetcher has advanced a step.
- When SCX & 7 > 0 and there is an object at X coordinate 0 of the current scanline then mode 3 is lengthened. The amount of dots this lengthens mode 3 by is whatever the lower 3 bits of SCX are.
- After checking for objects at X coordinate 0 the fetcher is advanced two steps. The first advancement lengthens mode 3 by 1 dot and the second advancement lengthens mode 3 by 3 dots.
- The lower address for the row of pixels of the target object tile is now retrieved and lengthens mode 3 by 1 dot.
- The upper address for the target object tile is now retrieved and does not shorten mode 3.
- At this point VRAM Access is checked for the lower and upper addresses for the target object. Before any mixing is done, if the OAM FIFO doesn’t have at least 8 pixels in it then transparent pixels with the lowest priority are pushed onto the OAM FIFO.
- Once this is done each pixel of the target object row is checked. If the target object pixel is not white and the pixel in the OAM FIFO is white, or if the pixel in the OAM FIFO has higher priority than the target object’s pixel, then the pixel in the OAM FIFO is replaced with the target object’s properties.

#### Pixel Rendering

- This is where the background FIFO and OAM FIFO are mixed. There are conditions where either a background pixel or an object pixel will have display priority.
- If there are pixels in the background and OAM FIFOs then a pixel is popped off each. If the OAM pixel is not transparent and LCDC.1 is enabled then the OAM pixel’s background priority property is used if it’s the same or higher priority as the background pixel’s background priority.
- Pixels won’t be pushed to the LCD if there is nothing in the background FIFO or the current pixel is pixel 160 or greater.
- If LCDC.0 is disabled then the background is disabled.
- When the background pixel is disabled the pixel color value will be 0, otherwise the color value will be whatever color pixel was popped off the background FIFO.
- When the pixel popped off the background FIFO has a color value other than 0 and it has priority then the object pixel will be discarded.
- At this point, the color of the pixel is retrieved from the BGP register and pushed to the LCD.
- When an object pixel has priority, the color value is retrieved from the popped pixel from the OAM FIFO.
- The color for the pixel is retrieved from either the OBP1 or OBP0 register depending on the pixel’s palette property. If the palette property is 1 then OBP1 is used, otherwise OBP0 is used. The pixel is then pushed to the LCD.

IMPORTANT: Many of the Mode 3 "penalties" discussed here happen naturally when the emulation is done correctly. It's not something that needs to be forcefully added.

## LCD Position and Scrolling

- These registers can be accessed even during Mode 3, but modifications may not take effect immediately

### FF42–FF43 — SCY, SCX: Background viewport Y position, X position

- These two registers specify the top-left coordinates of the visible 160×144 pixel area within the 256×256 pixels BG map. Values in the range 0–255 may be used
- The PPU calculates the bottom-right coordinates of the viewport with those formulas: bottom := (SCY + 143) % 256 and right := (SCX + 159) % 256. As suggested by the modulo operations, in case the values are larger than 255 they will “wrap around” towards the top-left corner of the tilemap

### FF4A–FF4B — WY, WX: Window Y position, X position plus 7

- These two registers specify the on-screen coordinates of the Window’s top-left pixel
- The Window is visible (if enabled) when both coordinates are in the ranges WX=0..166, WY=0..143 respectively. Values WX=7, WY=0 place the Window at the top left of the screen, completely covering the background

### Mid-frame behavior

#### Scrolling

- The scroll registers are re-read on each tile fetch, except for the low 3 bits of SCX, which are only read at the beginning of the scanline (for the initial shifting of pixels)

#### Window

- While the Window should work as just mentioned, writing to WX, WY etc. mid-frame shows a more articulated behavior
- For the window to be displayed on a scanline, the following conditions must be met:
  - WY condition was triggered: i.e. at some point in this frame the value of WY was equal to LY (checked at the start of Mode 2 only)
  - WX condition was triggered: i.e. the current X coordinate being rendered + 7 was equal to WX
  - Window enable bit in LCDC is set
- If the WY condition has already been triggered and at the start of a row the window enable bit was set, then resetting that bit before the WX condition gets triggered on that row yields a nice window glitch pixel where the window would have been activated
- The way the Window selects which line of its tilemap to render may be surprising: the Y position is selected by an internal counter, which is reset to 0 during VBlank and only incremented when the Window starts being rendered on a given scanline. In particular, this means that hiding the Window mid-frame in any way (via either WX or LCDC, usually to display a status bar at the top and bottom of the screen) will also inhibit incrementing that Y-position counter

## DMG Acid2 Test by mattcurrie

## Guide

### Hello World!

The "Hello World" text is constructed from 10 objects, and the exclaimation
mark is part of the background. There is also an additional solid white object
where the exclaimation mark is. Due to the 10 object per line limit, the solid
white object should not be drawn, allowing the background to show through.

### Hair

The mohawk hair is not visible in the reference image. The hair tiles are
part of the background. For rows 8-15, bit 0 of LCDC is reset, so the
background should not be displayed, and instead color 0 from BGP should be
drawn on the screen.

### Eyes

The eyes consist of four background/window tiles, and objects overlapping color
0 background pixels.

The left eye is drawn using the background, and the right eye is drawn using
the window (and also the right edge of the head beside the right eye). At the
bottom of the eye, the WX register is set to an off-screen value, so the window
is hidden until the WX register is set again for drawing the right side of the
chin.

The background/window tiles for the left half of both eyes have color 0 for the
pupil region of the eye. There are two dark grey colored objects that overlap
the left half of both eyes that have the OBJ-to-BG Priority (bit 7) set in the
OAM flags, so the object will only show through color 0 of the background/
window tile and replaces the white color.

The top half of each eye uses tile data from the $8000-8fff region of VRAM. The
bottom half of each eye uses tile data from the $8800-97ff region of VRAM.
The bottom half of the eye uses tiles with index $a1 and $a2 to test that
signed tile indexes are displayed correctly.

### Moles

The moles are not visible in the reference image, but can be seen in the
failure case images.

The mole beside the left eye is visible if the background tile data is read
from $8000-8fff instead of $8800-97ff.

The mole left of the nose is not visible because a blank object with a lower
X-coordinate has priority, even though it is defined later in OAM than the mole
object.

The mole right of the nose is not visible because a blank object at the same X
coordinate has priority because it occurs earlier in OAM than the mole object.

### Nose

The nose consists of four objects using the same tile data that is flipped
vertically and/or horizontally. The top left corner of the nose contains the
unflipped tile.

### Mouth

The mouth consists of eight 8x16 objects. The left and right edges of the mouth
contain unflipped tile data, and other objects use vertically flipped tile
data.

For the left side of the mouth, the objects specify tile index 12, and the right
side of the mouth specify tile index 13. Because bit 0 of the tile index is
ignored for 8x16 objects, the whole mouth effectively uses tile index 12.

### Chin

The right side of the chin is drawn using the window. After the right eye was
drawn, the window was hidden by setting WX to an off-screen value. For the
right side of the chin, the WX value is restored to an on-screen value. After
the chin has been drawn, the window is disabled using bit 5 of the LCDC
register so the window does not cover the footer text.

For the right side of the chin, the window has been updated to use the tile map
from the VRAM beginning at $9800. Because 16 rows of window have already been
drawn for the eye, the right side of the chin is rendered starting from address
$9840.

### Footer Text

For the footer text, the background is set to use the $9c00-9fff region for
the map data, and tile data is set to come from the $8800-97ff region.

## Failure Examples

See the table below for some examples of incorrect behaviour and the
corresponding attribute/flag. This is not intended to be an exhaustive list of
all possible failures.

| Failure Description                                              | Functionality Tested                                   |
| ---------------------------------------------------------------- | ------------------------------------------------------ |
| Nose missing                                                     | Object Palette (bit 4)                                 |
| Nose tiles flipped, right eyelash flipped                        | Object Horizontal Flip (bit 5)                         |
| Nose tiles filpped, mouth straight, eye whites bottom left wrong | Object Vertical Flip (bit 6)                           |
| Eye whites left half wrong                                       | Object to Background Priority (bit 7)                  |
| Hair visible                                                     | Background Enable (bit 0)                              |
| Tongue visible                                                   | Object Enable (bit 1)                                  |
| Half of mouth missing                                            | Object Size (bit 2)                                    |
| Footer missing                                                   | Background Tile Map (bit 3)                            |
| Eye whites top left quadrant wrong                               | Background/Window Tile Data (bit 4)                    |
| Half of footer missing                                           | Window Enable (bit 5)                                  |
| Right chin missing                                               | Window Tile Map (bit 6)                                |
| Eye displayed instead of right chin                              | Window internal line counter                           |
| Left mole visible                                                | Object Priority Lower X Coordinate                     |
| Right mole visible                                               | Object Priority Same X Coordinate                      |
| Hello World missing exclaimation mark (!)                        | 10 object per line limit                               |
| Half of mouth missing                                            | Bit 0 of tile index for 8x16 objects should be ignored |
