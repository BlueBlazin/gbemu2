## Halt

- halt is an instruction that pauses the CPU (during which less power is consumed) when executed. The CPU wakes up as soon as an interrupt is pending, that is, when the bitwise AND of IE and IF is non-zero
- Most commonly, IME is set. In this case, the CPU simply wakes up, and before executing the instruction after the halt, the interrupt handler is called normally
- If IME is not set, there are two distinct cases, depending on whether an interrupt is pending as the halt instruction is first executed
  - If no interrupt is pending, halt executes as normal, and the CPU resumes regular execution as soon as an interrupt becomes pending. However, since IME=0, the interrupt is not handled.
  - If an interrupt is pending, halt immediately exits, as expected, however the “halt bug”, explained below, is triggered

## Halt Bug

- When a halt instruction is executed with IME = 0 and [IE] & [IF] != 0, the halt instruction ends immediately, but pc fails to be normally incremented
- Under most circumstances, this causes the byte after the halt to be read a second time (and this behaviour can repeat if said byte executes another halt instruction). But, if the halt is immediately followed by a jump to elsewhere, then the behaviour will be slightly different; this is possible in only one of two ways:
  - The halt comes immediately after a ei instruction (whose effect is typically delayed by one instruction, hence IME still being zero for the halt): the interrupt is serviced and the handler called, but the interrupt returns to the halt, which is executed again, and thus waits for another interrupt
  - The halt is immediately followed by a rst instruction: the rst instruction’s return address will point at the rst itself, instead of the byte after it. Notably, a ret would return to the rst an execute it again
- If the bugged halt is preceded by a ei and followed by a rst, the former “wins”

### Double-halt-cancel

- What happens if a program executes a double halt instruction with interrupts disabled, then changes the data at the location in question?
- It is conventionally assumed that executing a double halt will permanently lock up the CPU. What actually happens on hardware though, is that the CPU keep reading the second halt instruction over and over
- If the CPU would theoretically read a different value all of a sudden, this code would be allowed to run instead
- This could be done using a custom cartridge that can switch ROM banks automatically in the background. However, this test ROM uses VRAM inaccessibility to execute a rst $38 instruction when VRAM becomes inaccessible and reads $FF

To emulate the glitch and pass this test ROM, an emulator needs to emulate a few things:

- Correct emulation of halt, re-reading the memory of the second halt instruction over and over.
- Correct emulation and timing of VRAM inaccessibility, even for code executing in VRAM, and not just writes/reads.
- Correct emulation and timing of DIV, including the ability to reset it by writing to it.

### AntonioND's Halt Bug Notes

(Unverified and may be outdated or may not)

- HALT instruction has three different behaviours depending on IME, IE and IF. It behaves the same
  way in all Game Boy models
- IME = 1
  - HALT executed normally. CPU stops executing instructions until (IE & IF & 1F) != 0. When a flag in IF is set and the corresponding IE flag is also set, the CPU jumps to the interrupt vector. The return address pushed to the stack is the next instruction to the HALT, not the HALT itself. The IF flag corresponding to the vector the CPU has jumped in is cleared.
- IME = 0
  - (IE & IF & 1Fh) = 0
    - HALT mode is entered. It works like the IME = 1 case, but when a IF flag is set and the corresponding IE flag is also set, the CPU doesn't jump to the interrupt vector, it just continues executing instructions. The IF flags aren't cleared
  - (IE & IF & 1Fh) != 0
    - HALT mode is not entered. HALT bug occurs: The CPU fails to increase PC when executing the next instruction. The IF flags aren't cleared
