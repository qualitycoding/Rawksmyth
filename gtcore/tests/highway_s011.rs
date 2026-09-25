//! T-007 (plan §2B.3): the highway layout returns the correct lane and depth
//! for a given chart and song time. CI tier; pure layout, no engine.

use gtcore::chart::ChartNote;
use gtcore::highway::*;

fn note(id: u32, start_s: f64, sustain_s: f64, string: u8, fret: u8) -> ChartNote {
    ChartNote { id, start_s, sustain_s, string, fret, target_hz: 100.0, legato: false }
}

fn chart() -> Vec<ChartNote> {
    vec![
        note(1, 2.0, 0.0, 0, 0),
        note(2, 3.0, 1.0, 3, 5),
        note(3, 4.5, 0.0, 5, 12),
        note(4, 10.0, 0.0, 1, 2),
    ]
}

fn poses(h: &ChartHighway, t: f64) -> Vec<NotePose> {
    let mut v = Vec::new();
    h.visible_notes(t, &mut v);
    v
}

fn ids(v: &[NotePose]) -> Vec<u32> {
    v.iter().map(|p| p.note_id).collect()
}

#[test]
fn t007_lane_and_depth_for_a_given_song_time() {
    let h = ChartHighway::new(&chart()); // lookahead 3 s
    let v = poses(&h, 1.5);
    // Notes 1 (0.5 s ahead), 2 (1.5 s), 3 (3.0 s — exactly at the far end); note 4 is not yet visible.
    assert_eq!(ids(&v), vec![1, 2, 3]);
    let (a, b, c) = (v[0], v[1], v[2]);
    assert_eq!((a.lane, a.fret), (0, 0));
    assert_eq!((b.lane, b.fret), (3, 5));
    assert_eq!((c.lane, c.fret), (5, 12));
    assert!((a.depth - 0.5 / 3.0).abs() < 1e-6);
    assert!((b.depth - 0.5).abs() < 1e-6);
    assert!((c.depth - 1.0).abs() < 1e-6);
    assert!(a.state == NoteState::Approaching);
    assert!((b.sustain_depth - 1.0 / 3.0).abs() < 1e-6);
    assert_eq!(a.sustain_depth, 0.0);
}

#[test]
fn a_note_is_at_the_strike_line_at_its_start_time() {
    let h = ChartHighway::new(&chart());
    let v = poses(&h, 2.0);
    let n1 = v.iter().find(|p| p.note_id == 1).unwrap();
    assert_eq!(n1.depth, 0.0);
    assert_eq!(n1.state, NoteState::Passing);
    // Depth is linear in time: 10 ms later it has moved 10 ms/3 s past.
    let v = poses(&h, 2.010);
    let n1 = v.iter().find(|p| p.note_id == 1).unwrap();
    assert!((n1.depth + 0.010 / 3.0).abs() < 1e-6);
}

#[test]
fn passed_notes_disappear_after_the_trail_but_sustains_stay() {
    let h = ChartHighway::new(&chart()); // trail 0.25 s
    // Note 1 (plain, start 2.0): visible through 2.25, gone after.
    assert!(ids(&poses(&h, 2.25)).contains(&1));
    assert!(!ids(&poses(&h, 2.26)).contains(&1));
    // Note 2 (start 3.0, sustain 1.0): visible while sustaining …
    let v = poses(&h, 3.8);
    let n2 = v.iter().find(|p| p.note_id == 2).expect("still sustaining");
    assert_eq!(n2.state, NoteState::Passing);
    assert!(n2.depth < 0.0 && n2.sustain_depth > 0.0);
    // … and gone once the sustain ends.
    assert!(!ids(&poses(&h, 4.01)).contains(&2));
}

#[test]
fn lead_in_negative_time_and_far_future() {
    let h = ChartHighway::new(&chart());
    assert!(poses(&h, -5.0).is_empty(), "nothing within 3 s of t = −5");
    assert_eq!(ids(&poses(&h, -1.0)), vec![1]);
    assert_eq!(ids(&poses(&h, 9.0)), vec![4]);
    assert!(poses(&h, 100.0).is_empty());
}

#[test]
fn output_is_ordered_by_start_and_out_is_reused() {
    // Chart supplied out of order.
    let mut c = chart();
    c.reverse();
    let h = ChartHighway::new(&c);
    let mut out = vec![NotePose { note_id: 999, lane: 0, fret: 0, depth: 0.0, sustain_depth: 0.0, state: NoteState::Approaching }];
    h.visible_notes(2.2, &mut out);
    assert_eq!(ids(&out), vec![1, 2, 3], "stale content cleared, start order restored");
    let cap = out.capacity();
    h.visible_notes(2.21, &mut out);
    assert_eq!(out.capacity(), cap, "no reallocation for a same-sized frame");
}

#[test]
fn degenerate_inputs_do_not_panic() {
    let h = ChartHighway::new(&[]);
    assert!(poses(&h, 1.0).is_empty());
    let h = ChartHighway::new(&chart());
    for t in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(poses(&h, t).is_empty());
    }
    // Negative sustain is treated as zero.
    let h = ChartHighway::new(&[note(1, 1.0, -3.0, 0, 0)]);
    assert_eq!(poses(&h, 1.0)[0].sustain_depth, 0.0);
}

/// A custom lookahead scales depth: with 1 s, a note 0.5 s ahead is at 0.5
/// and a note 1.5 s ahead is not shown.
#[test]
fn lookahead_scales_depth() {
    let h = ChartHighway::with_config(&chart(), HighwayConfig { lookahead_s: 1.0, trail_s: 0.25 });
    let v = poses(&h, 1.5);
    assert_eq!(ids(&v), vec![1]);
    assert!((v[0].depth - 0.5).abs() < 1e-6);
}
