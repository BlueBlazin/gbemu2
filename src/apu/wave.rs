#[derive(Debug, Clone)]
pub struct WaveChannel {
    // Channel state
    pub enabled: bool,
    pub dac_enabled: bool,

    // Period/frequency
    pub period: u16,       // 11-bit period value
    pub period_timer: u16, // Current period timer

    // Length timer (8-bit, 0-255, max 256)
    pub length_timer: u16,    // Current length timer value (0-256)
    pub length_enabled: bool, // Length timer enable

    // Volume control (2-bit output level)
    pub volume_code: u8, // 0-3 (00=mute, 01=100%, 10=50%, 11=25%)

    // Wave RAM and sample playback
    pub wave_ram: [u8; 16], // 16 bytes of wave RAM (FF30-FF3F)
    pub sample_buffer: u8,  // Last read sample (continuously emitted)
    pub sample_index: u8,   // Current position in 32-sample waveform (0-31)

    ram_busy: u8,
}

impl Default for WaveChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl WaveChannel {
    pub const fn new() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
            period: 0,
            period_timer: 0,
            length_timer: 0,
            length_enabled: false,
            volume_code: 0,
            wave_ram: [0; 16],
            sample_buffer: 0, // This buffer is cleared when APU is turned on
            sample_index: 0,
            ram_busy: 0,
        }
    }

    pub fn trigger(&mut self) {
        // If channel was enabled and reading from wave RAM, corruption occurs
        if self.enabled {
            self.corrupt_wave_ram();
        }

        // Ch3 is enabled (if DAC is on)
        self.enabled = self.dac_enabled;

        // If the length timer expired it is reset
        if self.length_timer == 0 {
            self.length_timer = 256; // Max length for wave channel (256)
        }

        // The period divider is set to the contents of NR33 and NR34
        self.period_timer = (2048 - self.period) * 2; // 2 T-cycles per period step

        // Wave RAM index is reset, but its not refilled
        // The first sample to play is the previous one still in the sample buffer
        // Sample #0 is skipped when first incrementing
        self.sample_index = 0;

        // Don't read sample yet - the buffer retains whatever was last read
        // This means the first sample played after trigger is the old buffer contents
    }

    pub fn clock_period(&mut self) {
        // count down the busy window each T-cycle
        if self.ram_busy > 0 {
            self.ram_busy -= 1;
        }

        if !self.enabled {
            return;
        }

        if self.period_timer > 0 {
            self.period_timer -= 1;
        }

        if self.period_timer == 0 {
            self.period_timer = (2048 - self.period) * 2; // Reset to full period (2 T-cycles)

            // Advance sample index - this increments at the channel's sample rate
            self.sample_index = (self.sample_index + 1) % 32;

            // Read the new sample from wave RAM and store in buffer
            if self.enabled {
                // Only read if enabled to prevent RAM corruption issues
                self.sample_buffer = self.read_sample();
                self.ram_busy = 2; // block CPU for the next 2 cycles
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
        if !self.enabled || !self.dac_enabled {
            return 0.0;
        }

        // Apply volume control by shifting the digital value
        let digital_value = match self.volume_code {
            0 => 0,                       // Mute (No sound)
            1 => self.sample_buffer,      // 100% volume (use samples as-is)
            2 => self.sample_buffer >> 1, // 50% volume (shift right once)
            3 => self.sample_buffer >> 2, // 25% volume (shift right twice)
            _ => 0,
        };

        // Convert 4-bit digital value (0-15) to normalized float
        // Game Boy DAC: digital 0 -> analog 1, digital 15 -> analog -1 (inverted)
        // So we need: 1.0 - (digital_value / 7.5)
        1.0 - (digital_value as f32 / 7.5)
    }

    pub fn update_dac(&mut self, nr30_value: u8) {
        let new_dac_enabled = (nr30_value & 0x80) != 0;

        // Turning the DAC OFF forces the output to 0 V and clears the internal
        // 4-bit sample latch on the DMG.  If we don’t replicate that, the next
        // trigger starts with stale data and the first note can vanish.
        if !new_dac_enabled && self.dac_enabled {
            self.enabled = false; // channel goes silent
            self.sample_buffer = 0; // hardware clears the latch here
        }

        self.dac_enabled = new_dac_enabled;
    }

    pub fn read_wave_ram(&self, address: u16) -> u8 {
        // When channel is active, access is only allowed if the APU just read a sample.
        if self.enabled && self.ram_busy > 0 {
            return 0xFF; // Blocked access reads FF
        }

        // If channel is disabled OR it's the right time, allow access.
        // The reference also redirects reads to the current playing sample's byte.
        let read_index = if self.enabled {
            // If enabled, hardware redirects the read to the current sample byte
            (self.sample_index / 2) as usize
        } else {
            (address - 0xFF30) as usize
        };

        if read_index < 16 {
            self.wave_ram[read_index]
        } else {
            0xFF
        }
    }

    pub fn write_wave_ram(&mut self, address: u16, value: u8) {
        // When channel is active, access is only allowed if the APU just read a sample.
        if self.enabled && self.ram_busy > 0 {
            return; // Blocked writes are ignored
        }

        // If channel is disabled OR it's the right time, allow access.
        // The reference redirects writes in the same way as reads.
        let write_index = if self.enabled {
            // If enabled, hardware redirects the write to the current sample byte
            (self.sample_index / 2) as usize
        } else {
            (address - 0xFF30) as usize
        };

        if write_index < 16 {
            self.wave_ram[write_index] = value;
        }
    }

    // Helper method to read current sample from wave RAM
    fn read_sample(&self) -> u8 {
        let byte_index = (self.sample_index / 2) as usize;
        let nibble_index = self.sample_index % 2;

        let byte = self.wave_ram[byte_index];
        if nibble_index == 0 {
            // Upper nibble (first sample in byte)
            (byte >> 4) & 0x0F
        } else {
            // Lower nibble (second sample in byte)
            byte & 0x0F
        }
    }

    // Helper method to set volume code from register value
    pub fn set_volume_code(&mut self, nr32_value: u8) {
        self.volume_code = (nr32_value >> 5) & 0x03;
    }

    // Helper method to set length timer from register value
    pub fn set_length_timer(&mut self, nr31_value: u8) {
        self.length_timer = 256 - (nr31_value as u16);
    }

    // Helper method to set period from register values
    pub fn set_period_low(&mut self, nr33_value: u8) {
        self.period = (self.period & 0x0700) | (nr33_value as u16);
    }

    pub fn set_period_high(&mut self, nr34_value: u8) {
        self.period = (self.period & 0x00FF) | (((nr34_value & 0x07) as u16) << 8);
    }

    // Helper method to check if length should be enabled
    pub fn set_length_enabled(&mut self, nr34_value: u8) {
        self.length_enabled = (nr34_value & 0x40) != 0;
    }

    // Wave RAM corruption when triggering while channel is reading
    fn corrupt_wave_ram(&mut self) {
        let byte_index = (self.sample_index / 2) as usize;

        if byte_index < 4 {
            // If reading one of the first four bytes, only first byte is rewritten
            let byte_value = self.wave_ram[byte_index];
            self.wave_ram[0] = byte_value;
        } else {
            // If reading one of the later 12 bytes, the first FOUR bytes are rewritten
            // with the four aligned bytes that the read was from
            let aligned_start = (byte_index / 4) * 4; // 4, 8, or 12
            for i in 0..4 {
                if aligned_start + i < 16 {
                    self.wave_ram[i] = self.wave_ram[aligned_start + i];
                }
            }
        }
    }
}
