//! gtapp: the headless application core (plan step S-013, hardware-free part).
//!
//! `Song` loads a song directory (chart + backing track + optional rendered
//! guitar reference). `Engine` wires pipeline → scorer → session recorder.
//! `run_session` drives an `Engine` and the RT core from any `AudioBackend`
//! — the deterministic file backend here, a cubeb backend on hardware — and
//! turns device loss into a saved session plus a user-facing notice (T-011).
//!
//! Not here: settings UI, the Godot scene, macOS permission plist, the live
//! three-thread wiring (those need a display and audio hardware).

use gtaudio::backend::{AudioBackend, BackendError};
use gtaudio::rt::{RtConfig, RtCore};
use gtaudio::wav;
use gtcore::chart::{parse_chart_bytes, ChartFile, ChartNote};
use gtcore::clock::{ClockModel, Frame};
use gtcore::dsp::EvidenceKind;
use gtcore::pipeline::Pipeline;
use gtcore::scoring::{JudgedNote, Scorer, WindowScorer};
use gtcore::session::{ClockSettings, SessionLog, SessionRecorder, SessionStatus};
use std::fmt;
use std::path::{Path, PathBuf};

pub const SAMPLE_RATE: f64 = 44100.0;
/// Silence before the backing track starts, output frames (§2B.1d: ≥ 1 s).
pub const LEAD_IN_S: f64 = 1.0;
/// Input buffer request (plan §0.3.1).
pub const BLOCK_FRAMES: usize = 256;
/// Stream time kept running after the input ends so the last notes'
/// evidence can complete.
pub const TAIL_S: f64 = 2.0;

#[derive(Debug)]
pub enum SongError {
    Io(String),
    Chart(String),
    Wav(String),
    SampleRate { file: String, found: u32 },
}

impl fmt::Display for SongError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SongError::Io(s) => write!(f, "could not read song file: {s}"),
            SongError::Chart(s) => write!(f, "invalid chart: {s}"),
            SongError::Wav(s) => write!(f, "invalid audio file: {s}"),
            SongError::SampleRate { file, found } => {
                write!(f, "{file} is {found} Hz; only {} Hz is supported", SAMPLE_RATE as u32)
            }
        }
    }
}

impl std::error::Error for SongError {}

pub struct Song {
    pub dir: PathBuf,
    pub chart: ChartFile,
    /// Backing track, mono.
    pub backing: Vec<f32>,
    /// Rendered guitar part (as heard, song-time aligned), mono.
    pub guitar_reference: Option<Vec<f32>>,
}

fn load_wav(dir: &Path, name: &str) -> Result<Vec<f32>, SongError> {
    let path = dir.join(name);
    let w = wav::read_file(&path).map_err(|e| SongError::Wav(format!("{name}: {e}")))?;
    if w.sample_rate as f64 != SAMPLE_RATE {
        return Err(SongError::SampleRate { file: name.to_string(), found: w.sample_rate });
    }
    Ok(w.to_mono())
}

impl Song {
    /// Load `<dir>/chart.json` and the assets it names. Asset names were
    /// already validated by the chart parser (no traversal, T-013).
    pub fn load(dir: &Path) -> Result<Song, SongError> {
        let bytes = std::fs::read(dir.join("chart.json")).map_err(|e| SongError::Io(format!("chart.json: {e}")))?;
        let chart = parse_chart_bytes(&bytes).map_err(|e| SongError::Chart(e.to_string()))?;
        let backing = match &chart.assets.backing_track {
            Some(name) => load_wav(dir, name)?,
            None => Vec::new(),
        };
        let guitar_reference = match &chart.assets.guitar_reference {
            Some(name) => Some(load_wav(dir, name)?),
            None => None,
        };
        Ok(Song { dir: dir.to_path_buf(), chart, backing, guitar_reference })
    }

    pub fn notes(&self) -> &[ChartNote] {
        &self.chart.notes
    }
}

/// User-adjustable timing settings (A-008) and the device's reported latencies.
#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub l_in_s: f64,
    pub l_out_s: f64,
    /// Signed audio offset δ_a.
    pub delta_a_s: f64,
    /// Signed video offset δ_v (only the renderer uses it; logged).
    pub delta_v_s: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { l_in_s: 0.012, l_out_s: 0.018, delta_a_s: 0.0, delta_v_s: 0.0 }
    }
}

impl Settings {
    pub fn clock(&self) -> ClockModel {
        ClockModel {
            fs: SAMPLE_RATE,
            s0: (LEAD_IN_S * SAMPLE_RATE) as Frame,
            l_in: self.l_in_s,
            l_out: self.l_out_s,
            delta_a: self.delta_a_s,
        }
    }

    fn clock_settings(&self) -> ClockSettings {
        ClockSettings {
            sample_rate: SAMPLE_RATE,
            l_in_s: self.l_in_s,
            l_out_s: self.l_out_s,
            delta_a_s: self.delta_a_s,
            delta_v_s: self.delta_v_s,
        }
    }
}

/// Frame of the input stream at which a player who plays exactly with the
/// music (as heard through the output latency) produces the sound of a note
/// charted at `song_time_s`: the inverse of `ClockModel::song_time`.
pub fn input_frame_for(clock: &ClockModel, song_time_s: f64) -> usize {
    clock.frame_for_song_time(song_time_s) as usize
}

/// A perfect playthrough as a device would capture it: the rendered guitar
/// part shifted to its place in the input stream.
pub fn simulated_input(guitar_reference: &[f32], clock: &ClockModel) -> Vec<f32> {
    let offset = input_frame_for(clock, 0.0);
    let mut v = vec![0.0f32; offset];
    v.extend_from_slice(guitar_reference);
    v
}

/// Pipeline + scorer + recorder for one session.
pub struct Engine {
    pipeline: Pipeline,
    scorer: WindowScorer,
    recorder: SessionRecorder,
    chart: Vec<ChartNote>,
    clock: ClockModel,
    evidence: Vec<gtcore::dsp::NoteEvidence>,
    fresh: Vec<JudgedNote>,
}

impl Engine {
    pub fn new(title: &str, started_at: &str, chart: &[ChartNote], settings: &Settings) -> Engine {
        let clock = settings.clock();
        let ids: Vec<u32> = chart.iter().map(|n| n.id).collect();
        Engine {
            pipeline: Pipeline::new(chart, &clock),
            scorer: WindowScorer::new(chart),
            recorder: SessionRecorder::new(title, started_at, settings.clock_settings(), &ids),
            chart: chart.to_vec(),
            clock,
            evidence: Vec::new(),
            fresh: Vec::new(),
        }
    }

    /// Feed the next contiguous input block; returns the judgments issued by
    /// this block (for the UI's event queue).
    pub fn feed(&mut self, block: &[f32], first: Frame) -> &[JudgedNote] {
        self.fresh.clear();
        self.evidence.clear();
        self.pipeline.process(block, first, &mut self.evidence);
        for ev in &self.evidence {
            match ev.kind {
                EvidenceKind::Refinement => self.recorder.record_refinement(ev.note_id, ev.cents_error),
                EvidenceKind::Decision => {
                    if let Some(note) = self.chart.iter().find(|n| n.id == ev.note_id) {
                        if let Some(j) = self.scorer.on_evidence(ev, note, &self.clock) {
                            self.recorder.record_judgment(&j);
                            self.fresh.push(j);
                        }
                    }
                }
            }
        }
        let now = first + block.len() as Frame;
        for j in self.scorer.expire(now, &self.clock) {
            self.recorder.record_judgment(&j);
            self.fresh.push(j);
        }
        &self.fresh
    }

    /// Close the session. For a completed song any note still unjudged is a
    /// Miss; for an interrupted one they stay unjudged.
    pub fn finish(mut self, status: SessionStatus) -> SessionLog {
        if status == SessionStatus::Completed {
            for j in self.scorer.expire(Frame::MAX, &self.clock) {
                self.recorder.record_judgment(&j);
            }
        }
        self.recorder.finish(status)
    }
}

pub struct SessionOptions<'a> {
    pub song: &'a Song,
    pub settings: Settings,
    pub started_at: String,
    /// Where to write the session log (atomically); `None` skips writing.
    pub log_path: Option<PathBuf>,
}

pub struct SessionOutcome {
    pub log: SessionLog,
    /// Every judgment in the order issued.
    pub judged: Vec<JudgedNote>,
    /// Message the UI must show the player (device loss, log write failure).
    pub notice: Option<String>,
}

/// Run one session over `backend`. Never panics on backend failure: a lost
/// device ends the session early with the partial log saved (T-011).
pub fn run_session<B: AudioBackend>(backend: &mut B, opts: SessionOptions<'_>) -> SessionOutcome {
    let SessionOptions { song, settings, started_at, log_path } = opts;
    let clock = settings.clock();
    let channels = backend.output_channels();
    let backing: Vec<f32> = song.backing.iter().flat_map(|&s| std::iter::repeat(s).take(channels)).collect();
    let (mut core, mut handles) =
        RtCore::new(RtConfig { channels, s0: clock.s0, input_ring_samples: 1 << 16 }, backing, Vec::new());
    let mut engine = Engine::new(&song.chart.title, &started_at, song.notes(), &settings);

    let mut judged_all: Vec<JudgedNote> = Vec::new();
    let mut scratch = vec![0.0f32; BLOCK_FRAMES * 4];
    let mut fed: Frame = 0;
    let result = backend.run(&mut |inp, out, ns| {
        core.callback(inp, out, ns);
        // Deterministic single-thread stepping: the "DSP thread" drains the
        // ring right after each callback.
        loop {
            let k = handles.input.pop_samples(&mut scratch);
            if k == 0 {
                break;
            }
            let first = fed;
            let js = engine.feed(&scratch[..k], first);
            judged_all.extend_from_slice(js);
            fed += k as Frame;
            if k < scratch.len() {
                break;
            }
        }
    });

    let (status, mut notice) = match result {
        Ok(_) => (SessionStatus::Completed, None),
        Err(BackendError::DeviceLost { at_frame }) => (
            SessionStatus::DeviceLost { at_frame },
            Some(format!(
                "The audio device was lost at {:.1} s. Your progress so far has been saved; \
                 reconnect the device to continue.",
                at_frame as f64 / SAMPLE_RATE
            )),
        ),
        Err(e) => (SessionStatus::Aborted, Some(format!("Audio error: {e}. The session was stopped and saved."))),
    };
    let log = engine.finish(status);
    if let Some(path) = log_path {
        if let Err(e) = log.write_atomic(&path) {
            let msg = format!("The session log could not be saved to {}: {e}", path.display());
            notice = Some(match notice {
                Some(n) => format!("{n} {msg}"),
                None => msg,
            });
        }
    }
    SessionOutcome { log, judged: judged_all, notice }
}
