// Corrected noise channel divider values
const NOISE_DIVISORS: [u32; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

#[derive(Debug, Clone)]
pub struct NoiseChannel {
    // Channel state
    pub enabled: bool,
    pub dac_enabled: bool,

    // LFSR (Linear Feedback Shift Register)
    pub lfsr: u16,       // 15-bit shift register
    pub lfsr_7bit: bool, // 7-bit mode flag

    // Frequency control
    pub clock_shift: u8,   // 0-15 (shift amount)
    pub clock_divider: u8, // 0-7 (divider code)
    pub period_timer: u32, // Current period timer

    // Volume and envelope
    pub volume: u8,               // Current volume 0-15
    pub initial_volume: u8,       // Initial volume from register
    pub envelope_direction: bool, // false = decrease, true = increase
    pub envelope_pace: u8,        // 0-7 (0 = disabled)
    pub envelope_timer: u8,       // Current envelope timer

    // Length timer
    pub length_timer: u16, // Current length timer value (u16 for consistency)
    pub length_enabled: bool, // Length timer enable
}

impl Default for NoiseChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl NoiseChannel {
    pub fn new() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
            lfsr: 0xFFFF, // Initialized to all 1s
            lfsr_7bit: false,
            clock_shift: 0,
            clock_divider: 0,
            period_timer: 0,
            volume: 0,
            initial_volume: 0,
            envelope_direction: false,
            envelope_pace: 0,
            envelope_timer: 0,
            length_timer: 0,
            length_enabled: false,
        }
    }

    pub fn trigger(&mut self) {
        self.enabled = self.dac_enabled;

        // Reset length timer if it was 0
        if self.length_timer == 0 {
            self.length_timer = 64; // Max length
        }

        // Reset period timer
        self.period_timer = self.calculate_period();

        // Reset envelope
        self.envelope_timer = if self.envelope_pace == 0 {
            8
        } else {
            self.envelope_pace
        };
        self.volume = self.initial_volume;

        self.lfsr = 0x7FFF
    }

    fn calculate_period(&self) -> u32 {
        (NOISE_DIVISORS[self.clock_divider as usize] << self.clock_shift) * 16
    }

    pub fn clock_period(&mut self) {
        // Using a noise channel clock shift of 14 or 15 results in the LFSR receiving no clocks
        if self.clock_shift >= 14 {
            return;
        }

        if self.period_timer > 0 {
            self.period_timer -= 1;
        }

        if self.period_timer == 0 {
            self.period_timer = self.calculate_period();
            if self.enabled {
                self.tick_lfsr();
            }
        }
    }

    fn tick_lfsr(&mut self) {
        // Get the XOR of bits 0 and 1
        let xor_result = (self.lfsr & 0x01) ^ ((self.lfsr & 0x02) >> 1);

        // Shift right
        self.lfsr >>= 1;

        // Set bit 14 (new MSB of 15-bit LFSR) to the XOR result
        self.lfsr |= xor_result << 14;

        // In 7-bit mode, also copy the result to bit 6
        if self.lfsr_7bit {
            self.lfsr &= !(1 << 6); // Clear bit 6
            self.lfsr |= xor_result << 6;
        }
    }

    pub fn clock_envelope(&mut self) {
        if self.envelope_pace == 0 {
            return;
        }

        if self.envelope_timer > 0 {
            self.envelope_timer -= 1;
        }

        if self.envelope_timer == 0 {
            self.envelope_timer = if self.envelope_pace == 0 {
                8
            } else {
                self.envelope_pace
            };

            if self.envelope_direction && self.volume < 15 {
                self.volume += 1;
            } else if !self.envelope_direction && self.volume > 0 {
                self.volume -= 1;
            }
        }
    }

    pub fn clock_length(&mut self) {
        if self.length_enabled && self.length_timer > 0 {
            self.length_timer -= 1;
            if self.length_timer == 0 {
                self.enabled = false;
            }
        }
    }

    pub fn output(&self) -> f32 {
        if !self.enabled || !self.dac_enabled || self.volume == 0 {
            return 0.0;
        }

        // 4-bit DAC: 0 → +1.0 , 15 → –1.0  (inverted)
        let high = 1.0 - (self.volume as f32 / 7.5);
        let low = 1.0; // digital 0 maps to analogue +1 V

        if (self.lfsr & 1) == 0 { high } else { low }
    }

    pub fn update_dac(&mut self, nr42_value: u8) {
        // DAC enabled when bits 3-7 are not all 0 (initial volume != 0 OR envelope increasing)
        self.dac_enabled = (nr42_value & 0xF8) != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    // Register interface methods
    pub fn set_length_timer(&mut self, value: u8) {
        self.length_timer = 64 - (value & 0x3F) as u16;
    }

    pub fn set_volume_envelope(&mut self, value: u8) {
        self.initial_volume = (value >> 4) & 0x0F;
        self.envelope_direction = (value & 0x08) != 0;
        self.envelope_pace = value & 0x07;
        self.update_dac(value);
    }

    pub fn set_frequency_randomness(&mut self, value: u8) {
        self.clock_shift = (value >> 4) & 0x0F;
        self.lfsr_7bit = (value & 0x08) != 0;
        self.clock_divider = value & 0x07;
    }

    pub fn set_control(&mut self, value: u8) {
        self.length_enabled = (value & 0x40) != 0;

        if (value & 0x80) != 0 {
            self.trigger();
        }
    }

    // Register read methods
    pub fn read_volume_envelope(&self) -> u8 {
        (self.initial_volume << 4)
            | (if self.envelope_direction { 0x08 } else { 0 })
            | self.envelope_pace
    }

    pub fn read_frequency_randomness(&self) -> u8 {
        (self.clock_shift << 4) | (if self.lfsr_7bit { 0x08 } else { 0 }) | self.clock_divider
    }

    pub fn read_control(&self) -> u8 {
        (if self.length_enabled { 0x40 } else { 0 }) | 0xBF // Most bits read as 1
    }
}
