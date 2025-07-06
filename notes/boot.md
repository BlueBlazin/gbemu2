## Gameboy State at Boot

### CPU registers

| Register | DMG              |
| -------- | ---------------- |
| A        | $01              |
| F        | Z=1 N=0 H=? C=?1 |
| B        | $00              |
| C        | $13              |
| D        | $00              |
| E        | $D8              |
| H        | $01              |
| L        | $4D              |
| PC       | $0100            |
| SP       | $FFFE            |

### Hardware registers

As far as timing-sensitive values are concerned, these values are recorded at PC = $0100

| Name | Address | DMG |
| ---- | ------- | --- |
| P1   | $FF00   | $CF |
| SB   | $FF01   | $00 |
| SC   | $FF02   | $7E |
| DIV  | $FF04   | $AB |
| TIMA | $FF05   | $00 |
| TMA  | $FF06   | $00 |
| TAC  | $FF07   | $F8 |
| IF   | $FF0F   | $E1 |
| LCDC | $FF40   | $91 |
| STAT | $FF41   | $85 |
| SCY  | $FF42   | $00 |
| SCX  | $FF43   | $00 |
| LY   | $FF44   | $00 |
| LYC  | $FF45   | $00 |
| DMA  | $FF46   | $FF |
| BGP  | $FF47   | $FC |
| WY   | $FF4A   | $00 |
| WX   | $FF4B   | $00 |
| IE   | $FFFF   | $00 |
