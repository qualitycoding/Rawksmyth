export interface ChartNote {
  id: number;
  timestampMs: number;
  stringIndex: number; // 6 = Low E (Red), 5 = A (Yellow), 4 = D (Blue), 3 = G (Orange), 2 = B (Green), 1 = High E (Purple)
  fretNumber: number; // 0 = open
  midiNote: number;
  frequencyHz: number;
  durationMs: number;
  noteName: string;
}

export interface SongChart {
  id: string;
  title: string;
  artist: string;
  bpm: number;
  durationMs: number;
  tuning: string;
  notes: ChartNote[];
}

export const GUITAR_STRINGS = [
  { index: 6, name: 'E', note: 'E2', midi: 40, freq: 82.41, color: '#EF4444', label: '6th (Low E)' },
  { index: 5, name: 'A', note: 'A2', midi: 45, freq: 110.00, color: '#EAB308', label: '5th (A)' },
  { index: 4, name: 'D', note: 'D3', midi: 50, freq: 146.83, color: '#3B82F6', label: '4th (D)' },
  { index: 3, name: 'G', note: 'G3', midi: 55, freq: 196.00, color: '#F97316', label: '3rd (G)' },
  { index: 2, name: 'B', note: 'B3', midi: 59, freq: 246.94, color: '#10B981', label: '2nd (B)' },
  { index: 1, name: 'e', note: 'E4', midi: 64, freq: 329.63, color: '#A855F7', label: '1st (High E)' },
];

export function getMidiForFret(stringIndex: number, fret: number): { midiNote: number; frequencyHz: number; noteName: string } {
  const baseString = GUITAR_STRINGS.find((s) => s.index === stringIndex) || GUITAR_STRINGS[0];
  const midi = baseString.midi + fret;
  const freq = 440 * Math.pow(2, (midi - 69) / 12);

  const names = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
  const octave = Math.floor(midi / 12) - 1;
  const noteName = `${names[midi % 12]}${octave}`;

  return { midiNote: midi, frequencyHz: freq, noteName };
}

export const EXAMPLE_SONGS: SongChart[] = [
  {
    id: 'thunder-riff',
    title: 'Thunder Riff (Rocksmith Anthem)',
    artist: 'Galactic Sound Lab',
    bpm: 120,
    durationMs: 32000,
    tuning: 'E Standard',
    notes: [
      // Measure 1: Low E chug and power hit
      { id: 1, timestampMs: 2000, stringIndex: 6, fretNumber: 0, ...getMidiForFret(6, 0), durationMs: 400 },
      { id: 2, timestampMs: 2500, stringIndex: 6, fretNumber: 0, ...getMidiForFret(6, 0), durationMs: 400 },
      { id: 3, timestampMs: 3000, stringIndex: 6, fretNumber: 3, ...getMidiForFret(6, 3), durationMs: 400 },
      { id: 4, timestampMs: 3500, stringIndex: 5, fretNumber: 2, ...getMidiForFret(5, 2), durationMs: 450 },

      // Measure 2: Open A to D string punch
      { id: 5, timestampMs: 4500, stringIndex: 5, fretNumber: 0, ...getMidiForFret(5, 0), durationMs: 400 },
      { id: 6, timestampMs: 5000, stringIndex: 5, fretNumber: 2, ...getMidiForFret(5, 2), durationMs: 400 },
      { id: 7, timestampMs: 5500, stringIndex: 4, fretNumber: 0, ...getMidiForFret(4, 0), durationMs: 400 },
      { id: 8, timestampMs: 6000, stringIndex: 4, fretNumber: 2, ...getMidiForFret(4, 2), durationMs: 600 },

      // Measure 3: Power 5th groove
      { id: 9, timestampMs: 7500, stringIndex: 6, fretNumber: 0, ...getMidiForFret(6, 0), durationMs: 400 },
      { id: 10, timestampMs: 8000, stringIndex: 6, fretNumber: 5, ...getMidiForFret(6, 5), durationMs: 400 },
      { id: 11, timestampMs: 8500, stringIndex: 6, fretNumber: 3, ...getMidiForFret(6, 3), durationMs: 400 },
      { id: 12, timestampMs: 9000, stringIndex: 6, fretNumber: 0, ...getMidiForFret(6, 0), durationMs: 600 },

      // Measure 4: Pentatonic fill across G and B
      { id: 13, timestampMs: 10500, stringIndex: 3, fretNumber: 0, ...getMidiForFret(3, 0), durationMs: 400 },
      { id: 14, timestampMs: 11000, stringIndex: 3, fretNumber: 2, ...getMidiForFret(3, 2), durationMs: 400 },
      { id: 15, timestampMs: 11500, stringIndex: 2, fretNumber: 0, ...getMidiForFret(2, 0), durationMs: 400 },
      { id: 16, timestampMs: 12000, stringIndex: 2, fretNumber: 3, ...getMidiForFret(2, 3), durationMs: 600 },

      // Measure 5: Climax high E scream
      { id: 17, timestampMs: 13500, stringIndex: 1, fretNumber: 0, ...getMidiForFret(1, 0), durationMs: 400 },
      { id: 18, timestampMs: 14000, stringIndex: 1, fretNumber: 3, ...getMidiForFret(1, 3), durationMs: 500 },
      { id: 19, timestampMs: 15000, stringIndex: 6, fretNumber: 0, ...getMidiForFret(6, 0), durationMs: 1000 },
    ],
  },
  {
    id: 'pentatonic-blues',
    title: 'A Minor Pentatonic Lead',
    artist: 'Delta Electric Trio',
    bpm: 95,
    durationMs: 25000,
    tuning: 'E Standard',
    notes: [
      { id: 101, timestampMs: 2000, stringIndex: 6, fretNumber: 5, ...getMidiForFret(6, 5), durationMs: 500 },
      { id: 102, timestampMs: 2800, stringIndex: 6, fretNumber: 8, ...getMidiForFret(6, 8), durationMs: 500 },
      { id: 103, timestampMs: 3600, stringIndex: 5, fretNumber: 5, ...getMidiForFret(5, 5), durationMs: 500 },
      { id: 104, timestampMs: 4400, stringIndex: 5, fretNumber: 7, ...getMidiForFret(5, 7), durationMs: 500 },
      { id: 105, timestampMs: 5200, stringIndex: 4, fretNumber: 5, ...getMidiForFret(4, 5), durationMs: 500 },
      { id: 106, timestampMs: 6000, stringIndex: 4, fretNumber: 7, ...getMidiForFret(4, 7), durationMs: 500 },
      { id: 107, timestampMs: 6800, stringIndex: 3, fretNumber: 5, ...getMidiForFret(3, 5), durationMs: 500 },
      { id: 108, timestampMs: 7600, stringIndex: 3, fretNumber: 7, ...getMidiForFret(3, 7), durationMs: 800 },
    ],
  },
];
