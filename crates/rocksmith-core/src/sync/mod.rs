//! Audio-Clock Synchronization and Dual Offset Calibration (Decision D-007, S-008).

#[derive(Debug, Clone)]
pub struct ClockSynchronizer {
    sample_rate: u32,
    audio_offset_ms: i32,  // Hardware roundtrip buffer calibration (-200 to +200 ms)
    visual_offset_ms: i32, // Display refresh & GPU buffering lag (-200 to +200 ms)
}

impl ClockSynchronizer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            audio_offset_ms: 0,
            visual_offset_ms: 0,
        }
    }

    pub fn set_audio_offset_ms(&mut self, offset: i32) {
        self.audio_offset_ms = offset.clamp(-200, 200);
    }

    pub fn set_visual_offset_ms(&mut self, offset: i32) {
        self.visual_offset_ms = offset.clamp(-200, 200);
    }

    /// Converts the continuous hardware processed sample count to exact audio song time in ms
    #[inline]
    pub fn samples_to_audio_time_ms(&self, sample_count: u64) -> u64 {
        let raw_ms = (sample_count as f64 * 1000.0 / self.sample_rate as f64) as i64;
        (raw_ms + self.audio_offset_ms as i64).max(0) as u64
    }

    /// Converts the hardware audio clock to visual rendering highway position in ms
    #[inline]
    pub fn samples_to_visual_time_ms(&self, sample_count: u64) -> u64 {
        let audio_ms = self.samples_to_audio_time_ms(sample_count) as i64;
        (audio_ms + self.visual_offset_ms as i64).max(0) as u64
    }
}
