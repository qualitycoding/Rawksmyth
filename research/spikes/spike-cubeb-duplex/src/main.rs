//! Spike 1 (plan §1.3 R6): cubeb duplex behaviour and round-trip latency.
//!
//!   spike-cubeb-duplex list
//!   spike-cubeb-duplex duplex [--rate 44100] [--latency 256] [--in N] [--out N]
//!                             [--gain 0.3] [--clicks 12] [--silent] [--json FILE]
//!
//! Questions it answers (plan claims C-001, C-011, C-014, risk R-104):
//!  * Does a cubeb duplex stream deliver equal input/output frame counts per
//!    callback, so one running frame index serves both (C-011)?
//!  * What callback sizes and timing jitter does the driver produce?
//!  * How large is the *measured* round-trip latency (a click emitted at
//!    output frame E arrives at input frame E + L_rt) compared with what the
//!    driver *reports* (L_in + L_out)? The difference is the R-104 error that
//!    the offset assist (D-013) exists to absorb.
//!
//! The callback is the real `gtaudio::rt::RtCore`, so this also exercises the
//! production callback under a real device. Loopback here is whatever the
//! machine offers: with the plan's reference interface it is a loopback
//! cable; on a laptop it is acoustic (speaker → microphone), which adds a
//! little acoustic delay and needs audible volume.
//!
//! The input is captured as *stereo* and channel 0 is used, because
//! cubeb-rs's duplex callback has one frame type for both directions.

use cubeb::{
    ChannelLayout, Context, DeviceType, SampleFormat, State, StereoFrame, StreamBuilder, StreamParamsBuilder,
    StreamPrefs,
};
use gtaudio::ring::ring;
use gtaudio::rt::{RtConfig, RtCore};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

type Frame = StereoFrame<f32>;
const CHUNK: usize = 8192;

/// FINDING: cubeb's WASAPI backend aborts the process ("hr !=
/// CO_E_NOTINITIALIZED") unless COM is initialized on the thread that creates
/// the context; cubeb-rs does not do this. The real backend must call it (and
/// on every thread that touches cubeb).
#[cfg(windows)]
fn init_com() {
    #[link(name = "ole32")]
    extern "system" {
        fn CoInitializeEx(reserved: *mut core::ffi::c_void, coinit: u32) -> i32;
    }
    // COINIT_MULTITHREADED = 0.
    let hr = unsafe { CoInitializeEx(std::ptr::null_mut(), 0) };
    if hr < 0 {
        eprintln!("CoInitializeEx failed: 0x{hr:08x}");
    }
}

#[cfg(not(windows))]
fn init_com() {}

fn main() {
    init_com();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("list") => list(),
        Some("duplex") => duplex(&args[1..]),
        _ => {
            eprintln!("usage: spike-cubeb-duplex list | duplex [options]  (see source header)");
            std::process::exit(1);
        }
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn stereo_params(rate: u32) -> cubeb::StreamParams {
    StreamParamsBuilder::new()
        .format(SampleFormat::Float32NE)
        .rate(rate)
        .channels(2)
        .layout(ChannelLayout::STEREO)
        .prefs(StreamPrefs::NONE)
        .take()
}

fn list() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = Context::init(Some(c"gt spike"), None)?;
    let params = stereo_params(44100);
    println!("backend: {}", ctx.backend_id());
    println!("preferred sample rate: {:?}", ctx.preferred_sample_rate());
    println!("max channel count: {:?}", ctx.max_channel_count());
    println!("min latency @44.1k stereo f32: {:?} frames", ctx.min_latency(&params));
    for (label, ty) in [("INPUT", DeviceType::INPUT), ("OUTPUT", DeviceType::OUTPUT)] {
        let coll = ctx.enumerate_devices(ty)?;
        println!("\n{label} devices:");
        for (i, d) in coll.iter().enumerate() {
            println!(
                "  [{i}] {:?}  state={:?} ch={} rate={}..{} (default {}) latency {}..{} preferred={:?}",
                d.friendly_name().unwrap_or("?"),
                d.state(),
                d.max_channels(),
                d.min_rate(),
                d.max_rate(),
                d.default_rate(),
                d.latency_lo(),
                d.latency_hi(),
                d.preferred(),
            );
        }
    }
    Ok(())
}

/// 60 ms logarithmic chirp, 300 Hz → 8 kHz, Hann-tapered, peak 1. A chirp
/// (rather than a short click) survives poor speakers and microphones: its
/// matched-filter output has high processing gain and a sharp peak.
fn click_template(rate: u32) -> Vec<f32> {
    let n = (0.060 * rate as f64) as usize;
    let (f0, f1) = (300.0f64, 8000.0f64);
    let t_total = n as f64 / rate as f64;
    let k = (f1 / f0).ln() / t_total;
    (0..n)
        .map(|i| {
            let t = i as f64 / rate as f64;
            let phase = 2.0 * std::f64::consts::PI * f0 * ((k * t).exp() - 1.0) / k;
            let w = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (n - 1) as f64).cos();
            (phase.sin() * w) as f32
        })
        .collect()
}

#[derive(Default)]
struct Stats {
    in_lens: Vec<u32>,
    out_lens: Vec<u32>,
    t_ns: Vec<u64>,
}

fn drain_stats(rx: &mut gtaudio::ring::Consumer, st: &mut Stats) {
    let mut w = [0u32; 4096];
    loop {
        let k = rx.pop_words(&mut w);
        for c in w[..k - k % 4].chunks_exact(4) {
            st.in_lens.push(c[0]);
            st.out_lens.push(c[1]);
            st.t_ns.push(c[2] as u64 | (c[3] as u64) << 32);
        }
        if k < w.len() {
            break;
        }
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

fn duplex(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let rate: u32 = flag(args, "--rate").map(|s| s.parse()).transpose()?.unwrap_or(44100);
    let latency: u32 = flag(args, "--latency").map(|s| s.parse()).transpose()?.unwrap_or(256);
    let gain: f32 = flag(args, "--gain").map(|s| s.parse()).transpose()?.unwrap_or(0.3);
    let clicks: usize = flag(args, "--clicks").map(|s| s.parse()).transpose()?.unwrap_or(12);
    let in_idx: Option<usize> = flag(args, "--in").map(|s| s.parse()).transpose()?;
    let out_idx: Option<usize> = flag(args, "--out").map(|s| s.parse()).transpose()?;
    let silent = args.iter().any(|a| a == "--silent");
    let only_out = args.iter().any(|a| a == "--only-out");
    let only_in = args.iter().any(|a| a == "--only-in");
    let json_path = flag(args, "--json");
    let fs = rate as f64;

    // ---- click train as the "backing track" (output frames == frame index, s0 = 0)
    let template = click_template(rate);
    let lead = rate as usize; // first click at 1.0 s
    let spacing = (0.75 * fs) as usize;
    let total = lead + clicks * spacing + rate as usize;
    let mut backing = vec![0.0f32; total * 2];
    let expected: Vec<usize> = (0..clicks).map(|k| lead + k * spacing).collect();
    if !silent {
        for &e in &expected {
            for (j, t) in template.iter().enumerate() {
                backing[(e + j) * 2] = gain * t;
                backing[(e + j) * 2 + 1] = gain * t;
            }
        }
    }
    let (mut core, mut handles) =
        RtCore::new(RtConfig { channels: 2, s0: 0, input_ring_samples: 1 << 18 }, backing, Vec::new());
    let (mut stats_tx, mut stats_rx) = ring(1 << 16);

    let ctx = Context::init(Some(c"gt spike"), None)?;
    let params = stereo_params(rate);
    let in_coll = ctx.enumerate_devices(DeviceType::INPUT)?;
    let out_coll = ctx.enumerate_devices(DeviceType::OUTPUT)?;
    let mut in_name = "default input".to_string();
    let mut out_name = "default output".to_string();

    let states = Arc::new(Mutex::new(Vec::<String>::new()));
    let states_cb = states.clone();
    let device_changed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let device_changed_cb = device_changed.clone();
    let start = Instant::now();
    let mut in_mono = vec![0.0f32; CHUNK];
    let mut out_scratch = vec![0.0f32; CHUNK * 2];

    let mut builder = StreamBuilder::<Frame>::new();
    builder
        .name("gt spike duplex")
        .latency(latency)
        .data_callback(move |input: &[Frame], output: &mut [Frame]| {
            let t = start.elapsed().as_nanos() as u64;
            let n = output.len();
            stats_tx.push_words(&[input.len() as u32, n as u32, t as u32, (t >> 32) as u32]);
            let mut done = 0;
            while done < n {
                let m = (n - done).min(CHUNK);
                for i in 0..m {
                    in_mono[i] = input.get(done + i).map_or(0.0, |f| f.l);
                }
                core.callback(&in_mono[..m], &mut out_scratch[..2 * m], t as i64);
                for i in 0..m {
                    output[done + i] = Frame { l: out_scratch[2 * i], r: out_scratch[2 * i + 1] };
                }
                done += m;
            }
            n as isize
        })
        .state_callback(move |s: State| states_cb.lock().unwrap().push(format!("{s:?}")));
    // Registering a device-changed callback is opt-in: see the FINDING in the
    // header comment of the results file — on some backends it makes stream
    // creation fail with NotSupported.
    if args.iter().any(|a| a == "--device-changed-cb") {
        builder.device_changed_cb(move || {
            device_changed_cb.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });
    }
    if !only_out { match in_idx {
        Some(i) => {
            let d = in_coll.iter().nth(i).ok_or("no such input device index")?;
            in_name = d.friendly_name().unwrap_or("?").to_string();
            builder.input(d.devid(), &params);
        }
        None => {
            builder.default_input(&params);
        }
    }
    } if !only_in { match out_idx {
        Some(i) => {
            let d = out_coll.iter().nth(i).ok_or("no such output device index")?;
            out_name = d.friendly_name().unwrap_or("?").to_string();
            builder.output(d.devid(), &params);
        }
        None => {
            builder.default_output(&params);
        }
    } }
    let stream = builder.init(&ctx)?;
    stream.start()?;
    std::thread::sleep(Duration::from_millis(300));
    let out_latency = stream.latency();
    let in_latency = stream.input_latency();

    let run_for = Duration::from_secs_f64(total as f64 / fs + 0.5);
    let mut recorded: Vec<f32> = Vec::with_capacity((run_for.as_secs_f64() * fs) as usize + 1_000_000);
    let mut stats = Stats::default();
    let mut buf = vec![0.0f32; 16384];
    let began = Instant::now();
    while began.elapsed() < run_for {
        std::thread::sleep(Duration::from_millis(20));
        loop {
            let k = handles.input.pop_samples(&mut buf);
            recorded.extend_from_slice(&buf[..k]);
            if k < buf.len() {
                break;
            }
        }
        drain_stats(&mut stats_rx, &mut stats);
    }
    stream.stop()?;
    std::thread::sleep(Duration::from_millis(100));
    loop {
        let k = handles.input.pop_samples(&mut buf);
        recorded.extend_from_slice(&buf[..k]);
        if k < buf.len() {
            break;
        }
    }
    drain_stats(&mut stats_rx, &mut stats);
    let published = handles.clock.read();
    drop(stream);

    // ---- C-011 and callback statistics
    let calls = stats.in_lens.len();
    let mismatches = stats.in_lens.iter().zip(&stats.out_lens).filter(|(a, b)| a != b).count();
    let zero_input = stats.in_lens.iter().filter(|&&n| n == 0).count();
    let mut sizes = std::collections::BTreeMap::new();
    for &n in &stats.out_lens {
        *sizes.entry(n).or_insert(0usize) += 1;
    }
    let mut dts: Vec<f64> = stats.t_ns.windows(2).map(|w| (w[1] - w[0]) as f64 / 1e6).collect();
    dts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean_dt = dts.iter().sum::<f64>() / dts.len().max(1) as f64;
    let sd_dt = (dts.iter().map(|d| (d - mean_dt).powi(2)).sum::<f64>() / dts.len().max(1) as f64).sqrt();

    // ---- round-trip latency by normalized cross-correlation
    let mut sq = vec![0.0f64; recorded.len() + 1];
    for (i, x) in recorded.iter().enumerate() {
        sq[i + 1] = sq[i] + (*x as f64) * (*x as f64);
    }
    let tmpl_energy = template.iter().map(|t| (*t as f64).powi(2)).sum::<f64>().sqrt();
    let unit: Vec<f32> = template.iter().map(|t| (*t as f64 / tmpl_energy) as f32).collect();
    let rec_rms = (sq[recorded.len()] / recorded.len().max(1) as f64).sqrt();
    println!("recorded input: {} frames, overall RMS {:.5} ({:.1} dBFS)", recorded.len(), rec_rms, 20.0 * (rec_rms + 1e-12).log10());
    // (frames, ncc, peak-to-sidelobe ratio)
    let mut round_trips: Vec<Option<(f64, f64, f64)>> = Vec::new();
    for &e in &expected {
        let lo = e.saturating_sub((0.01 * fs) as usize);
        let hi = (e + (0.5 * fs) as usize).min(recorded.len().saturating_sub(unit.len()));
        if hi <= lo {
            round_trips.push(None);
            continue;
        }
        let ncc: Vec<f64> = (lo..hi)
            .map(|i| {
                let c: f64 = unit.iter().enumerate().map(|(j, t)| recorded[i + j] as f64 * *t as f64).sum();
                c / ((sq[i + unit.len()] - sq[i]).sqrt() + 1e-12)
            })
            .collect();
        let (bi, best) = ncc.iter().enumerate().fold((0, f64::MIN), |a, (i, &v)| if v > a.1 { (i, v) } else { a });
        // Sidelobe: the best correlation more than one chirp length away from the peak.
        let guard = unit.len();
        let side = ncc
            .iter()
            .enumerate()
            .filter(|(i, _)| i.abs_diff(bi) > guard)
            .fold(0.0f64, |m, (_, &v)| m.max(v));
        let psr = best / side.max(1e-9);
        // Reject matches on a signal too quiet to be real acoustic/electrical
        // capture (normalized correlation is scale-invariant and will happily
        // "find" the chirp in numerical residue at -150 dBFS).
        let at = lo + bi;
        let level = ((sq[at + unit.len()] - sq[at]) / unit.len() as f64).sqrt();
        round_trips.push(if best >= 0.15 && psr >= 2.0 && level >= 1e-4 { Some((((at) as f64) - e as f64, best, psr)) } else { None });
    }
    let found: Vec<f64> = round_trips.iter().flatten().map(|(f, _, _)| *f).collect();
    let mut sorted = found.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_frames = percentile(&sorted, 0.5);

    // ---- report
    let l_out = out_latency.as_ref().ok().copied();
    let l_in = in_latency.as_ref().ok().copied();
    let reported = match (l_out, l_in) {
        (Some(o), Some(i)) => Some((o + i) as f64),
        (Some(o), None) => Some(o as f64),
        _ => None,
    };
    let ms = |f: f64| f / fs * 1e3;
    println!("== spike 1: cubeb duplex ==");
    println!("backend: {}   input: {in_name}   output: {out_name}", ctx.backend_id());
    println!("requested: {rate} Hz, stereo f32, latency {latency} frames");
    println!("stream.latency() (output): {out_latency:?}   stream.input_latency(): {in_latency:?}");
    println!("stream states: {:?}   device-changed callbacks: {}", states.lock().unwrap(), device_changed.load(std::sync::atomic::Ordering::Relaxed));
    println!("\n-- C-011 (equal frame counts) and callback behaviour --");
    println!("callbacks: {calls}   input len != output len in {mismatches} of them   zero-length input in {zero_input}");
    println!("callback sizes (frames → count): {sizes:?}");
    println!(
        "interval ms: mean {mean_dt:.3} sd {sd_dt:.3} p50 {:.3} p99 {:.3} max {:.3}",
        percentile(&dts, 0.5),
        percentile(&dts, 0.99),
        dts.last().copied().unwrap_or(f64::NAN)
    );
    println!("RtCore frames processed: {}   input-ring overruns: {}   last clock sample: {published:?}", core_frames(&published), handles_overruns(&handles));
    println!("\n-- round-trip latency ({} of {} clicks detected) --", found.len(), clicks);
    for (k, r) in round_trips.iter().enumerate() {
        match r {
            Some((f, ncc, psr)) => println!("click {k:2}: {:8.1} frames = {:7.2} ms   (ncc {ncc:.2}, peak/sidelobe {psr:.1})", f, ms(*f)),
            None => println!("click {k:2}: not detected"),
        }
    }
    if !found.is_empty() {
        println!(
            "measured L_rt: median {:.1} frames = {:.2} ms   min {:.2} ms  max {:.2} ms  spread {:.2} ms",
            median_frames,
            ms(median_frames),
            ms(sorted[0]),
            ms(*sorted.last().unwrap()),
            ms(sorted.last().unwrap() - sorted[0])
        );
        match reported {
            Some(r) => println!(
                "driver-reported L_in+L_out: {r:.0} frames = {:.2} ms   → measured − reported = {:+.2} ms (R-104 error; includes acoustic path if not a cable)",
                ms(r),
                ms(median_frames - r)
            ),
            None => println!("driver reported no usable latency"),
        }
    } else {
        println!("no valid clicks: the input carried no usable signal (level < -80 dBFS at the expected times). The microphone is muted/blocked/echo-cancelled, the loopback path is not connected, or the output is too quiet (try --gain). No latency can be stated.");
    }

    if let Some(p) = json_path {
        let rts: Vec<String> = round_trips
            .iter()
            .map(|r| r.map_or("null".to_string(), |(f, _, _)| format!("{f:.2}")))
            .collect();
        let json = format!(
            "{{\n  \"backend\": \"{}\",\n  \"input\": \"{}\",\n  \"output\": \"{}\",\n  \"rate\": {rate},\n  \"requested_latency_frames\": {latency},\n  \
             \"output_latency_frames\": {},\n  \"input_latency_frames\": {},\n  \"callbacks\": {calls},\n  \"input_output_len_mismatches\": {mismatches},\n  \
             \"zero_length_input_callbacks\": {zero_input},\n  \"callback_interval_ms\": {{\"mean\": {mean_dt:.4}, \"sd\": {sd_dt:.4}, \"p99\": {:.4}, \"max\": {:.4}}},\n  \
             \"clicks_detected\": {},\n  \"round_trip_frames\": [{}],\n  \"median_round_trip_ms\": {}\n}}\n",
            ctx.backend_id(),
            in_name.replace('"', "'"),
            out_name.replace('"', "'"),
            l_out.map_or("null".into(), |v| v.to_string()),
            l_in.map_or("null".into(), |v| v.to_string()),
            percentile(&dts, 0.99),
            dts.last().copied().unwrap_or(f64::NAN),
            found.len(),
            rts.join(", "),
            if found.is_empty() { "null".to_string() } else { format!("{:.3}", ms(median_frames)) },
        );
        std::fs::write(&p, json)?;
        println!("\nwrote {p}");
    }
    Ok(())
}

fn core_frames(c: &Option<gtcore::clock::ClockSample>) -> u64 {
    c.map_or(0, |s| s.frames_written)
}

fn handles_overruns(h: &gtaudio::rt::RtHandles) -> u64 {
    h.input.overruns()
}
