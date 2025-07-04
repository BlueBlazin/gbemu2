use crate::driver::sleep;
use crate::emulator::{with_state, with_state_mut};

pub mod square;

pub use square::SquareChannel;

// APU register addresses
pub const NR10_REGISTER: u16 = 0xFF10; // CH1 Sweep
pub const NR11_REGISTER: u16 = 0xFF11; // CH1 Length/Duty
pub const NR12_REGISTER: u16 = 0xFF12; // CH1 Volume/Envelope
pub const NR13_REGISTER: u16 = 0xFF13; // CH1 Period Low
pub const NR14_REGISTER: u16 = 0xFF14; // CH1 Period High/Control

pub const NR21_REGISTER: u16 = 0xFF16; // CH2 Length/Duty
pub const NR22_REGISTER: u16 = 0xFF17; // CH2 Volume/Envelope
pub const NR23_REGISTER: u16 = 0xFF18; // CH2 Period Low
pub const NR24_REGISTER: u16 = 0xFF19; // CH2 Period High/Control

pub const NR50_REGISTER: u16 = 0xFF24; // Master Volume/VIN
pub const NR51_REGISTER: u16 = 0xFF25; // Sound Panning
pub const NR52_REGISTER: u16 = 0xFF26; // Master Control

// APU timing constants
const SAMPLE_RATE: u32 = 48000; // Output sample rate
const CPU_FREQUENCY: u32 = 4194304; // Game Boy CPU frequency
const CYCLES_PER_SAMPLE: u32 = CPU_FREQUENCY / SAMPLE_RATE; // ~87 cycles per sample
const BUFFER_SIZE: usize = 2048; // Audio buffer size in samples

#[derive(Debug, Clone)]
pub struct ApuState {
    // Master control
    pub master_enable: bool,
    pub master_volume_left: u8,
    pub master_volume_right: u8,
    pub vin_left: bool,
    pub vin_right: bool,

    // Channel panning
    pub ch1_left: bool,
    pub ch1_right: bool,
    pub ch2_left: bool,
    pub ch2_right: bool,

    // Channels
    pub ch1: SquareChannel,
    pub ch2: SquareChannel,

    // Frame sequencer
    pub frame_sequencer_step: u8,   // 0-7
    pub frame_sequencer_timer: u32, // Timer for frame sequencer

    // Audio buffer
    pub audio_buffer: Vec<f32>,
    pub sample_accumulator: f32,
    pub sample_count: u32,
}

impl ApuState {
    pub fn new() -> Self {
        Self {
            master_enable: false,
            master_volume_left: 0,
            master_volume_right: 0,
            vin_left: false,
            vin_right: false,
            ch1_left: false,
            ch1_right: false,
            ch2_left: false,
            ch2_right: false,
            ch1: SquareChannel::new(),
            ch2: SquareChannel::new(),
            frame_sequencer_step: 0,
            frame_sequencer_timer: 0,
            audio_buffer: Vec::with_capacity(BUFFER_SIZE),
            sample_accumulator: 0.0,
            sample_count: 0,
        }
    }

    pub fn is_ch1_enabled(&self) -> bool {
        self.ch1.enabled
    }

    pub fn is_ch2_enabled(&self) -> bool {
        self.ch2.enabled
    }

    pub fn frame_sequencer_clock(&mut self) {
        match self.frame_sequencer_step {
            0 => {
                // Length counters
                self.ch1.clock_length();
                self.ch2.clock_length();
            }
            1 => {
                // Nothing
            }
            2 => {
                // Length counters and CH1 sweep
                self.ch1.clock_length();
                self.ch2.clock_length();
                self.ch1.clock_sweep();
            }
            3 => {
                // Nothing
            }
            4 => {
                // Length counters
                self.ch1.clock_length();
                self.ch2.clock_length();
            }
            5 => {
                // Nothing
            }
            6 => {
                // Length counters and CH1 sweep
                self.ch1.clock_length();
                self.ch2.clock_length();
                self.ch1.clock_sweep();
            }
            7 => {
                // Envelope
                self.ch1.clock_envelope();
                self.ch2.clock_envelope();
            }
            _ => {}
        }

        self.frame_sequencer_step = (self.frame_sequencer_step + 1) % 8;
    }

    pub fn mix_sample(&mut self) {
        if !self.master_enable {
            return;
        }

        // Get channel outputs
        let ch1_out = self.ch1.output() as f32;
        let ch2_out = self.ch2.output() as f32;

        // Mix channels (simple addition for now)
        let mut left_mix = 0.0;
        let mut right_mix = 0.0;

        if self.ch1_left {
            left_mix += ch1_out;
        }
        if self.ch1_right {
            right_mix += ch1_out;
        }
        if self.ch2_left {
            left_mix += ch2_out;
        }
        if self.ch2_right {
            right_mix += ch2_out;
        }

        // Apply master volume
        left_mix *= (self.master_volume_left as f32 + 1.0) / 8.0;
        right_mix *= (self.master_volume_right as f32 + 1.0) / 8.0;

        // Convert to final sample (-1.0 to 1.0 range)
        let final_sample = (left_mix + right_mix) / 2.0 / 15.0; // 15 is max channel volume

        self.sample_accumulator += final_sample;
        self.sample_count += 1;

        // Downsample when we have enough samples
        if self.sample_count >= CYCLES_PER_SAMPLE {
            let averaged_sample = if self.sample_count > 0 {
                self.sample_accumulator / (self.sample_count as f32)
            } else {
                0.0
            };
            self.audio_buffer.push(averaged_sample);
            self.sample_accumulator = 0.0;
            self.sample_count = 0;
        }
    }

    pub fn is_buffer_full(&self) -> bool {
        self.audio_buffer.len() >= BUFFER_SIZE
    }

    pub fn drain_buffer(&mut self) -> Vec<f32> {
        std::mem::replace(&mut self.audio_buffer, Vec::with_capacity(BUFFER_SIZE))
    }
}

pub async fn apu() {
    let mut div_apu_counter = 0u32;

    loop {
        sleep(1).await; // 1 T-cycle

        // Check if frame sequencer needs to run
        div_apu_counter += 1;
        let needs_frame_sequencer = if div_apu_counter >= 8192 {
            div_apu_counter = 0;
            true
        } else {
            false
        };

        // Combine all state access into a single call to avoid RefCell conflicts
        let should_emit_event = with_state_mut(|state| {
            // Check if APU is enabled
            if !state.apu.master_enable {
                return false;
            }

            // Clock period timers every T-cycle for square channels (they run at 1 MHz)
            state.apu.ch1.clock_period();
            state.apu.ch2.clock_period();

            // Mix audio sample every T-cycle
            state.apu.mix_sample();

            // Check for buffer full event
            state.apu.is_buffer_full()
        });

        // Check if we need to handle any events or timing
        if !with_state(|state| state.apu.master_enable) {
            continue;
        }

        // Emit audio buffer full event if needed
        if should_emit_event {
            crate::driver::emit_event(crate::driver::Event::AudioBufferFull);
        }

        // Run frame sequencer if needed
        if needs_frame_sequencer {
            with_state_mut(|state| {
                state.apu.frame_sequencer_clock();
            });
        }
    }
}

// APU register read/write functions
pub fn read_apu_register(address: u16, apu_state: &ApuState) -> u8 {
    match address {
        NR10_REGISTER => {
            (apu_state.ch1.sweep_pace << 4)
                | (if apu_state.ch1.sweep_direction {
                    0x08
                } else {
                    0
                })
                | apu_state.ch1.sweep_step
        }
        NR11_REGISTER => {
            apu_state.ch1.duty << 6 // Length timer is write-only
        }
        NR12_REGISTER => {
            (apu_state.ch1.initial_volume << 4)
                | (if apu_state.ch1.envelope_direction {
                    0x08
                } else {
                    0
                })
                | apu_state.ch1.envelope_pace
        }
        NR13_REGISTER => 0xFF, // Write-only
        NR14_REGISTER => {
            (if apu_state.ch1.length_enabled {
                0x40
            } else {
                0
            }) | 0xBF // Most bits read as 1
        }

        NR21_REGISTER => {
            apu_state.ch2.duty << 6 // Length timer is write-only
        }
        NR22_REGISTER => {
            (apu_state.ch2.initial_volume << 4)
                | (if apu_state.ch2.envelope_direction {
                    0x08
                } else {
                    0
                })
                | apu_state.ch2.envelope_pace
        }
        NR23_REGISTER => 0xFF, // Write-only
        NR24_REGISTER => {
            (if apu_state.ch2.length_enabled {
                0x40
            } else {
                0
            }) | 0xBF // Most bits read as 1
        }

        NR50_REGISTER => {
            (if apu_state.vin_left { 0x80 } else { 0 })
                | (apu_state.master_volume_left << 4)
                | (if apu_state.vin_right { 0x08 } else { 0 })
                | apu_state.master_volume_right
        }
        NR51_REGISTER => {
            (if apu_state.ch2_left { 0x20 } else { 0 })
                | (if apu_state.ch1_left { 0x10 } else { 0 })
                | (if apu_state.ch2_right { 0x02 } else { 0 })
                | (if apu_state.ch1_right { 0x01 } else { 0 })
        }
        NR52_REGISTER => {
            (if apu_state.master_enable { 0x80 } else { 0 })
                | (if apu_state.is_ch2_enabled() { 0x02 } else { 0 })
                | (if apu_state.is_ch1_enabled() { 0x01 } else { 0 })
                | 0x70 // Bits 4-6 always read as 1
        }

        _ => 0xFF, // Unused registers
    }
}

pub fn write_apu_register(address: u16, value: u8, apu_state: &mut ApuState) {
    if !apu_state.master_enable && address != NR52_REGISTER {
        return; // Ignore writes when APU is disabled
    }

    match address {
        NR10_REGISTER => {
            apu_state.ch1.sweep_pace = (value >> 4) & 0x07;
            apu_state.ch1.sweep_direction = (value & 0x08) != 0;
            apu_state.ch1.sweep_step = value & 0x07;
        }
        NR11_REGISTER => {
            apu_state.ch1.duty = (value >> 6) & 0x03;
            apu_state.ch1.length_timer = 64 - (value & 0x3F);
        }
        NR12_REGISTER => {
            apu_state.ch1.initial_volume = (value >> 4) & 0x0F;
            apu_state.ch1.envelope_direction = (value & 0x08) != 0;
            apu_state.ch1.envelope_pace = value & 0x07;
            apu_state.ch1.update_dac(value);
        }
        NR13_REGISTER => {
            apu_state.ch1.period = (apu_state.ch1.period & 0x0700) | (value as u16);
        }
        NR14_REGISTER => {
            let trigger = (value & 0x80) != 0;
            apu_state.ch1.length_enabled = (value & 0x40) != 0;
            apu_state.ch1.period = (apu_state.ch1.period & 0x00FF) | (((value & 0x07) as u16) << 8);

            if trigger {
                apu_state.ch1.trigger(true); // CH1 has sweep
            }
        }

        NR21_REGISTER => {
            apu_state.ch2.duty = (value >> 6) & 0x03;
            apu_state.ch2.length_timer = 64 - (value & 0x3F);
        }
        NR22_REGISTER => {
            apu_state.ch2.initial_volume = (value >> 4) & 0x0F;
            apu_state.ch2.envelope_direction = (value & 0x08) != 0;
            apu_state.ch2.envelope_pace = value & 0x07;
            apu_state.ch2.update_dac(value);
        }
        NR23_REGISTER => {
            apu_state.ch2.period = (apu_state.ch2.period & 0x0700) | (value as u16);
        }
        NR24_REGISTER => {
            let trigger = (value & 0x80) != 0;
            apu_state.ch2.length_enabled = (value & 0x40) != 0;
            apu_state.ch2.period = (apu_state.ch2.period & 0x00FF) | (((value & 0x07) as u16) << 8);

            if trigger {
                apu_state.ch2.trigger(false); // CH2 has no sweep
            }
        }

        NR50_REGISTER => {
            apu_state.vin_left = (value & 0x80) != 0;
            apu_state.master_volume_left = (value >> 4) & 0x07;
            apu_state.vin_right = (value & 0x08) != 0;
            apu_state.master_volume_right = value & 0x07;
        }
        NR51_REGISTER => {
            apu_state.ch2_left = (value & 0x20) != 0;
            apu_state.ch1_left = (value & 0x10) != 0;
            apu_state.ch2_right = (value & 0x02) != 0;
            apu_state.ch1_right = (value & 0x01) != 0;
        }
        NR52_REGISTER => {
            let old_enable = apu_state.master_enable;
            apu_state.master_enable = (value & 0x80) != 0;

            if old_enable && !apu_state.master_enable {
                // APU turned off - reset everything
                *apu_state = ApuState::new();
            }
        }

        _ => {} // Unused registers
    }
}
