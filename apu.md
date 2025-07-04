# APU

- The Game Boy has four sound generation units, called channels 1 through 4, notated “CH1”, “CH2”, etc
- Each channel generates an electronic signal; these signals are then mixed into two new channels (for stereo: one for the left ear, one for the right ear), which are then individually amplified, and then output
- Channels 1 and 2, the “pulse channels”, produce pulse width modulated waves with 4 fixed pulse width settings
- Channel 3, the “wave” channel, produces arbitrary user-supplied waves
- Channel 4 is the “noise” channel, producing a pseudo-random wave

Each channel contains components that control different aspects of sound generation:

| Channel  | Sweep | Frequency      | Wave Form | Length Timer | Volume   |
| -------- | ----- | -------------- | --------- | ------------ | -------- |
| Square 1 | Sweep | Period Counter | Duty      | Length Timer | Envelope |
| Square 2 |       | Period Counter | Duty      | Length Timer | Envelope |
| Wave     |       | Period Counter | Wave      | Length Timer | Volume   |
| Noise    |       | Period Counter | LFSR      | Length Timer | Envelope |

## Common Concepts

- The APU runs off the same Master Clock as the rest of the Game Boy, which is to say, it is fully synced with the CPU and PPU
- All interfaces to the APU use durations instead of frequencies
- The Game Boy’s APU is actually full of tricky details. These details are listed in a later section.

### Triggering

- Triggering a channel causes it to turn on if it wasn’t, and to start playing its wave from the beginning
  - If the channel’s DAC is off, the channel will not turn on
  - Except for pulse channels, whose phase position is only ever reset by turning the APU off
- Most changes to a channel’s parameters take effect immediately, but some require re-triggering the channel

### Volume & envelope

- The volume can be controlled in two ways: there is a “master volume” control4 (which has separate settings for the left and right outputs), and each channel’s volume can be individually set as well
- Additionally, an envelope can be configured for CH1, CH2 and CH4, which allows automatically adjusting the volume over time
- The parameters that can be controlled are the initial volume, the envelope’s direction (but not its slope), and its duration
- Internally, all envelopes are ticked at 64 Hz, and every 1–7 of those ticks, the volume will be increased or decreased

### Length timer

- All channels can be individually set to automatically shut themselves down after a certain amount of time
- If the functionality is enabled, a channel’s length timer ticks up5 at 256 Hz (tied to DIV-APU) from the value it’s initially set at
- When the length timer reaches 64 (CH1, CH2, and CH4) or 256 (CH3), the channel is turned off

### Frequency

- Music notes and audio waves are typically manipulated in terms of frequency, however the Game Boy APU primarily works with durations; thus, periods will be used instead of frequency
- The terms “period” and “period value” throughout these notes refer to a parameter that has a somewhat nonintuitive relationship with frequency. See the description of each NRx3 register for more information

## Audio Registers

- Audio registers are named following a NRxy scheme, where x is the channel number (or 5 for “global” registers), and y is the register’s ID within the channel
- Since many registers share common properties, a notation is often used where e.g. NRx2 is used to designate NR12, NR22, NR32, and NR42 at the same time, for simplicity

As a rule of thumb, for any x in 1, 2, 3, 4:

- NRx0 is some channel-specific feature (if present),
- NRx1 controls the length timer,
- NRx2 controls the volume and envelope,
- NRx3 controls the period (maybe only partially),
- NRx4 has the channel’s trigger and length timer enable bits, as well as any leftover bits of period;
  But there are some exceptions

### Global control registers

#### FF26 — NR52: Audio master control

Bit 7: Audio on/off
Bit 3: CH4 on?
Bit 2: CH3 on?
Bit 1: CH2 on?
Bit 0: CH1 on?

- Audio on/off (Read/Write): This controls whether the APU is powered on at all (akin to LCDC bit 7)
- CHn on? (Read-only): Each of these four bits allows checking whether channels are active2. Writing to those does not enable or disable the channels, despite many emulators behaving as if
- A channel is turned on by triggering it (i.e. setting bit 7 of NRx4). A channel is turned off when any of the following occurs:
  - The channel’s length timer is enabled in NRx4 and expires, or
  - For CH1 only: when the period sweep overflows, or
  - The channel’s DAC is turned off. The envelope reaching a volume of 0 does NOT turn the channel off

#### FF25 — NR51: Sound panning

Bit 7: CH4 left
Bit 6: CH3 left
Bit 5: CH2 left
Bit 4: CH1 left
Bit 3: CH4 right
Bit 2: CH3 right
Bit 1: CH2 right
Bit 0: CH1 right

- Setting a bit to 1 enables the channel to go into the selected output

#### FF24 — NR50: Master volume & VIN panning

Bit 7: VIN left
Bits 6, 5, 4: Left volume
Bit 3: VIN right
Bit 2, 1, 0: Right volume

- VIN left/right: These work exactly like the bits in NR51. They should be set at 0 if external sound hardware is not being used
- Left/right volume: These specify the master volume, i.e. how much each output should be scaled

## Sound Channel 1 — Pulse with period sweep

### FF10 — NR10: Channel 1 sweep

- This register controls CH1’s period sweep functionality
  Bits 6, 5, 4: Pace
  Bit 3: Direction
  Bits 2, 1, 0: Individual step

- Pace: This dictates how often sweep “iterations” happen, in units of 128 Hz ticks (7.8 ms). Note that the value written to this field is not re-read by the hardware until a sweep iteration completes, or the channel is (re)triggered
- However, if 0 is written to this field, then iterations are instantly disabled (but see below), and it will be reloaded as soon as it’s set to something else
- Direction: 0 = Addition (period increases); 1 = Subtraction (period decreases)
- Individual step: On each iteration, the new period L[t+1] is computed from the current L[t] as:
  - L[t+1] = L[t] +/- L[t] / 2^step
- On each sweep iteration, the period in NR13 and NR14 is modified and written back
- In addition mode, if the period value would overflow, the channel is turned off instead. This occurs even if sweep iterations are disabled by the pace being 0

### FF11 — NR11: Channel 1 length timer & duty cycle

- This register controls both the channel’s length timer and duty cycle

Bits 7, 6: Wave duty
Bits 5, 4, 3, 2, 1, 0: Initial length timer

- Duty cycle (Read/Write): Controls the output waveform
- Initial length timer (Write-only): The higher this field is, the shorter the time before the channel is cut

### FF12 — NR12: Channel 1 volume & envelope

- This register controls the digital amplitude of the “high” part of the pulse, and the sweep applied to that setting
  Bits 7, 6, 5, 4: Initial volume
  Bit 3: Env dir
  Bit 2, 1, 0: Sweep pace

- Initial volume: How loud the channel initially is. Note that these bits are readable, but are not updated by the envelope functionality
- Env dir: The envelope’s direction; 0 = decrease volume over time, 1 = increase volume over time
- Sweep pace: The envelope ticks at 64 Hz, and the channel’s envelope will be increased / decreased (depending on bit 3) every Sweep pace of those ticks. A setting of 0 disables the envelope
- Setting bits 3-7 of this register all to 0 (initial volume = 0, envelope = decreasing) turns the DAC off

#### FF13 — NR13: Channel 1 period low [write-only]

- This register stores the low 8 bits of the channel’s 11-bit “period value”. The upper 3 bits are stored in the low 3 bits of NR14
- The period divider of pulse and wave channels is an up counter. Each time it is clocked, its value increases by 1; when it overflows (being clocked when it’s already 2047, or $7FF), its value is set from the contents of NR13 and NR14
- This means it treats the value in the period as a negative number in 11-bit two’s complement. The higher the period value in the register, the lower the period, and the higher the frequency
  - Period value $500 means -$300, or 1 sample per 768 input cycles
  - Period value $740 means -$C0, or 1 sample per 192 input cycles
- The pulse channels’ period dividers are clocked at 1048576 Hz, once per four dots, and their waveform is 8 samples long
- Period value $500 means -$300, or 1 sample per 768 input cycles
- Period value $740 means -$C0, or 1 sample per 192 input cycles
- Period value $740 produces a higher frequency than $500. Even though the period value $740 is not four times $500, $740 produces a frequency that is four times that of $500, or two octaves higher, because ($800 - $740) or 192 is one-quarter of ($800 - $500) or 768
- Period changes (written to NR13 or NR14) only take effect after the current “sample” ends

### FF14 — NR14: Channel 1 period high & control

Bit 7: Trigger
Bit 6: Length enable
Bits 2, 1, 0: Period

- Trigger (Write-only): Writing any value to NR14 with this bit set triggers the channel, causing the following to occur:
  - Ch1 is enabled
  - If length timer expired it is reset
  - The period divider is set to the contents of NR13 and NR14
  - Envelope timer is reset
  - Volume is set to contents of NR12 initial volume
  - Sweep does several things
- Length enable (Read/Write): Takes effect immediately upon writing to this register
- Period (Write-only): The upper 3 bits of the period value; the lower 8 bits are stored in NR13

## Sound Channel 2 — Pulse

- This sound channel works exactly like channel 1, except that it lacks a period sweep (and thus an equivalent to NR10
- NR21 ($FF16) → NR11
- NR22 ($FF17) → NR12
- NR23 ($FF18) → NR13
- NR24 ($FF19) → NR14

## Finer technical explanation

### DIV-APU

- A “DIV-APU” counter is increased every time DIV’s bit 4 goes from 1 to 0, therefore at a frequency of 512 Hz
- Thus, the counter can be made to increase faster by writing to DIV while its relevant bit is set (which clears DIV, and triggers the falling edge)

The following events occur every N DIV-APU ticks:

| Event          | Rate | Frequency |
| -------------- | ---- | --------- |
| Envelope sweep | 8    | 64 Hz     |
| Sound length   | 2    | 256 Hz    |
| CH1 freq sweep | 4    | 128 Hz    |

Frequency note: Indicated values are under normal operation; the frequencies will obviously differ if writing to DIV to increase the counter faster

### DACs

- Channel x’s DAC is enabled if and only if [NRx2] & $F8 != 0; the exception is CH3, whose DAC is directly controlled by bit 7 of NR30 instead. Note that the envelope functionality changes the volume, but not the value stored in NRx2, and thus doesn’t disable the DACs
- If a DAC is enabled, the digital range $0 to $F is linearly translated to the analog range -1 to 1, in arbitrary units. Importantly, the slope is negative: “digital 0” maps to “analog 1”, not “analog -1”
- If a DAC is disabled, it fades to an analog value of 0, which corresponds to “digital 7.5”. The nature of this fade is not entirely deterministic and varies between models
- NR52’s low 4 bits report whether the channels are turned on, not their DACs

### Channels

- A channel is activated by a write to NRx4’s MSB, unless its DAC is off, which forces it to be disabled as well
- The opposite is not true, however: a disabled channel outputs 0, which an enabled DAC will dutifully convert into “analog 1”
- A channel can be deactivated in one of the following ways:
  - Turning off its DAC
  - Its length timer expiring
  - (CH1 only) Frequency sweep overflowing the frequency

### Pulse channel with sweep (CH1)

- The first square channel has a frequency sweep unit, controlled by NR10
- This internally has a “sweep timer”, “enabled flag”, and a period/frequency “shadow register”
- The “enabled flag” controls if the sweep unit is active, the “sweep timer” is clocked at 128 Hz by the DIV-APU, and the “shadow register” holds the current output period
- During a trigger event, several things occur:
  - CH1 period value is copied to the “shadow register”
  - The “sweep timer” is reset
  - The “enabled flag” is set if either the sweep pace or individual step are non-zero, cleared otherwise
  - If the individual step is non-zero, frequency calculation and overflow check are performed immediately
- Frequency calculation consists of taking the value in the frequency “shadow register”, shifting it right by the individual step, optionally negating the value (depending on the direction) and summing this with the frequency “shadow register” to produce a new frequency
- The overflow check simply calculates the new frequency and if it is greater than 2047, or $7FF, CH1 is disabled
- When the “sweep timer” is clocked if the “enabled flag” is set and the sweep pace is not zero, a new frequency is calculated and the overflow check is performed. If the new frequency is 2047 or less and the individual step is not zero, this new frequency is written back to the “shadow register” and CH1 frequency in NR13 and NR14, then frequency calculation and overflow check are run again immediately using this new value, but this second new frequency is not written back
- CH1 frequency can be modified via NR13 and NR14 while sweep is active, but the “shadow register” won’t be affected so the next time the “sweep timer” updates the channel’s frequency, this modification will be lost. This can be avoided by triggering the channel

### Pulse channels (CH1, CH2)

- Each pulse channel has an internal “duty step” counter, which is used to index into the selected waveform
- The “duty step” increments at the channel’s sample rate, which is 8 times the channel’s frequency
- The “duty step” counter cannot be reset, except by turning the APU off, which sets both back to 0
- Retriggering a pulse channel causes its “duty step timer” to reset, thus retriggering a pulse channel often enough will cause its “duty step” to never advance
- When first starting up a pulse channel, it will always output a (digital) zero

## More Notes

These are notes from another source:

The square channel has something called duty cycles. There are 4 of these duty cycles, each defining an 8-step sequence of 1s and 0s, highs and lows. The gg8 wiki defines what these sequences are. I define these duty cycles using a simple 2-dimensional boolean lookup table. You can change which duty cycle to use at any time, and they generate different sounding tones depending on which duty cycle is used.

Each channel has something called a "frequency" register, which for Square 2 is on NR23 and NR24, but I've also seen it defined as "period" and a "timer". I like to think of it as a sort of "timer" for simplicity, so I'll call it the timer. The square wave's "timer" is set to (2048 - frequency)\*4. Every CPU cycle, this timer is decremented, when it hits 0, the timer resets back.

The "output" from the channel is either 0 or the current "volume" depending on where you are in the waveform, or to put it another way, what step of the 8-step duty cycle you're on. I have a variable called a "sequence pointer", that points to one of these 8-steps, is incremented whenever the timer mentioned earlier hits 0, and wraps back to 0 every 8 steps.

Some channels have up to 3 other components. You don't need to implement these completely for a "basic" output, but you want to do them eventually. To summarize, sweep can periodically adjust the "frequency" of a channel, Envelope periodically adjusts the volume, the length counter shuts off a channel after a period of time. All 3 are clocked by another component in the APU called the frame sequencer. This sequencer is clocked about every 8192 CPU cycles (it runs at 512hz, CPU runs at 4194304hz, 4194304/512=8192). There's 8-steps to the frame sequencer, so every 8192 cpu cycles is 1 step. Every other step, the length counter is clocked in each channel, on the 7th step, the envelope is clocked, and on the 2nd and 6th step, the sweep generator is clocked.

In order to actually hear the output, you need to convert the output to something that our sound output library actually likes, usually raw PCM samples. They're pretty simple in structure, just a line of "volume levels" like the APU outputs. They can be encoded in different ways, like using floats or integers. I prefer to use float samples since that's what I sort of used for the NES. To convert a channel's output to a float, all I do is take the output and divide by 100. The other problem is that the APU can output samples much more frequently than our computers like them, you can think of them outputting a sample once per CPU cycle, but the CPU runs at 4194304hz, and our computers like samples at much lower rates like 44100 or 48000hz. So much like how you have to resize a large image to fit a small screen, we have to resize a large set of samples for a smaller buffer. There's many techniques you can utilize to downsample, but the easiest to implement is nearest-neighbor. Essentially, only gather samples every CPU-Rate/Sampling-Rate, for example, if you want to use 48000hz, 4194304/48000 = 87 (rounded), so gather samples every 87 CPU cycles.

## Square Channel Example Duty Table

```rs
const DUTY_TABLE: [[bool; 8]; 4] = [
    [false, false, false, false, false, false, false, true],
    [true, false, false, false, false, false, false, true],
    [true, false, false, false, false, true, true, true],
    [false, true, true, true, true, true, true, false],
];
```

## Sound Channel 3 — Wave output

- While other channels only offer limited control over the waveform they generate, this channel allows outputting any wave
- While the “length” of the wave is fixed at 32 “samples”, 4-bit each, the speed at which it is read can be customized
- It’s possible to “shorten” the wave by either feeding it a repeating pattern, or doubling each sample and doubling the read rate
- It’s also possible to artificially “increase” the wave’s length by loading a new wave as soon as the whole buffer has been read

### FF1A — NR30: Channel 3 DAC enable

- This register controls CH3’s DAC. Like other channels, turning the DAC off immediately turns the channel off as well
  Bit 7: DAC on/off

### FF1B — NR31: Channel 3 length timer [write-only]

- This register controls the channel’s length timer
  Bits 7, 6, 5, 4, 3, 2, 1: Initial length timer
  The higher the length timer, the shorter the time before the channel is cut

### FF1C — NR32: Channel 3 output level

- This channel lacks the envelope functionality that the other three channels have, and has a much coarser volume control
  Bits 6, 5: Output level

Output level: Controls the channel’s volume as follows:

| Bits 6-5 (binary) | Output level                                              |
| ----------------- | --------------------------------------------------------- |
| 00                | Mute (No sound)                                           |
| 01                | 100% volume (use samples read from Wave RAM as-is)        |
| 10                | 50% volume (shift samples read from Wave RAM right once)  |
| 11                | 25% volume (shift samples read from Wave RAM right twice) |

### FF1D — NR33: Channel 3 period low [write-only]

- This register stores the low 8 bits of the channel’s 11-bit “period value”
- The upper 3 bits are stored in the low 3 bits of NR34
- The wave channel’s period divider is clocked at 2097152 Hz, once per two dots, and its waveform is 32 samples long
- Period value $500 means -$300, or 1 sample per 768 input cycles
  - or (2097152 ÷ 768) = 2730.7 Hz sample rate
  - or (2097152 ÷ 768 ÷ 32) = 85.333 Hz tone frequency
- Period value $740 means -$C0, or 1 sample per 192 input cycles
  - or (2097152 ÷ 192) = 10923 Hz sample rate
  - or (2097152 ÷ 192 ÷ 32) = 341.33 Hz tone frequency
- Period changes (written to NR33 or NR34) only take effect after the following time wave RAM is read

### FF1E — NR34: Channel 3 period high & control

Bit 7: Trigger
Bit 6: Length enable
Bits 2, 1, 0: Period

- Trigger (Write-only): Writing any value to NR34 with this bit set triggers the channel, causing the following to occur:
  - Ch3 is enabled.
  - If the length timer expired it is reset.
  - The period divider is set to the contents of NR33 and NR34.
  - Volume is set to contents of NR32 initial volume.
  - Wave RAM index is reset, but its not refilled.
- Length enable (Read/Write): Takes effect immediately upon writing to this register
- Period (Write-only): The upper 3 bits of the period value; the lower 8 bits are stored in NR33

### FF30–FF3F — Wave pattern RAM

- Wave RAM is 16 bytes long; each byte holds two “samples”, each 4 bits
- As CH3 plays, it reads wave RAM left to right, upper nibble first. That is, $FF30’s upper nibble, $FF30’s lower nibble, $FF31’s upper nibble, and so on
- Wave RAM can be accessed normally even if the DAC is on, as long as the channel is not active

## Wave channel (CH3) Details

- CH3 has an internal “sample index” counter. The “sample index” increments at the channel’s sample rate, which is 32 times the channel’s frequency Each time it increments, the corresponding “sample” (nibble) is read from wave RAM. (This means that sample #0 is skipped when first starting up CH3.)
- CH3 does not emit samples directly, but stores every sample read into a buffer, and emits that continuously; (re)triggering the channel does not clear nor refresh this buffer, so the last sample ever read will be emitted again. This buffer is cleared when turning the APU on, so CH3 will emit a “digital 0” when first powered on
- CH3 output level control does not, in fact, alter the output level. It shifts the digital value CH3 is outputting, not the analog value. This only matters when changing the setting mid-playback: the digital values being shifted bias them towards 0, which biases the analog output towards “1”; the HPF will smooth this over time, but not instantly

### Sound Channel 4 — Noise

- This channel is used to output white noise, which is done by randomly switching the amplitude between two levels fairly fast
- The frequency can be adjusted in order to make the noise appear “harder” (lower frequency) or “softer” (higher frequency)
- The random function that switches the output level can also be manipulated. Certain settings can cause the wave to be more regular, sounding closer to a pulse than noise

#### FF20 — NR41: Channel 4 length timer [write-only]

- This register controls the channel’s length timer
  Bits 5, 4, 3, 2, 1, 0: Initial length timer
- The higher the length timer, the shorter the time before the channel is cut

#### FF21 — NR42: Channel 4 volume & envelope

- This register controls the digital amplitude of the “high” part of the pulse, and the sweep applied to that setting
  Bits 7, 6, 5, 4: Initial volume
  Bit 3: Env dir
  Bit 2, 1, 0: Sweep pace

- Initial volume: How loud the channel initially is. Note that these bits are readable, but are not updated by the envelope functionality
- Env dir: The envelope’s direction; 0 = decrease volume over time, 1 = increase volume over time
- Sweep pace: The envelope ticks at 64 Hz, and the channel’s envelope will be increased / decreased (depending on bit 3) every Sweep pace of those ticks. A setting of 0 disables the envelope
- Setting bits 3-7 of this register all to 0 (initial volume = 0, envelope = decreasing) turns the DAC off

#### FF22 — NR43: Channel 4 frequency & randomness

- This register allows controlling the way the amplitude is randomly switched
  Bits 7, 6, 5, 4: Clock shift
  Bit 3: LFSR width
  Bits 2, 1, 0: Clock divider

- Clock shift: See frequency formula below
- LFSR width: 0 = 15-bit, 1 = 7-bit (more regular output; some frequencies sound more like pulse than noise)
- Clock divider: See the frequency formula below. Note that divider = 0 is treated as divider = 0.5 instead
- The frequency at which the LFSR is clocked is (262144 / (divider \* 2^shift)) Hz
- If the bit shifted out is a 0, the channel emits a 0; otherwise, it emits the volume selected in NR42

#### FF23 — NR44: Channel 4 control

Bit 7: Trigger
Bit 6: Length enable

- Trigger (Write-only): Writing any value to NR44 with this bit set triggers the channel, causing the following to occur:
  - Ch4 is enabled
  - If the length timer expired it is reset
  - Envelope timer is reset
  - Volume is set to contents of NR42 initial volume
  - LFSR bits are reset
- Length enable (Read/Write): Takes effect immediately upon writing to this register

### Noise channel (CH4) Details

- CH4 revolves around a LFSR.
- The LFSR has 16 bits: 15 bits for its current state and 1 bit to temporarily store the next bit to shift in
- When CH4 is ticked (at the frequency specified via NR43):
  - The result of LFSR_0 ⊙ LFSR_1 (1 if bit 0 and bit 1 are identical, 0 otherwise) is written to bit 15.
  - If “short mode” was selected in NR43, then bit 15 is copied to bit 7 as well.
  - Finally, the entire LFSR is shifted right, and bit 0 selects between 0 and the chosen volume.
- The LFSR is set to 0 when (re)triggering the channel
- If the “active” portion of the LFSR only contains “1” bits, only “1” bits will be generated; this prevents CH4 from ever changing values (until retriggered), essentially silencing it

Noise divisors example from another emulator's source:

```rs
const DIVISORS: [usize; 8] = [8, 16, 32, 48, 64, 80, 96, 112];
```
