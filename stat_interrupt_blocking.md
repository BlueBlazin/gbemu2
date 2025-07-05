# STAT Interrupt Blocking

## FF41 — STAT: LCD status

Bit 6: LYC int select
Bit 5: Mode 2 int select
Bit 4: Mode 1 int select
Bit 3: Mode 0 int select
Bit 2: LYC == LY
Bits 1, 0: PPU mode

- LYC int select (Read/Write): If set, selects the LYC == LY condition for the STAT interrupt
- Mode 2 int select (Read/Write): If set, selects the Mode 2 condition for the STAT interrupt
- Mode 1 int select (Read/Write): If set, selects the Mode 1 condition for the STAT interrupt
- Mode 0 int select (Read/Write): If set, selects the Mode 0 condition for the STAT interrupt
- LYC == LY (Read-only): Set when LY contains the same value as LYC; it is constantly updated
- PPU mode (Read-only): Indicates the PPU’s current status. Reports 0 instead when the PPU is disabled

## FF0F — IF: Interrupt flag

Bit 4: Joypad
Bit 3: Serial
Bit 2: Timer
Bit 1: LCD
Bit 0: VBlank

- When an interrupt request signal (some internal wire going from the PPU/APU/… to the CPU) changes from low to high, the corresponding bit in the IF register becomes set. For example, bit 0 becomes set when the PPU enters the VBlank period
- Any set bits in the IF register are only requesting an interrupt. The actual execution of the interrupt handler happens only if both the IME flag and the corresponding bit in the IE register are set; otherwise the interrupt “waits” until both IME and IE allow it to be serviced
- Since the CPU automatically sets and clears the bits in the IF register, it is usually not necessary to write to the IF register. However, the user may still do that in order to manually request (or discard) interrupts. Just like real interrupts, a manually requested interrupt isn’t serviced unless/until IME and IE allow it

## Interrupt Handling

- The IF bit corresponding to this interrupt and the IME flag are reset by the CPU. The former “acknowledges” the interrupt, while the latter prevents any further interrupts from being handled until the program re-enables them, typically by using the reti instruction
- The corresponding interrupt handler (see the IE and IF register descriptions above) is called by the CPU. This is a regular call, exactly like what would be performed by a call <address> instruction (the current PC is pushed onto the stack and then set to the address of the interrupt handler)

The following interrupt service routine is executed when control is being transferred to an interrupt handler:

1. Two wait states are executed (2 M-cycles pass while nothing happens; presumably the CPU is executing nops during this time).
2. The current value of the PC register is pushed onto the stack, consuming 2 more M-cycles.
3. The PC register is set to the address of the handler (one of: $40, $48, $50, $58, $60). This consumes one last M-cycle.

## INT $48 — STAT interrupt

- There are various sources which can trigger this interrupt to occur as described in STAT register ($FF41)
- The various STAT interrupt sources (modes 0-2 and LYC=LY) have their state (inactive=low and active=high) logically ORed into a shared “STAT interrupt line” if their respective enable bit is turned on
- A STAT interrupt will be triggered by a rising edge (transition from low to high) on the STAT interrupt line

## STAT blocking

- If a STAT interrupt source logically ORs the interrupt line high while (or immediately after) it’s already set high by another source, then there will be no low-to-high transition and so no interrupt will occur. This phenomenon is known as “STAT blocking”
- As mentioned in the description of the STAT register, the PPU cycles through the different modes in a fixed order. So for example, if interrupts are enabled for two consecutive modes such as Mode 0 and Mode 1, then no interrupt will trigger for Mode 1 (since the STAT interrupt line won’t have a chance to go low between them)
