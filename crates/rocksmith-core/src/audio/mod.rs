//! Duplex Audio I/O layer with MockAudioBackend and cross-platform traits.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use rtrb::{Consumer, Producer, RingBuffer};

pub struct DuplexAudioConfig {
    pub sample_rate: u32,
    pub input_buffer_size: usize,
    pub output_buffer_size: usize,
    pub channels: u16,
}

impl Default for DuplexAudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            input_buffer_size: 256,
            output_buffer_size: 512,
            channels: 2,
        }
    }
}

pub trait AudioBackend: Send + Sync {
    fn start(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
    fn is_running(&self) -> bool;
    fn get_processed_sample_count(&self) -> u64;
}

/// Deterministic mock audio backend for headless CI testing (T-011, T-015a)
pub struct MockAudioBackend {
    config: DuplexAudioConfig,
    is_running: Arc<AtomicBool>,
    processed_samples: Arc<AtomicU64>,
    input_producer: Producer<f32>,
    output_consumer: Consumer<f32>,
    simulated_failure: bool,
}

impl MockAudioBackend {
    pub fn new(config: DuplexAudioConfig) -> (Self, Consumer<f32>, Producer<f32>) {
        let (in_prod, in_cons) = RingBuffer::new(config.input_buffer_size * 16);
        let (out_prod, out_cons) = RingBuffer::new(config.output_buffer_size * 16);

        let backend = Self {
            config,
            is_running: Arc::new(AtomicBool::new(false)),
            processed_samples: Arc::new(AtomicU64::new(0)),
            input_producer: in_prod,
            output_consumer: out_cons,
            simulated_failure: false,
        };

        (backend, in_cons, out_prod)
    }

    pub fn inject_input_samples(&mut self, samples: &[f32]) -> usize {
        let mut count = 0;
        for &s in samples {
            if self.input_producer.push(s).is_ok() {
                count += 1;
            }
        }
        self.processed_samples.fetch_add(count as u64, Ordering::Relaxed);
        count
    }

    pub fn simulate_device_disconnect(&mut self) {
        self.simulated_failure = true;
        self.is_running.store(false, Ordering::SeqCst);
    }
}

impl AudioBackend for MockAudioBackend {
    fn start(&mut self) -> Result<(), String> {
        if self.simulated_failure {
            return Err("Simulated audio endpoint failure: device disconnected".to_string());
        }
        self.is_running.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        self.is_running.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    fn get_processed_sample_count(&self) -> u64 {
        self.processed_samples.load(Ordering::Relaxed)
    }
}
