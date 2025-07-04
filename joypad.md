# Joypad Input

## Joypad Register

### FF00 — P1/JOYP: Joypad

- The eight Game Boy action/direction buttons are arranged as a 2×4 matrix
- Eeither action or direction buttons can be selected by writing to this register

| 7   | 6   | 5              | 4            | 3            | 2          | 1        | 0         |
| --- | --- | -------------- | ------------ | ------------ | ---------- | -------- | --------- |
|     |     | Select buttons | Select d-pad | Start / Down | Select/ Up | B / Left | A / Right |

- Select buttons: If this bit is 0, then buttons (SsBA) can be read from the lower nibble
- Select d-pad: If this bit is 0, then directional keys can be read from the lower nibble
- The lower nibble is Read-only
- Rather unconventionally for the Game Boy, a button being pressed is seen as the corresponding bit being 0, not 1
- If neither buttons nor d-pad is selected ($30 was written), then the low nibble reads $F (all buttons released)

## Joypad Interrupt

### INT $60 — Joypad interrupt

- The Joypad interrupt is requested when any of P1 bits 0-3 change from High to Low
- This happens when a button is pressed (provided that the action/direction buttons are enabled by bit 5/4, respectively), however, due to switch bounce, one or more High to Low transitions are usually produced when pressing a button

## Joypad to Keyboard Mapping

| Button | Key             |
| ------ | --------------- |
| A      | a               |
| B      | s               |
| Up     | up arrow key    |
| Down   | down arrow key  |
| Left   | left arrow key  |
| Right  | right arrow key |
| Start  | enter           |
| Select | space           |
