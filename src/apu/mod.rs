use crate::driver::sleep;
use crate::emulator::with_state_mut;

pub mod noise;
pub mod square;
pub mod wave;

pub use noise::NoiseChannel;
pub use square::SquareChannel;
pub use wave::WaveChannel;

// Precomputed square wave sample tables for Game Boy audio
// Each table contains 8 steps per duty cycle × 16 volume levels
// Values are normalized floats from -1.0 to 1.0

// Duty cycle 0: 12.5%
const SQUARE_WAVE_DUTY_0: [[f32; 8]; 16] = [
    [
        0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
    ],
    [
        -0.066667, -0.066667, -0.066667, -0.066667, -0.066667, -0.066667, -0.066667, 0.066667,
    ],
    [
        -0.133333, -0.133333, -0.133333, -0.133333, -0.133333, -0.133333, -0.133333, 0.133333,
    ],
    [
        -0.200000, -0.200000, -0.200000, -0.200000, -0.200000, -0.200000, -0.200000, 0.200000,
    ],
    [
        -0.266667, -0.266667, -0.266667, -0.266667, -0.266667, -0.266667, -0.266667, 0.266667,
    ],
    [
        -0.333333, -0.333333, -0.333333, -0.333333, -0.333333, -0.333333, -0.333333, 0.333333,
    ],
    [
        -0.400000, -0.400000, -0.400000, -0.400000, -0.400000, -0.400000, -0.400000, 0.400000,
    ],
    [
        -0.466667, -0.466667, -0.466667, -0.466667, -0.466667, -0.466667, -0.466667, 0.466667,
    ],
    [
        -0.533333, -0.533333, -0.533333, -0.533333, -0.533333, -0.533333, -0.533333, 0.533333,
    ],
    [
        -0.600000, -0.600000, -0.600000, -0.600000, -0.600000, -0.600000, -0.600000, 0.600000,
    ],
    [
        -0.666667, -0.666667, -0.666667, -0.666667, -0.666667, -0.666667, -0.666667, 0.666667,
    ],
    [
        -0.733333, -0.733333, -0.733333, -0.733333, -0.733333, -0.733333, -0.733333, 0.733333,
    ],
    [
        -0.800000, -0.800000, -0.800000, -0.800000, -0.800000, -0.800000, -0.800000, 0.800000,
    ],
    [
        -0.866667, -0.866667, -0.866667, -0.866667, -0.866667, -0.866667, -0.866667, 0.866667,
    ],
    [
        -0.933333, -0.933333, -0.933333, -0.933333, -0.933333, -0.933333, -0.933333, 0.933333,
    ],
    [
        -1.000000, -1.000000, -1.000000, -1.000000, -1.000000, -1.000000, -1.000000, 1.000000,
    ],
];

// Duty cycle 1: 25%
const SQUARE_WAVE_DUTY_1: [[f32; 8]; 16] = [
    [
        0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
    ],
    [
        0.066667, -0.066667, -0.066667, -0.066667, -0.066667, -0.066667, -0.066667, 0.066667,
    ],
    [
        0.133333, -0.133333, -0.133333, -0.133333, -0.133333, -0.133333, -0.133333, 0.133333,
    ],
    [
        0.200000, -0.200000, -0.200000, -0.200000, -0.200000, -0.200000, -0.200000, 0.200000,
    ],
    [
        0.266667, -0.266667, -0.266667, -0.266667, -0.266667, -0.266667, -0.266667, 0.266667,
    ],
    [
        0.333333, -0.333333, -0.333333, -0.333333, -0.333333, -0.333333, -0.333333, 0.333333,
    ],
    [
        0.400000, -0.400000, -0.400000, -0.400000, -0.400000, -0.400000, -0.400000, 0.400000,
    ],
    [
        0.466667, -0.466667, -0.466667, -0.466667, -0.466667, -0.466667, -0.466667, 0.466667,
    ],
    [
        0.533333, -0.533333, -0.533333, -0.533333, -0.533333, -0.533333, -0.533333, 0.533333,
    ],
    [
        0.600000, -0.600000, -0.600000, -0.600000, -0.600000, -0.600000, -0.600000, 0.600000,
    ],
    [
        0.666667, -0.666667, -0.666667, -0.666667, -0.666667, -0.666667, -0.666667, 0.666667,
    ],
    [
        0.733333, -0.733333, -0.733333, -0.733333, -0.733333, -0.733333, -0.733333, 0.733333,
    ],
    [
        0.800000, -0.800000, -0.800000, -0.800000, -0.800000, -0.800000, -0.800000, 0.800000,
    ],
    [
        0.866667, -0.866667, -0.866667, -0.866667, -0.866667, -0.866667, -0.866667, 0.866667,
    ],
    [
        0.933333, -0.933333, -0.933333, -0.933333, -0.933333, -0.933333, -0.933333, 0.933333,
    ],
    [
        1.000000, -1.000000, -1.000000, -1.000000, -1.000000, -1.000000, -1.000000, 1.000000,
    ],
];

// Duty cycle 2: 50%
const SQUARE_WAVE_DUTY_2: [[f32; 8]; 16] = [
    [
        0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
    ],
    [
        0.066667, -0.066667, -0.066667, -0.066667, -0.066667, 0.066667, 0.066667, 0.066667,
    ],
    [
        0.133333, -0.133333, -0.133333, -0.133333, -0.133333, 0.133333, 0.133333, 0.133333,
    ],
    [
        0.200000, -0.200000, -0.200000, -0.200000, -0.200000, 0.200000, 0.200000, 0.200000,
    ],
    [
        0.266667, -0.266667, -0.266667, -0.266667, -0.266667, 0.266667, 0.266667, 0.266667,
    ],
    [
        0.333333, -0.333333, -0.333333, -0.333333, -0.333333, 0.333333, 0.333333, 0.333333,
    ],
    [
        0.400000, -0.400000, -0.400000, -0.400000, -0.400000, 0.400000, 0.400000, 0.400000,
    ],
    [
        0.466667, -0.466667, -0.466667, -0.466667, -0.466667, 0.466667, 0.466667, 0.466667,
    ],
    [
        0.533333, -0.533333, -0.533333, -0.533333, -0.533333, 0.533333, 0.533333, 0.533333,
    ],
    [
        0.600000, -0.600000, -0.600000, -0.600000, -0.600000, 0.600000, 0.600000, 0.600000,
    ],
    [
        0.666667, -0.666667, -0.666667, -0.666667, -0.666667, 0.666667, 0.666667, 0.666667,
    ],
    [
        0.733333, -0.733333, -0.733333, -0.733333, -0.733333, 0.733333, 0.733333, 0.733333,
    ],
    [
        0.800000, -0.800000, -0.800000, -0.800000, -0.800000, 0.800000, 0.800000, 0.800000,
    ],
    [
        0.866667, -0.866667, -0.866667, -0.866667, -0.866667, 0.866667, 0.866667, 0.866667,
    ],
    [
        0.933333, -0.933333, -0.933333, -0.933333, -0.933333, 0.933333, 0.933333, 0.933333,
    ],
    [
        1.000000, -1.000000, -1.000000, -1.000000, -1.000000, 1.000000, 1.000000, 1.000000,
    ],
];

// Duty cycle 3: 75%
const SQUARE_WAVE_DUTY_3: [[f32; 8]; 16] = [
    [
        0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
    ],
    [
        -0.066667, 0.066667, 0.066667, 0.066667, 0.066667, 0.066667, 0.066667, -0.066667,
    ],
    [
        -0.133333, 0.133333, 0.133333, 0.133333, 0.133333, 0.133333, 0.133333, -0.133333,
    ],
    [
        -0.200000, 0.200000, 0.200000, 0.200000, 0.200000, 0.200000, 0.200000, -0.200000,
    ],
    [
        -0.266667, 0.266667, 0.266667, 0.266667, 0.266667, 0.266667, 0.266667, -0.266667,
    ],
    [
        -0.333333, 0.333333, 0.333333, 0.333333, 0.333333, 0.333333, 0.333333, -0.333333,
    ],
    [
        -0.400000, 0.400000, 0.400000, 0.400000, 0.400000, 0.400000, 0.400000, -0.400000,
    ],
    [
        -0.466667, 0.466667, 0.466667, 0.466667, 0.466667, 0.466667, 0.466667, -0.466667,
    ],
    [
        -0.533333, 0.533333, 0.533333, 0.533333, 0.533333, 0.533333, 0.533333, -0.533333,
    ],
    [
        -0.600000, 0.600000, 0.600000, 0.600000, 0.600000, 0.600000, 0.600000, -0.600000,
    ],
    [
        -0.666667, 0.666667, 0.666667, 0.666667, 0.666667, 0.666667, 0.666667, -0.666667,
    ],
    [
        -0.733333, 0.733333, 0.733333, 0.733333, 0.733333, 0.733333, 0.733333, -0.733333,
    ],
    [
        -0.800000, 0.800000, 0.800000, 0.800000, 0.800000, 0.800000, 0.800000, -0.800000,
    ],
    [
        -0.866667, 0.866667, 0.866667, 0.866667, 0.866667, 0.866667, 0.866667, -0.866667,
    ],
    [
        -0.933333, 0.933333, 0.933333, 0.933333, 0.933333, 0.933333, 0.933333, -0.933333,
    ],
    [
        -1.000000, 1.000000, 1.000000, 1.000000, 1.000000, 1.000000, 1.000000, -1.000000,
    ],
];

// Combined lookup table for all duty cycles
const SQUARE_WAVE_TABLES: [[[f32; 8]; 16]; 4] = [
    SQUARE_WAVE_DUTY_0,
    SQUARE_WAVE_DUTY_1,
    SQUARE_WAVE_DUTY_2,
    SQUARE_WAVE_DUTY_3,
];

// Usage: let sample = SQUARE_WAVE_TABLES[duty_cycle][volume][step];

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

pub const NR30_REGISTER: u16 = 0xFF1A; // CH3 DAC Enable
pub const NR31_REGISTER: u16 = 0xFF1B; // CH3 Length
pub const NR32_REGISTER: u16 = 0xFF1C; // CH3 Volume
pub const NR33_REGISTER: u16 = 0xFF1D; // CH3 Period Low
pub const NR34_REGISTER: u16 = 0xFF1E; // CH3 Period High/Control

pub const NR41_REGISTER: u16 = 0xFF20; // CH4 Length
pub const NR42_REGISTER: u16 = 0xFF21; // CH4 Volume/Envelope
pub const NR43_REGISTER: u16 = 0xFF22; // CH4 Frequency/Randomness
pub const NR44_REGISTER: u16 = 0xFF23; // CH4 Control

// Wave RAM registers (FF30-FF3F)
pub const WAVE_RAM_START: u16 = 0xFF30;
pub const WAVE_RAM_END: u16 = 0xFF3F;

pub const NR50_REGISTER: u16 = 0xFF24; // Master Volume/VIN
pub const NR51_REGISTER: u16 = 0xFF25; // Sound Panning
pub const NR52_REGISTER: u16 = 0xFF26; // Master Control

// APU timing constants
const SAMPLE_RATE: u32 = 48000; // Output sample rate
const CPU_FREQUENCY: u32 = 4194304; // Game Boy CPU frequency
const CYCLES_PER_SAMPLE: u32 = CPU_FREQUENCY / SAMPLE_RATE; // ~87 cycles per sample

// Audio buffer constants
const BUFFER_SIZE: usize = 1024; // Target buffer size in samples

// Audio filtering constants
const SMOOTHING_FACTOR: f32 = 0.95; // Low-pass filter coefficient (0.0 = no smoothing, 1.0 = no change)

// High pass filter alpha
const ALPHA: f32 = 0.995;

#[derive(Debug, Clone)]
pub struct HpFilter {
    y_prev: f32, // previous output sample (y[n-1])
    x_prev: f32, // previous input sample  (x[n-1])
}

impl HpFilter {
    /// Create a new filter with zeroed state.
    pub fn new() -> Self {
        Self {
            y_prev: 0.0,
            x_prev: 0.0,
        }
    }

    /// Step the filter: returns the high-pass-filtered sample.
    ///
    /// Formula (single-pole DC blocker):
    ///     y[n] = x[n] − x[n−1] + α · y[n−1]
    /// with α ≈ 0.995 for a ~20 Hz corner at 48 kHz.
    #[inline]
    pub fn step(&mut self, x: f32) -> f32 {
        let y = x - self.x_prev + ALPHA * self.y_prev;
        self.x_prev = x;
        self.y_prev = y;
        y
    }

    /// Reset internal history to silence.
    pub fn reset(&mut self) {
        self.y_prev = 0.0;
        self.x_prev = 0.0;
    }
}

// Simple audio filter for anti-aliasing and smoothing
#[derive(Debug, Clone)]
pub struct AudioFilter {
    last_sample: f32,
    smoothing_factor: f32,
}

impl AudioFilter {
    fn new() -> Self {
        Self {
            last_sample: 0.0,
            smoothing_factor: SMOOTHING_FACTOR,
        }
    }

    fn smooth_sample(&mut self, new_sample: f32) -> f32 {
        self.last_sample =
            self.last_sample * self.smoothing_factor + new_sample * (1.0 - self.smoothing_factor);
        self.last_sample
    }

    fn reset(&mut self) {
        self.last_sample = 0.0;
    }
}

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
    pub ch3_left: bool,
    pub ch3_right: bool,
    pub ch4_left: bool,
    pub ch4_right: bool,

    // Channels
    pub ch1: SquareChannel,
    pub ch2: SquareChannel,
    pub ch3: WaveChannel,
    pub ch4: NoiseChannel,

    // Frame sequencer
    pub frame_sequencer_step: u8,   // 0-7
    pub frame_sequencer_timer: u32, // Timer for frame sequencer

    // Audio buffer and filtering
    pub audio_buffer: Vec<f32>,
    pub sample_accumulator: f32,
    pub sample_count: u32,
    pub audio_filter: AudioFilter,

    pub hp_filter1: HpFilter,
    pub hp_filter2: HpFilter,
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
            ch3_left: false,
            ch3_right: false,
            ch4_left: false,
            ch4_right: false,
            ch1: SquareChannel::new(),
            ch2: SquareChannel::new(),
            ch3: WaveChannel::new(),
            ch4: NoiseChannel::new(),
            frame_sequencer_step: 0,
            frame_sequencer_timer: 0,
            audio_buffer: Vec::with_capacity(BUFFER_SIZE),
            sample_accumulator: 0.0,
            sample_count: 0,
            audio_filter: AudioFilter::new(),
            hp_filter1: HpFilter::new(),
            hp_filter2: HpFilter::new(),
        }
    }

    pub fn is_ch1_enabled(&self) -> bool {
        self.ch1.enabled
    }

    pub fn is_ch2_enabled(&self) -> bool {
        self.ch2.enabled
    }

    pub fn is_ch3_enabled(&self) -> bool {
        self.ch3.enabled
    }

    pub fn is_ch4_enabled(&self) -> bool {
        self.ch4.enabled
    }

    pub fn is_length_clock_step(&self) -> bool {
        matches!(self.frame_sequencer_step, 0 | 2 | 4 | 6)
    }

    pub fn is_envelope_clock_step(&self) -> bool {
        self.frame_sequencer_step == 7
    }

    pub fn frame_sequencer_clock(&mut self) {
        match self.frame_sequencer_step {
            0 => {
                // Length counters
                self.ch1.clock_length();
                self.ch2.clock_length();
                self.ch3.clock_length();
                self.ch4.clock_length();
            }
            1 => {
                // Nothing
            }
            2 => {
                // Length counters and CH1 sweep
                self.ch1.clock_length();
                self.ch2.clock_length();
                self.ch3.clock_length();
                self.ch4.clock_length();
                self.ch1.clock_sweep();
            }
            3 => {
                // Nothing
            }
            4 => {
                // Length counters
                self.ch1.clock_length();
                self.ch2.clock_length();
                self.ch3.clock_length();
                self.ch4.clock_length();
            }
            5 => {
                // Nothing
            }
            6 => {
                // Length counters and CH1 sweep
                self.ch1.clock_length();
                self.ch2.clock_length();
                self.ch3.clock_length();
                self.ch4.clock_length();
                self.ch1.clock_sweep();
            }
            7 => {
                // Envelope
                self.ch1.clock_envelope();
                self.ch2.clock_envelope();
                self.ch4.clock_envelope();
            }
            _ => {}
        }

        self.frame_sequencer_step = (self.frame_sequencer_step + 1) % 8;
    }

    pub fn mix_sample(&mut self) {
        if !self.master_enable {
            return;
        }

        // ───── 1. mix the channels (already in -1.0 … +1.0 range) ─────
        let ch1 = self.ch1.output();
        let ch2 = self.ch2.output();
        let ch3 = self.ch3.output();
        let ch4 = self.ch4.output();

        let mut left = 0.0;
        let mut right = 0.0;

        if self.ch1_left {
            left += ch1
        }
        if self.ch1_right {
            right += ch1
        }
        if self.ch2_left {
            left += ch2
        }
        if self.ch2_right {
            right += ch2
        }
        if self.ch3_left {
            left += ch3
        }
        if self.ch3_right {
            right += ch3
        }
        if self.ch4_left {
            left += ch4
        }
        if self.ch4_right {
            right += ch4
        }

        // apply master volume (0-7 ⇒ 1/8 … 1)
        left *= (self.master_volume_left as f32 + 1.0) / 8.0;
        right *= (self.master_volume_right as f32 + 1.0) / 8.0;

        // down-mix to mono for now
        let raw = (left + right) * 0.5;

        // ───── 2. accumulate over one PCM frame (≈87 T-cycles) ─────
        self.sample_accumulator += raw;
        self.sample_count += 1;

        if self.sample_count >= CYCLES_PER_SAMPLE {
            // average the 87 CPU-rate samples
            let mut sample = self.sample_accumulator / self.sample_count as f32;

            // 2.a low-pass anti-alias (simple leaky integrator)
            sample = self.audio_filter.smooth_sample(sample);

            // 2.b *two* independent first-order high-pass filters (DMG hardware)
            sample = self.hp_filter1.step(sample);
            sample = self.hp_filter2.step(sample);

            // 2.c hard-clamp – keeps SDL happy
            sample = sample.clamp(-1.0, 1.0);

            // push to output FIFO
            self.audio_buffer.push(sample);

            // reset accumulators for the next frame
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
    // One PCM sample corresponds to this many CPU T-cycles at 48 kHz.
    const STEP: usize = CYCLES_PER_SAMPLE as usize; // 4 194 304 / 48 000 ≃ 87
    let mut div_apu_counter: u32 = 0; // DIV-APU runs at 512 Hz

    loop {
        // Sleep until the next audio quantum
        sleep(STEP).await;

        div_apu_counter += STEP as u32;
        let needs_frame_sequencer = if div_apu_counter >= 8192 {
            // 4 419 4304 / 512
            div_apu_counter -= 8192;
            true
        } else {
            false
        };

        let buffer_full = with_state_mut(|state| {
            if !state.apu.master_enable {
                return false; // APU powered down
            }

            for _ in 0..STEP {
                state.apu.ch1.clock_period();
                state.apu.ch2.clock_period();
                state.apu.ch3.clock_period();
                state.apu.ch4.clock_period();

                state.apu.mix_sample(); // one raw DAC sample
            }

            state.apu.is_buffer_full()
        });

        if buffer_full {
            crate::driver::emit_event(crate::driver::Event::AudioBufferFull);
        }

        if needs_frame_sequencer {
            with_state_mut(|state| state.apu.frame_sequencer_clock());
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

        NR30_REGISTER => {
            (if apu_state.ch3.dac_enabled { 0x80 } else { 0 }) | 0x7F // Most bits read as 1
        }
        NR31_REGISTER => 0xFF, // Write-only
        NR32_REGISTER => {
            (apu_state.ch3.volume_code << 5) | 0x9F // Most bits read as 1
        }
        NR33_REGISTER => 0xFF, // Write-only
        NR34_REGISTER => {
            (if apu_state.ch3.length_enabled {
                0x40
            } else {
                0
            }) | 0xBF // Most bits read as 1
        }

        NR41_REGISTER => 0xFF, // Write-only
        NR42_REGISTER => apu_state.ch4.read_volume_envelope(),
        NR43_REGISTER => apu_state.ch4.read_frequency_randomness(),
        NR44_REGISTER => apu_state.ch4.read_control(),

        NR50_REGISTER => {
            (if apu_state.vin_left { 0x80 } else { 0 })
                | (apu_state.master_volume_left << 4)
                | (if apu_state.vin_right { 0x08 } else { 0 })
                | apu_state.master_volume_right
        }
        NR51_REGISTER => {
            (if apu_state.ch4_left { 0x80 } else { 0 })
                | (if apu_state.ch3_left { 0x40 } else { 0 })
                | (if apu_state.ch2_left { 0x20 } else { 0 })
                | (if apu_state.ch1_left { 0x10 } else { 0 })
                | (if apu_state.ch4_right { 0x08 } else { 0 })
                | (if apu_state.ch3_right { 0x04 } else { 0 })
                | (if apu_state.ch2_right { 0x02 } else { 0 })
                | (if apu_state.ch1_right { 0x01 } else { 0 })
        }
        NR52_REGISTER => {
            (if apu_state.master_enable { 0x80 } else { 0 })
                | (if apu_state.is_ch4_enabled() { 0x08 } else { 0 })
                | (if apu_state.is_ch3_enabled() { 0x04 } else { 0 })
                | (if apu_state.is_ch2_enabled() { 0x02 } else { 0 })
                | (if apu_state.is_ch1_enabled() { 0x01 } else { 0 })
                | 0x70 // Bits 4-6 always read as 1
        }

        WAVE_RAM_START..=WAVE_RAM_END => apu_state.ch3.read_wave_ram(address),

        _ => 0xFF, // Unused registers
    }
}

pub fn write_apu_register(address: u16, value: u8, apu_state: &mut ApuState) {
    if !apu_state.master_enable && address != NR52_REGISTER {
        return; // Ignore writes when APU is disabled
    }

    match address {
        NR10_REGISTER => {
            let old_direction = apu_state.ch1.sweep_direction;
            apu_state.ch1.sweep_pace = (value >> 4) & 0x07;
            apu_state.ch1.sweep_direction = (value & 0x08) != 0;
            apu_state.ch1.sweep_step = value & 0x07;
            
            // Clearing the sweep direction bit after subtraction calculation disables the channel
            if old_direction && !apu_state.ch1.sweep_direction && apu_state.ch1.sweep_calculation_made {
                apu_state.ch1.enabled = false;
            }
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
            let old_length_enabled = apu_state.ch1.length_enabled;
            let trigger = (value & 0x80) != 0;
            apu_state.ch1.length_enabled = (value & 0x40) != 0;
            apu_state.ch1.period = (apu_state.ch1.period & 0x00FF) | (((value & 0x07) as u16) << 8);

            // Extra length clocking
            if !apu_state.is_length_clock_step() && !old_length_enabled && apu_state.ch1.length_enabled {
                if apu_state.ch1.length_timer > 0 && !trigger {
                    apu_state.ch1.length_timer -= 1;
                    if apu_state.ch1.length_timer == 0 {
                        apu_state.ch1.enabled = false;
                    }
                }
            }

            if trigger {
                // Check for length = 63 instead of 64
                let should_adjust_length = !apu_state.is_length_clock_step() && 
                                         apu_state.ch1.length_enabled && 
                                         apu_state.ch1.length_timer == 0;
                
                apu_state.ch1.trigger(true); // CH1 has sweep
                
                if should_adjust_length && apu_state.ch1.length_timer == 64 {
                    apu_state.ch1.length_timer = 63;
                }

                // Check for envelope timer +1
                if apu_state.is_envelope_clock_step() && apu_state.ch1.envelope_pace != 0 {
                    apu_state.ch1.envelope_timer = apu_state.ch1.envelope_timer.saturating_add(1);
                }
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
            let old_length_enabled = apu_state.ch2.length_enabled;
            let trigger = (value & 0x80) != 0;
            apu_state.ch2.length_enabled = (value & 0x40) != 0;
            apu_state.ch2.period = (apu_state.ch2.period & 0x00FF) | (((value & 0x07) as u16) << 8);

            // Extra length clocking
            if !apu_state.is_length_clock_step() && !old_length_enabled && apu_state.ch2.length_enabled {
                if apu_state.ch2.length_timer > 0 && !trigger {
                    apu_state.ch2.length_timer -= 1;
                    if apu_state.ch2.length_timer == 0 {
                        apu_state.ch2.enabled = false;
                    }
                }
            }

            if trigger {
                // Check for length = 63 instead of 64
                let should_adjust_length = !apu_state.is_length_clock_step() && 
                                         apu_state.ch2.length_enabled && 
                                         apu_state.ch2.length_timer == 0;
                
                apu_state.ch2.trigger(false); // CH2 has no sweep
                
                if should_adjust_length && apu_state.ch2.length_timer == 64 {
                    apu_state.ch2.length_timer = 63;
                }

                // Check for envelope timer +1
                if apu_state.is_envelope_clock_step() && apu_state.ch2.envelope_pace != 0 {
                    apu_state.ch2.envelope_timer = apu_state.ch2.envelope_timer.saturating_add(1);
                }
            }
        }

        NR30_REGISTER => {
            apu_state.ch3.update_dac(value);
        }
        NR31_REGISTER => {
            apu_state.ch3.set_length_timer(value);
        }
        NR32_REGISTER => {
            apu_state.ch3.set_volume_code(value);
        }
        NR33_REGISTER => {
            apu_state.ch3.set_period_low(value);
        }
        NR34_REGISTER => {
            let old_length_enabled = apu_state.ch3.length_enabled;
            let trigger = (value & 0x80) != 0;
            apu_state.ch3.set_length_enabled(value);
            apu_state.ch3.set_period_high(value);

            // Extra length clocking
            if !apu_state.is_length_clock_step() && !old_length_enabled && apu_state.ch3.length_enabled {
                if apu_state.ch3.length_timer > 0 && !trigger {
                    apu_state.ch3.length_timer -= 1;
                    if apu_state.ch3.length_timer == 0 {
                        apu_state.ch3.enabled = false;
                    }
                }
            }

            if trigger {
                // Check for length = 255 instead of 256 (wave channel max)
                let should_adjust_length = !apu_state.is_length_clock_step() && 
                                         apu_state.ch3.length_enabled && 
                                         apu_state.ch3.length_timer == 0;
                
                apu_state.ch3.trigger();
                
                if should_adjust_length && apu_state.ch3.length_timer == 256 {
                    apu_state.ch3.length_timer = 255;
                }
            }
        }

        NR41_REGISTER => {
            apu_state.ch4.set_length_timer(value);
        }
        NR42_REGISTER => {
            apu_state.ch4.set_volume_envelope(value);
        }
        NR43_REGISTER => {
            apu_state.ch4.set_frequency_randomness(value);
        }
        NR44_REGISTER => {
            let old_length_enabled = apu_state.ch4.length_enabled;
            let trigger = (value & 0x80) != 0;
            
            // Extra length clocking - do this before set_control
            if !apu_state.is_length_clock_step() && !old_length_enabled && (value & 0x40) != 0 {
                if apu_state.ch4.length_timer > 0 && !trigger {
                    apu_state.ch4.length_timer -= 1;
                    if apu_state.ch4.length_timer == 0 {
                        apu_state.ch4.enabled = false;
                    }
                }
            }

            // Check for length = 63 instead of 64
            let should_adjust_length = trigger && !apu_state.is_length_clock_step() && 
                                     (value & 0x40) != 0 && 
                                     apu_state.ch4.length_timer == 0;

            // Check for envelope timer +1
            let should_adjust_envelope = trigger && apu_state.is_envelope_clock_step() && 
                                        apu_state.ch4.envelope_pace != 0;

            apu_state.ch4.set_control(value);
            
            if should_adjust_length && apu_state.ch4.length_timer == 64 {
                apu_state.ch4.length_timer = 63;
            }

            if should_adjust_envelope {
                apu_state.ch4.envelope_timer = apu_state.ch4.envelope_timer.saturating_add(1);
            }
        }

        NR50_REGISTER => {
            apu_state.vin_left = (value & 0x80) != 0;
            apu_state.master_volume_left = (value >> 4) & 0x07;
            apu_state.vin_right = (value & 0x08) != 0;
            apu_state.master_volume_right = value & 0x07;
        }
        NR51_REGISTER => {
            apu_state.ch4_left = (value & 0x80) != 0;
            apu_state.ch3_left = (value & 0x40) != 0;
            apu_state.ch2_left = (value & 0x20) != 0;
            apu_state.ch1_left = (value & 0x10) != 0;
            apu_state.ch4_right = (value & 0x08) != 0;
            apu_state.ch3_right = (value & 0x04) != 0;
            apu_state.ch2_right = (value & 0x02) != 0;
            apu_state.ch1_right = (value & 0x01) != 0;
        }
        NR52_REGISTER => {
            let old_enable = apu_state.master_enable;
            apu_state.master_enable = (value & 0x80) != 0;

            if !apu_state.master_enable {
                // APU turned off - reset all registers, but keep master_enable state
                let master_enable = apu_state.master_enable;
                *apu_state = ApuState::new();
                apu_state.master_enable = master_enable;
            } else if !old_enable {
                // APU turned ON - reset frame sequencer
                apu_state.frame_sequencer_step = 0;
                apu_state.frame_sequencer_timer = 8192; // Schedule first tick
            }
        }

        WAVE_RAM_START..=WAVE_RAM_END => {
            apu_state.ch3.write_wave_ram(address, value);
        }

        _ => {} // Unused registers
    }
}
