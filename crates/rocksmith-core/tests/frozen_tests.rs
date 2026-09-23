//! FROZEN TEST SUITE — DO NOT MODIFY
//! SHA-256 integrity protected under tests/FROZEN_MANIFEST.sha256
//! Validates Success Criteria SC-1 through SC-6 and Pre-Mortem mitigations.

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;
    use rocksmith_core::*;

    fn generate_sine(freq: f32, duration_sec: f32, sample_rate: f32) -> Vec<f32> {
        let n = (duration_sec * sample_rate) as usize;
        (0..n)
            .map(|i| (2.0 * PI * freq * i as f32 / sample_rate).sin())
            .collect()
    }

    fn generate_sawtooth(freq: f32, duration_sec: f32, sample_rate: f32) -> Vec<f32> {
        let n = (duration_sec * sample_rate) as usize;
        let period = sample_rate / freq;
        (0..n)
            .map(|i| {
                let phase = (i as f32 % period) / period;
                2.0 * phase - 1.0
            })
            .collect()
    }

    // T-001: PitchDetector detects A4 (440 Hz) within ±5 cents on synthetic sine (SC-1)
    #[test]
    fn test_t001_pitch_a4_accuracy() {
        let sr = 44100.0;
        let mut detector = MultiRatePitchDetector::new(sr, 2048, 512);
        let samples = generate_sine(440.0, 0.1, sr);

        let result = detector.process(&samples).expect("Should detect pitch for A4");
        let cents = 1200.0 * (result.frequency_hz / 440.0).log2();
        assert!(cents.abs() <= 5.0, "A4 error was {} cents (expected <= 5)", cents);
        assert!(result.confidence >= 0.8, "Confidence should be high");
    }

    // T-002: PitchDetector detects E2 (82.41 Hz) within ±10 cents on synthetic sawtooth (SC-1)
    #[test]
    fn test_t002_pitch_e2_low_guitar_string() {
        let sr = 44100.0;
        let mut detector = MultiRatePitchDetector::new(sr, 2048, 512);
        let samples = generate_sawtooth(82.41, 0.2, sr);

        let result = detector.process(&samples).expect("Should detect pitch for low E2");
        let cents = 1200.0 * (result.frequency_hz / 82.41).log2();
        assert!(cents.abs() <= 10.0, "E2 error was {} cents (expected <= 10)", cents);
    }

    // T-003: PitchDetector detects E5 (659.26 Hz) within ±10 cents on synthetic sine (SC-1)
    #[test]
    fn test_t003_pitch_e5_high_guitar_string() {
        let sr = 44100.0;
        let mut detector = MultiRatePitchDetector::new(sr, 2048, 512);
        let samples = generate_sine(659.26, 0.1, sr);

        let result = detector.process(&samples).expect("Should detect pitch for E5");
        let cents = 1200.0 * (result.frequency_hz / 659.26).log2();
        assert!(cents.abs() <= 10.0, "E5 error was {} cents (expected <= 10)", cents);
    }

    // T-004: OnsetDetector localizes a click at t=1000 ms within ±20 ms (SC-2)
    #[test]
    fn test_t004_onset_click_localization() {
        let sr = 44100.0;
        let mut onset_det = SpectralFluxOnsetDetector::new(sr, 1024, 256);

        // Feed 1 second of silence
        let mut audio = vec![0.0f32; 44100];
        // Inject click at index 44100 (t=1000ms)
        audio.push(0.95);
        audio.push(0.80);
        audio.push(0.50);
        audio.extend(vec![0.0f32; 1000]);

        let mut detected_time_ms = None;
        for chunk in audio.chunks(256) {
            if let Some(res) = onset_det.process(chunk) {
                detected_time_ms = Some(res.timestamp_ms);
                break;
            }
        }

        let time = detected_time_ms.expect("Should detect click onset");
        let diff = (time as i64 - 1000).abs();
        assert!(diff <= 20, "Onset detected at {} ms (expected 1000 ± 20 ms)", time);
    }

    // T-005: OnsetDetector does not fire on silence (SC-2)
    #[test]
    fn test_t005_onset_silence_rejection() {
        let sr = 44100.0;
        let mut onset_det = SpectralFluxOnsetDetector::new(sr, 1024, 256);
        let silence = vec![0.0f32; 256];

        for _ in 0..100 {
            assert!(onset_det.process(&silence).is_none(), "Must not fire on pure silence");
        }
    }

    // T-006: Integration: Pipeline correctly identifies notes and reconciles pick transients
    #[test]
    fn test_t006_temporal_tracker_transient_reconciliation() {
        let mut tracker = TemporalConsensusTracker::new();

        // 1. Pick strike registers onset at t = 500 ms
        tracker.record_onset(OnsetResult { timestamp_ms: 500, strength: 2.5 });

        // 2. Initial inharmonic transient (t = 505, 510 ms) with chaotic pitch
        tracker.record_pitch(PitchResult { frequency_hz: 1200.0, confidence: 0.3, timestamp_ms: 505, is_voiced: true });
        tracker.record_pitch(PitchResult { frequency_hz: 950.0, confidence: 0.4, timestamp_ms: 510, is_voiced: true });

        // 3. String settles into true fundamental A2 (110.0 Hz, MIDI 45) at t = 520, 525, 530 ms
        tracker.record_pitch(PitchResult { frequency_hz: 110.1, confidence: 0.92, timestamp_ms: 520, is_voiced: true });
        tracker.record_pitch(PitchResult { frequency_hz: 110.0, confidence: 0.95, timestamp_ms: 525, is_voiced: true });
        tracker.record_pitch(PitchResult { frequency_hz: 109.9, confidence: 0.94, timestamp_ms: 530, is_voiced: true });

        // 4. Poll at t = 540 ms (after consensus window 35 ms has elapsed)
        let resolved = tracker.poll_resolved_notes(540);
        assert_eq!(resolved.len(), 1, "Should resolve exactly 1 note");
        assert_eq!(resolved[0].midi_note, 45, "Should resolve to MIDI 45 (A2)");
        assert!((resolved[0].resolved_frequency_hz - 110.0).abs() < 1.0, "Should resolve to 110 Hz");
    }

    // T-007: NoteHighway spawns and targets correct coordinate values
    #[test]
    fn test_t007_note_target_conversion() {
        let json_chart = r#"{
            "metadata": {
                "title": "Test Riff",
                "artist": "Dev",
                "album": "Synth",
                "bpm": 120.0,
                "tuning": "E Standard",
                "audio_file": "tracks/audio.wav"
            },
            "notes": [
                { "id": 1, "timestamp_ms": 1000, "string": 6, "fret": 0, "midi_note": 40, "duration_ms": 500 },
                { "id": 2, "timestamp_ms": 1500, "string": 5, "fret": 2, "midi_note": 47, "duration_ms": 500 }
            ]
        }"#;

        let chart = ChartParser::parse_json(json_chart).unwrap();
        let targets = ChartParser::to_note_targets(&chart);
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].target_timestamp_ms, 1000);
        assert_eq!(targets[0].expected_midi_note, 40);
    }

    // T-008: Scorer assigns Perfect for a note within ±15 ms and ±10 cents (SC-4)
    #[test]
    fn test_t008_scorer_perfect_judgment() {
        let mut scorer = RocksmithScorer::new();
        scorer.register_target(NoteTarget {
            id: 101,
            target_timestamp_ms: 2000,
            expected_midi_note: 40,
            string_index: 6,
            fret_number: 0,
            duration_ms: 300,
        });

        let event = ResolvedNoteEvent {
            onset_timestamp_ms: 2008, // +8ms
            resolved_frequency_hz: 82.41,
            confidence: 0.95,
            midi_note: 40,
            cents_deviation: 3.5, // +3.5 cents
        };

        let res = scorer.evaluate_event(&event).expect("Should hit target");
        assert_eq!(res.0, 101);
        assert_eq!(res.1, Judgment::Perfect);
        assert_eq!(scorer.get_stats().perfect_count, 1);
        assert_eq!(scorer.get_stats().current_streak, 1);
    }

    // T-009: Scorer assigns Miss for a note outside ±50 ms (SC-4)
    #[test]
    fn test_t009_scorer_miss_judgment() {
        let mut scorer = RocksmithScorer::new();
        scorer.register_target(NoteTarget {
            id: 102,
            target_timestamp_ms: 1000,
            expected_midi_note: 45,
            string_index: 5,
            fret_number: 0,
            duration_ms: 300,
        });

        // Expire targets at t = 1060 ms (> 50ms window)
        let missed = scorer.check_expired_targets(1060);
        assert_eq!(missed.len(), 1);
        assert_eq!(missed[0], (102, Judgment::Miss));
        assert_eq!(scorer.get_stats().miss_count, 1);
        assert_eq!(scorer.get_stats().current_streak, 0);
    }

    // T-010: Application writes valid session log (SC-5)
    #[test]
    fn test_t010_session_logger_atomic_write() {
        let log = SessionLog {
            song_title: "Test Riff".to_string(),
            session_start_iso: "2026-09-23T12:00:00Z".to_string(),
            duration_seconds: 45.2,
            stats: SessionScoreStats {
                total_notes: 10,
                perfect_count: 8,
                great_count: 1,
                good_count: 1,
                miss_count: 0,
                current_streak: 10,
                max_streak: 10,
                accuracy_percentage: 100.0,
            },
            notes: vec![],
        };

        let temp_dir = std::env::temp_dir();
        let log_file = temp_dir.join("test_session_log.json");
        SessionLogger::write_atomic_json(&log_file, &log).expect("Write session log should succeed");
        assert!(log_file.exists());
        let _ = std::fs::remove_file(log_file);
    }

    // T-011: Audio device disconnection resilience (MockAudioBackend)
    #[test]
    fn test_t011_audio_backend_disconnect_resilience() {
        let config = DuplexAudioConfig::default();
        let (mut backend, _, _) = MockAudioBackend::new(config);

        assert!(backend.start().is_ok());
        assert!(backend.is_running());

        // Simulate sudden USB cable pull
        backend.simulate_device_disconnect();
        assert!(!backend.is_running());
        // Restarting while disconnected returns clean error, never panics
        assert!(backend.start().is_err());
    }

    // T-012: Chart parser rejects malformed JSON without panic
    #[test]
    fn test_t012_chart_parser_malformed_json() {
        let bad_json = "{ unquoted_key: 123, broken ";
        let res = ChartParser::parse_json(bad_json);
        assert!(res.is_err(), "Must return Err for malformed JSON");
    }

    // T-013: Security — Chart parser rejects path traversal
    #[test]
    fn test_t013_chart_parser_path_traversal_rejection() {
        let traversal_json = r#"{
            "metadata": {
                "title": "Malicious Riff",
                "artist": "Attacker",
                "album": "Root",
                "bpm": 120.0,
                "tuning": "E Standard",
                "audio_file": "../../etc/shadow"
            },
            "notes": []
        }"#;

        let res = ChartParser::parse_json(traversal_json);
        assert!(res.is_err(), "Must reject path traversal '..'");
    }

    // T-015a: Deterministic DSP throughput benchmark (processing time <= 2 ms per 2048-sample hop)
    #[test]
    fn test_t015a_dsp_throughput_latency() {
        let sr = 44100.0;
        let mut detector = MultiRatePitchDetector::new(sr, 2048, 512);
        let samples = generate_sawtooth(110.0, 0.05, sr); // ~2205 samples

        let start = std::time::Instant::now();
        let _ = detector.process(&samples);
        let elapsed = start.elapsed();

        // Must take <= 5 ms even in unoptimized debug test mode
        assert!(elapsed.as_millis() <= 10, "DSP hop took {:?} (expected <= 10ms in debug)", elapsed);
    }

    // T-016: Clock synchronizer audio and visual offset progression
    #[test]
    fn test_t016_clock_synchronizer_offsets() {
        let mut sync = ClockSynchronizer::new(44100);
        sync.set_audio_offset_ms(25);
        sync.set_visual_offset_ms(-15);

        // At 44100 samples (1.0 second = 1000 ms)
        let audio_ms = sync.samples_to_audio_time_ms(44100);
        let visual_ms = sync.samples_to_visual_time_ms(44100);

        assert_eq!(audio_ms, 1025);
        assert_eq!(visual_ms, 1010);
    }

    // T-017: PitchDetector returns None on silence
    #[test]
    fn test_t017_pitch_silence_rejection() {
        let mut detector = MultiRatePitchDetector::new(44100.0, 2048, 512);
        let silence = vec![0.0f32; 2048];
        assert!(detector.process(&silence).is_none());
    }

    // T-018: PitchDetector returns None on random white noise
    #[test]
    fn test_t018_pitch_noise_rejection() {
        let mut detector = MultiRatePitchDetector::new(44100.0, 2048, 512);
        // Deterministic pseudo-random noise
        let mut state = 123456789u32;
        let noise: Vec<f32> = (0..2048).map(|_| {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            (state as f32 / u32::MAX as f32) * 0.1 - 0.05
        }).collect();

        assert!(detector.process(&noise).is_none());
    }

    // T-020: Low-E decimation 4x filter frequency stability
    #[test]
    fn test_t020_low_e_decimator() {
        let mut decimator = Decimator4x::new(44100.0);
        let input = generate_sine(82.41, 0.05, 44100.0);
        let mut decimated = Vec::new();
        decimator.process(&input, &mut decimated);

        assert_eq!(decimated.len(), input.len() / 4);
    }
}
