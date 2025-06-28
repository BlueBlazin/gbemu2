## Interrupts

### IME: Interrupt master enable flag [write only]

IME is a flag internal to the CPU that controls whether any interrupt handlers are called, regardless of the contents of IE. IME cannot be read in any way, and is modified by these instructions/events only:

- ei: Enables interrupt handling (that is, IME := 1)
- di: Disables interrupt handling (that is, IME := 0)
- reti: Enables interrupts and returns (same as ei immediately followed by ret)
- When an interrupt handler is executed: Disables interrupts before calling the interrupt handler

IME is unset (interrupts are disabled) when the game starts running
The effect of ei is delayed by one instruction. This means that ei followed immediately by di does not allow any interrupts between them. This interacts with the halt bug in an interesting way

### FFFF — IE: Interrupt enable

bit 4: Joypad
bit 3: Serial
bit 2: Timer
bit 1: LCD
bit 0: VBlank

- VBlank (Read/Write): Controls whether the VBlank interrupt handler may be called
- LCD (Read/Write): Controls whether the LCD interrupt handler may be called
- Timer (Read/Write): Controls whether the Timer interrupt handler may be called
- Serial (Read/Write): Controls whether the Serial interrupt handler may be called
- Joypad (Read/Write): Controls whether the Joypad interrupt handler may be called

### FF0F — IF: Interrupt flag

bit 4: Joypad
bit 3: Serial
bit 2: Timer
bit 1: LCD
bit 0: VBlank

- VBlank (Read/Write): Controls whether the VBlank interrupt handler is being requested
- LCD (Read/Write): Controls whether the LCD interrupt handler is being requested
- Timer (Read/Write): Controls whether the Timer interrupt handler is being requested
- Serial (Read/Write): Controls whether the Serial interrupt handler is being requested
- Joypad (Read/Write): Controls whether the Joypad interrupt handler is being requested

### Interrupt Handling

1. The IF bit corresponding to this interrupt and the IME flag are reset by the CPU. The former “acknowledges” the interrupt, while the latter prevents any further interrupts from being handled until the program re-enables them, typically by using the reti instruction
2. The corresponding interrupt handler is called by the CPU. This is a regular call, exactly like what would be performed by a call <address> instruction (the current PC is pushed onto the stack and then set to the address of the interrupt handler)

The following interrupt service routine is executed when control is being transferred to an interrupt handler:

- Two wait states are executed (2 M-cycles pass while nothing happens; presumably the CPU is executing nops during this time).
- The current value of the PC register is pushed onto the stack, consuming 2 more M-cycles.
- The PC register is set to the address of the handler (one of: $40, $48, $50, $58, $60). This consumes one last M-cycle.
- The entire process lasts 5 M-cycles

### Interrupt Priorities

In the following circumstances it is possible that more than one bit in the IF register is set, requesting more than one interrupt at once:

- More than one interrupt request signal changed from low to high at the same time
- Several interrupts have been requested while IME/IE didn’t allow them to be serviced
- The user has written a value with several bits set (for example binary 00011111) to the IF register
- If IME and IE allow the servicing of more than one of the requested interrupts, the interrupt with the highest priority is serviced first. The priorities follow the order of the bits in the IE and IF registers: Bit 0 (VBlank) has the highest priority, and Bit 4 (Joypad) has the lowest priority

### Nested Interrupts

The CPU automatically disables all the other interrupts by setting IME=0 when it services an interrupt. Usually IME remains zero until the interrupt handler returns (and sets IME=1 by means of the reti instruction). However, if you want to allow the servicing of other interrupts (of any priority) during the execution of an interrupt handler, you may do so by using the ei instruction in the handler

## Interrupt Sources
