// Import precomputed sample tables from parent module
use super::SQUARE_WAVE_TABLES;

#[derive(Debug, Clone)]
pub struct SquareChannel {
    // Channel state
    pub enabled: bool,
    pub dac_enabled: bool,
    pub first_trigger: bool, // Track if channel has been triggered since power on

    // Duty and waveform
    pub duty: u8,      // 0-3 (duty cycle selection)
    pub duty_step: u8, // 0-7 (position in duty cycle)

    // Period/frequency
    pub period: u16,       // 11-bit period value
    pub period_timer: u16, // Current period timer

    // Volume and envelope
    pub volume: u8,               // Current volume 0-15
    pub initial_volume: u8,       // Initial volume from register
    pub envelope_direction: bool, // false = decrease, true = increase
    pub envelope_pace: u8,        // 0-7 (0 = disabled)
    pub envelope_timer: u8,       // Current envelope timer

    // Length timer
    pub length_timer: u8,     // Current length timer value
    pub length_enabled: bool, // Length timer enable

    // Sweep (CH1 only)
    pub sweep_enabled: bool,
    pub sweep_pace: u8,               // 0-7 (0 = disabled)
    pub sweep_direction: bool,        // false = addition, true = subtraction
    pub sweep_step: u8,               // 0-7
    pub sweep_timer: u8,              // Current sweep timer
    pub sweep_shadow: u16,            // Shadow register for sweep
    pub sweep_calculation_made: bool, // Track if calculation was made in subtraction mode
}

impl Default for SquareChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl SquareChannel {
    pub const fn new() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
            first_trigger: false,
            duty: 0,
            duty_step: 0,
            period: 0,
            period_timer: 0,
            volume: 0,
            initial_volume: 0,
            envelope_direction: false,
            envelope_pace: 0,
            envelope_timer: 0,
            length_timer: 0,
            length_enabled: false,
            sweep_enabled: false,
            sweep_pace: 0,
            sweep_direction: false,
            sweep_step: 0,
            sweep_timer: 0,
            sweep_shadow: 0,
            sweep_calculation_made: false,
        }
    }

    pub fn trigger(&mut self, has_sweep: bool) {
        self.enabled = self.dac_enabled;
        self.first_trigger = true;

        // Reset length timer if it was 0
        if self.length_timer == 0 {
            self.length_timer = 64; // Max length for square channels
        }

        self.period_timer = (2048 - self.period) * 4; // low bits forced to 0

        // Reset envelope
        self.envelope_timer = if self.envelope_pace == 0 {
            8
        } else {
            self.envelope_pace
        };
        self.volume = self.initial_volume;

        // Reset sweep (CH1 only)
        if has_sweep {
            self.sweep_shadow = self.period;
            self.sweep_timer = if self.sweep_pace == 0 {
                8
            } else {
                self.sweep_pace
            };
            self.sweep_enabled = self.sweep_pace != 0 || self.sweep_step != 0;
            self.sweep_calculation_made = false; // Reset calculation tracking

            if self.sweep_step != 0 {
                self.calculate_sweep_frequency();
            }
        }
    }

    pub fn calculate_sweep_frequency(&mut self) -> bool {
        let new_freq = if self.sweep_direction {
            // Mark that we made a calculation in subtraction mode
            self.sweep_calculation_made = true;
            self.sweep_shadow
                .saturating_sub(self.sweep_shadow >> self.sweep_step)
        } else {
            self.sweep_shadow + (self.sweep_shadow >> self.sweep_step)
        };

        if new_freq > 2047 {
            self.enabled = false;
            return false; // Overflow
        }

        if self.sweep_step != 0 {
            self.sweep_shadow = new_freq;
            self.period = new_freq;
        }

        true
    }

    // Calculate new sweep frequency but don't write back (for second calculation)
    fn calculate_sweep_frequency_no_writeback(&self) -> bool {
        let new_freq = if self.sweep_direction {
            self.sweep_shadow
                .saturating_sub(self.sweep_shadow >> self.sweep_step)
        } else {
            self.sweep_shadow + (self.sweep_shadow >> self.sweep_step)
        };

        new_freq <= 2047
    }

    pub fn clock_sweep(&mut self) {
        if self.sweep_timer > 0 {
            self.sweep_timer -= 1;
        }

        if self.sweep_timer == 0 {
            self.sweep_timer = if self.sweep_pace == 0 {
                8
            } else {
                self.sweep_pace
            };

            if self.sweep_enabled && self.sweep_pace != 0 && self.calculate_sweep_frequency() {
                // Calculate again immediately but don't write back - check for overflow again
                if !self.calculate_sweep_frequency_no_writeback() {
                    self.enabled = false;
                }
            }
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

    pub fn clock_period(&mut self) {
        // Duty cycle clocking is disabled until the first trigger
        if !self.first_trigger {
            return;
        }

        if self.period_timer > 0 {
            self.period_timer -= 1;
        }

        if self.period_timer == 0 {
            self.period_timer = (2048 - self.period) * 4;
            self.duty_step = (self.duty_step + 1) % 8;
        }
    }

    pub fn output(&self) -> f32 {
        if !self.enabled || !self.dac_enabled {
            return 0.0;
        }

        // First duty step after power on is played as if it were 0
        let effective_duty_step = if !self.first_trigger && self.duty_step == 0 {
            0 // Play as duty cycle 0 (low)
        } else {
            self.duty_step
        };

        // Use precomputed sample tables for direct float output
        SQUARE_WAVE_TABLES[self.duty as usize][self.volume as usize][effective_duty_step as usize]
    }

    pub fn update_dac(&mut self, nr2_value: u8) {
        self.dac_enabled = (nr2_value & 0xF8) != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }
}
