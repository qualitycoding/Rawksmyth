import { PitchEstimate } from './mcLeodPitch';
import { OnsetDetectionEvent } from './spectralOnset';

export interface ResolvedNoteEvent {
  onsetTimestampMs: number;
  resolvedFrequencyHz: number;
  confidence: number;
  midiNote: number;
  centsDeviation: number;
  noteName: string;
}

interface CandidateNote {
  onsetTimestampMs: number;
  pitches: { timestampMs: number; freqHz: number; confidence: number }[];
  resolved: boolean;
}

/**
 * Temporal Note Tracker: Attack/Pitch Reconciler (Decision D-006)
 * Reconciles the chaotic 0-15ms pick attack transient with 15-35ms consensus pitch.
 */
export class TemporalNoteTracker {
  private transientBlankMs: number = 15;
  private consensusWindowMs: number = 35;
  private maxHistoryMs: number = 120;
  private candidates: CandidateNote[] = [];

  public recordOnset(onset: OnsetDetectionEvent): void {
    const last = this.candidates[this.candidates.length - 1];
    if (last && onset.timestampMs - last.onsetTimestampMs < 30) {
      return; // Skip duplicate bounce
    }

    this.candidates.push({
      onsetTimestampMs: onset.timestampMs,
      pitches: [],
      resolved: false
    });
  }

  public recordPitch(pitch: PitchEstimate): void {
    for (const cand of this.candidates) {
      if (cand.resolved) continue;

      const delta = pitch.timestampMs - cand.onsetTimestampMs;
      // Discard initial 0-15ms pick noise; accumulate 15-35ms stable readings
      if (delta >= this.transientBlankMs && delta <= this.maxHistoryMs) {
        cand.pitches.push({
          timestampMs: pitch.timestampMs,
          freqHz: pitch.frequencyHz,
          confidence: pitch.confidence
        });
      }
    }
  }

  public pollResolvedNotes(currentTimeMs: number): ResolvedNoteEvent[] {
    const resolvedEvents: ResolvedNoteEvent[] = [];

    for (const cand of this.candidates) {
      if (cand.resolved) continue;

      const age = currentTimeMs - cand.onsetTimestampMs;
      if (age >= this.consensusWindowMs && cand.pitches.length > 0) {
        // Sort pitches to extract median (rejecting transient harmonic overtones)
        cand.pitches.sort((a, b) => a.freqHz - b.freqHz);
        const medianIdx = Math.floor(cand.pitches.length / 2);
        const medianFreq = cand.pitches[medianIdx].freqHz;

        let totalConf = 0;
        for (const p of cand.pitches) totalConf += p.confidence;
        const avgConf = totalConf / cand.pitches.length;

        const midiFloat = 69 + 12 * Math.log2(medianFreq / 440);
        const roundedMidi = Math.round(midiFloat);
        const cents = (midiFloat - roundedMidi) * 100;

        const names = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
        const octave = Math.floor(roundedMidi / 12) - 1;
        const noteName = `${names[roundedMidi % 12]}${octave}`;

        resolvedEvents.push({
          onsetTimestampMs: cand.onsetTimestampMs,
          resolvedFrequencyHz: medianFreq,
          confidence: avgConf,
          midiNote: Math.max(0, Math.min(127, roundedMidi)),
          centsDeviation: cents,
          noteName
        });

        cand.resolved = true;
      }
    }

    // Prune resolved candidates older than 300ms
    this.candidates = this.candidates.filter(
      (c) => !c.resolved || currentTimeMs - c.onsetTimestampMs < 300
    );

    return resolvedEvents;
  }

  public reset(): void {
    this.candidates = [];
  }
}
