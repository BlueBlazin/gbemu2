## Timer and Divider Registers

### FF04 — DIV: Divider register

- This register is incremented at a rate of 16384Hz
- Writing any value to this register resets it to $00
- Additionally, this register is reset when executing the stop instruction, and only begins ticking again once stop mode ends

### FF05 — TIMA: Timer counter

- This timer is incremented at the clock frequency specified by the TAC register ($FF07)
- When the value overflows (exceeds $FF) it is reset to the value specified in TMA (FF06) and an interrupt is requested

### FF06 — TMA: Timer modulo

- When TIMA overflows, it is reset to the value in this register and an interrupt is requested
- However, if TMA is set to $FE, an interrupt is only requested every two increments, which effectively divides the selected clock by two. Setting TMA to $FD would divide the clock by three, and so on
- If a TMA write is executed on the same M-cycle as the content of TMA is transferred to TIMA due to a timer overflow, the old value is transferred to TIMA

### FF07 — TAC: Timer control

bit 2: enable
bits 1 0: clock select

- Enable: Controls whether TIMA is incremented. Note that DIV is always counting, regardless of this bit
- Clock select: Controls the frequency at which TIMA is incremented, as follows:

| Clock Select | Increment every | Frequency (Hz) |
| ------------ | --------------- | -------------- |
| 00           | 256 M-cycles    | 4096           |
| 01           | 4 M-cycles      | 262144         |
| 10           | 16 M-cycles     | 65536          |
| 11           | 64 M-cycles     | 16384          |
