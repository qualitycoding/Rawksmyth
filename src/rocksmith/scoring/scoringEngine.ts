import { ChartNote } from '../chart/chartModel';
import { ResolvedNoteEvent } from '../dsp/temporalTracker';

export type JudgmentType = 'PERFECT' | 'GREAT' | 'GOOD' | 'MISS';

export interface ScoredEvent {
  noteId: number;
  judgment: JudgmentType;
  timeDiffMs: number;
  centsDeviation: number;
  expectedNote: string;
  detectedNote: string;
  timestampMs: number;
}

export interface ScoreState {
  score: number;
  streak: number;
  maxStreak: number;
  multiplier: number;
  accuracy: number;
  totalNotes: number;
  perfectCount: number;
  greatCount: number;
  goodCount: number;
  missCount: number;
}

export class ScoringEngine {
  private activeTargets: ChartNote[] = [];
  private scoredEvents: ScoredEvent[] = [];
  private state: ScoreState = {
    score: 0,
    streak: 0,
    maxStreak: 0,
    multiplier: 1,
    accuracy: 100,
    totalNotes: 0,
    perfectCount: 0,
    greatCount: 0,
    goodCount: 0,
    missCount: 0,
  };

  private perfectWindowMs = 15;
  private greatWindowMs = 30;
  private goodWindowMs = 50;
  private pitchToleranceCents = 35;

  public loadChart(notes: ChartNote[]): void {
    this.activeTargets = [...notes];
    this.scoredEvents = [];
    this.state = {
      score: 0,
      streak: 0,
      maxStreak: 0,
      multiplier: 1,
      accuracy: 100,
      totalNotes: 0,
      perfectCount: 0,
      greatCount: 0,
      goodCount: 0,
      missCount: 0,
    };
  }

  public evaluateDetectedNote(event: ResolvedNoteEvent): ScoredEvent | null {
    let bestIdx = -1;
    let minDiff = Infinity;

    for (let i = 0; i < this.activeTargets.length; i++) {
      const target = this.activeTargets[i];
      const diff = Math.abs(event.onsetTimestampMs - target.timestampMs);
      if (diff < minDiff) {
        minDiff = diff;
        bestIdx = i;
      }
    }

    if (bestIdx !== -1) {
      const target = this.activeTargets[bestIdx];
      const timeDiff = event.onsetTimestampMs - target.timestampMs;
      const absTimeDiff = Math.abs(timeDiff);
      const isPitchMatch = event.midiNote === target.midiNote;
      const absCents = Math.abs(event.centsDeviation);

      if (absTimeDiff <= this.goodWindowMs && isPitchMatch && absCents <= this.pitchToleranceCents) {
        let judgment: JudgmentType = 'GOOD';
        let basePoints = 50;

        if (absTimeDiff <= this.perfectWindowMs && absCents <= 10) {
          judgment = 'PERFECT';
          basePoints = 150;
        } else if (absTimeDiff <= this.greatWindowMs && absCents <= 20) {
          judgment = 'GREAT';
          basePoints = 100;
        }

        const scored: ScoredEvent = {
          noteId: target.id,
          judgment,
          timeDiffMs: timeDiff,
          centsDeviation: event.centsDeviation,
          expectedNote: target.noteName,
          detectedNote: event.noteName,
          timestampMs: event.onsetTimestampMs,
        };

        this.applyHit(scored, basePoints);
        this.activeTargets.splice(bestIdx, 1);
        return scored;
      }
    }

    return null;
  }

  public checkExpired(currentTimeMs: number): ScoredEvent[] {
    const expiredEvents: ScoredEvent[] = [];

    while (this.activeTargets.length > 0) {
      const front = this.activeTargets[0];
      if (currentTimeMs - front.timestampMs > this.goodWindowMs) {
        const expired = this.activeTargets.shift()!;
        const scored: ScoredEvent = {
          noteId: expired.id,
          judgment: 'MISS',
          timeDiffMs: this.goodWindowMs + 10,
          centsDeviation: 0,
          expectedNote: expired.noteName,
          detectedNote: 'None',
          timestampMs: currentTimeMs,
        };

        this.applyMiss(scored);
        expiredEvents.push(scored);
      } else {
        break;
      }
    }

    return expiredEvents;
  }

  private applyHit(hit: ScoredEvent, basePoints: number): void {
    this.state.streak++;
    if (this.state.streak > this.state.maxStreak) {
      this.state.maxStreak = this.state.streak;
    }

    // Rocksmith / rhythm streak multiplier
    if (this.state.streak >= 30) this.state.multiplier = 4;
    else if (this.state.streak >= 20) this.state.multiplier = 3;
    else if (this.state.streak >= 10) this.state.multiplier = 2;
    else this.state.multiplier = 1;

    this.state.score += basePoints * this.state.multiplier;

    if (hit.judgment === 'PERFECT') this.state.perfectCount++;
    else if (hit.judgment === 'GREAT') this.state.greatCount++;
    else if (hit.judgment === 'GOOD') this.state.goodCount++;

    this.recalculateAccuracy();
    this.scoredEvents.push(hit);
  }

  private applyMiss(miss: ScoredEvent): void {
    this.state.streak = 0;
    this.state.multiplier = 1;
    this.state.missCount++;
    this.recalculateAccuracy();
    this.scoredEvents.push(miss);
  }

  private recalculateAccuracy(): void {
    const hits = this.state.perfectCount + this.state.greatCount + this.state.goodCount;
    this.state.totalNotes = hits + this.state.missCount;
    this.state.accuracy = this.state.totalNotes > 0
      ? Math.round((hits / this.state.totalNotes) * 1000) / 10
      : 100;
  }

  public getState(): ScoreState {
    return { ...this.state };
  }

  public getScoredEvents(): ScoredEvent[] {
    return [...this.scoredEvents];
  }
}
