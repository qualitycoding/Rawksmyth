use gtcore::dsp::onset::*;
use gtcore::dsp::*;
use gtfixtures::*;

fn run(cfg: &OnsetConfig, seeds: std::ops::Range<u64>) -> (usize, usize, usize, f64, f64, f64) {
    let (mut total, mut missed, mut extra) = (0, 0, 0);
    let mut abs = Vec::new();
    for seed in seeds {
        let (sig, events) = fifty_pluck_set(seed);
        let mut det = SpectralFluxOnset::new(cfg.clone());
        let mut onsets = Vec::new();
        for (i, blk) in sig.chunks(256).enumerate() { det.process(blk, (i * 256) as u64, &mut onsets); }
        let mut matched = 0;
        for ev in &events {
            total += 1;
            let m = onsets.iter().filter(|o| (o.frame as i64 - ev.onset_frame as i64).abs() < 2000)
                .min_by_key(|o| (o.frame as i64 - ev.onset_frame as i64).abs());
            match m { Some(o) => { matched += 1; abs.push(((o.frame as i64 - ev.onset_frame as i64) as f64 / 44.1).abs()) }, None => missed += 1 }
        }
        extra += onsets.len().saturating_sub(matched);
    }
    abs.sort_by(|x, y| x.partial_cmp(y).unwrap());
    (total, missed, extra, abs[abs.len()/2], abs[(abs.len() as f64*0.95) as usize - 1], *abs.last().unwrap())
}

fn main() {
    let cfg = OnsetConfig::default();
    for (label, seeds) in [("seeds 1..11 (thresholds were tuned here)", 1..11u64), ("seeds 100..140 (held out)", 100..140u64)] {
        let (t, m, x, med, p95, mx) = run(&cfg, seeds);
        println!("{label}: total {t} missed {m} spurious {x} |err| ms median {med:.2} p95 {p95:.2} max {mx:.2}");
    }
}
