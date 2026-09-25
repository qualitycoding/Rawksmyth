//! `gt-headless <song_dir> [--log <session.json>] [--fail-at <seconds>]`
//!
//! Plays a song's rendered guitar reference through the full pipeline on the
//! deterministic file backend (no audio hardware, faster than real time) and
//! writes the session log. Exit code 0 on a completed session, 2 if the
//! (simulated) device was lost, 1 on error.

use gtaudio::backend::FileBackend;
use gtapp::*;
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(dir) = args.next() else {
        eprintln!("usage: gt-headless <song_dir> [--log <session.json>] [--fail-at <seconds>]");
        std::process::exit(1);
    };
    let (mut log_path, mut fail_at) = (None, None);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--log" => log_path = args.next().map(PathBuf::from),
            "--fail-at" => fail_at = args.next().and_then(|s| s.parse::<f64>().ok()),
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(1);
            }
        }
    }
    let song = match Song::load(&PathBuf::from(&dir)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let Some(reference) = song.guitar_reference.as_ref() else {
        eprintln!("song has no guitar_reference to play back");
        std::process::exit(1);
    };
    let settings = Settings::default();
    let clock = settings.clock();
    let input = simulated_input(reference, &clock);
    let mut backend = FileBackend::new(input, SAMPLE_RATE)
        .block_size(BLOCK_FRAMES)
        .channels(2)
        .latencies(settings.l_in_s, settings.l_out_s)
        .tail_frames((TAIL_S * SAMPLE_RATE) as u64);
    if let Some(s) = fail_at {
        backend = backend.fail_at_frame((s * SAMPLE_RATE) as u64);
    }
    let out = run_session(
        &mut backend,
        SessionOptions { song: &song, settings, started_at: "headless".into(), log_path },
    );
    let s = &out.log.summary;
    println!(
        "{}: {} notes — Perfect {} Great {} Good {} Miss {} — score {:.1}% (longest streak {})",
        out.log.song_title, s.total, s.perfect, s.great, s.good, s.miss, s.score_percent, s.longest_streak
    );
    if let Some(n) = &out.notice {
        println!("NOTICE: {n}");
    }
    std::process::exit(match out.log.status {
        gtcore::session::SessionStatus::Completed => 0,
        gtcore::session::SessionStatus::DeviceLost { .. } => 2,
        _ => 1,
    });
}
