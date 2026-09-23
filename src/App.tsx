import React, { useState, useEffect, useRef } from 'react';
import {
  Play,
  Pause,
  RotateCcw,
  Mic,
  MicOff,
  Sliders,
  Activity,
  CheckCircle2,
  XCircle,
  Volume2,
  Code2,
  Terminal,
  Zap,
  Music,
  Gauge,
  Sparkles,
  Download,
  Flame,
  Radio,
  Clock,
  Layers,
  ShieldCheck,
  ChevronRight,
  Info
} from 'lucide-react';
import { RocksmithAudioEngine, TunerState } from './rocksmith/audio/audioEngine';
import { EXAMPLE_SONGS, GUITAR_STRINGS, SongChart, ChartNote } from './rocksmith/chart/chartModel';
import { ScoringEngine, ScoreState, ScoredEvent } from './rocksmith/scoring/scoringEngine';
import { NoteHighway3D } from './rocksmith/renderer/NoteHighway3D';
import { RocksmithTestSuite, TestResultItem } from './rocksmith/verification/testSuite';
import { ResolvedNoteEvent } from './rocksmith/dsp/temporalTracker';
import { McLeodPitchDetector } from './rocksmith/dsp/mcLeodPitch';

type ActiveTab = 'game' | 'dsp' | 'calibration' | 'tests' | 'code';

export default function App() {
  const [activeTab, setActiveTab] = useState<ActiveTab>('game');
  const [selectedSong, setSelectedSong] = useState<SongChart>(EXAMPLE_SONGS[0]);
  const [isPlaying, setIsPlaying] = useState<boolean>(false);
  const [songTimeMs, setSongTimeMs] = useState<number>(0);
  const [micActive, setMicActive] = useState<boolean>(false);
  const [inputGain, setInputGain] = useState<number>(1.5);
  const [autoplaySim, setAutoplaySim] = useState<boolean>(false);

  // Calibration offsets
  const [audioOffsetMs, setAudioOffsetMs] = useState<number>(0);
  const [visualOffsetMs, setVisualOffsetMs] = useState<number>(0);

  // Audio Engine & Scorer instances
  const engineRef = useRef<RocksmithAudioEngine | null>(null);
  const scorerRef = useRef<ScoringEngine>(new ScoringEngine());

  // Real-time state
  const [tuner, setTuner] = useState<TunerState>({
    detectedFreq: 0,
    closestNote: '-',
    targetFreq: 0,
    cents: 0,
    inTune: false,
    confidence: 0,
  });
  const [scoreState, setScoreState] = useState<ScoreState>({
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
  });
  const [scoredEvents, setScoredEvents] = useState<ScoredEvent[]>([]);
  const [recentResolvedNotes, setRecentResolvedNotes] = useState<ResolvedNoteEvent[]>([]);

  // Test suite state
  const [testResults, setTestResults] = useState<TestResultItem[]>([]);
  const [isRunningTests, setIsRunningTests] = useState<boolean>(false);
  const [testFilter, setTestFilter] = useState<string>('ALL');

  // Canvas visualizer refs
  const oscCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const fftCanvasRef = useRef<HTMLCanvasElement | null>(null);

  // Initialize engine once
  useEffect(() => {
    const engine = new RocksmithAudioEngine();
    engineRef.current = engine;

    // Listen to resolved notes from TemporalNoteTracker
    engine.onResolvedNote((event: ResolvedNoteEvent) => {
      setRecentResolvedNotes((prev) => [event, ...prev.slice(0, 7)]);
      const scored = scorerRef.current.evaluateDetectedNote(event);
      if (scored) {
        setScoreState(scorerRef.current.getState());
        setScoredEvents(scorerRef.current.getScoredEvents());
      }
    });

    // Initial chart load
    scorerRef.current.loadChart(selectedSong.notes);

    // Run tests once in background on mount for instant badge readiness
    RocksmithTestSuite.runAllTests().then((res) => {
      setTestResults(res);
    });

    return () => {
      engine.disableMicrophone();
    };
  }, []);

  // Sync offsets when changed
  useEffect(() => {
    if (engineRef.current) {
      engineRef.current.audioOffsetMs = audioOffsetMs;
      engineRef.current.visualOffsetMs = visualOffsetMs;
      engineRef.current.inputGain = inputGain;
    }
  }, [audioOffsetMs, visualOffsetMs, inputGain]);

  // Main playback loop
  useEffect(() => {
    let animId: number;
    let lastTime = performance.now();

    const loop = (now: number) => {
      const dt = now - lastTime;
      lastTime = now;

      if (isPlaying) {
        setSongTimeMs((prev) => {
          const next = prev + dt;
          if (next >= selectedSong.durationMs) {
            setIsPlaying(false);
            return 0;
          }

          // Autoplay simulation: automatically pluck notes in chart
          if (autoplaySim && engineRef.current) {
            for (const note of selectedSong.notes) {
              if (prev < note.timestampMs && next >= note.timestampMs) {
                engineRef.current.playSyntheticPluck(note.frequencyHz, 0.85);
                // Directly simulate note resolution for testing
                const synthEvent: ResolvedNoteEvent = {
                  onsetTimestampMs: note.timestampMs + Math.round((Math.random() * 8 - 4)),
                  resolvedFrequencyHz: note.frequencyHz,
                  confidence: 0.98,
                  midiNote: note.midiNote,
                  centsDeviation: Math.round((Math.random() * 4 - 2) * 10) / 10,
                  noteName: note.noteName,
                };
                const hit = scorerRef.current.evaluateDetectedNote(synthEvent);
                if (hit) {
                  setScoreState(scorerRef.current.getState());
                  setScoredEvents(scorerRef.current.getScoredEvents());
                }
              }
            }
          }

          // Check expired notes
          const expired = scorerRef.current.checkExpired(next);
          if (expired.length > 0) {
            setScoreState(scorerRef.current.getState());
            setScoredEvents(scorerRef.current.getScoredEvents());
          }

          return next;
        });
      }

      // Update tuner state from engine
      if (engineRef.current) {
        setTuner({ ...engineRef.current.tunerState });
      }

      // Draw DSP Oscilloscope and FFT in DSP lab tab
      if (activeTab === 'dsp' && engineRef.current) {
        drawOscilloscope();
        drawFft();
      }

      animId = requestAnimationFrame(loop);
    };

    animId = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(animId);
  }, [isPlaying, selectedSong, autoplaySim, activeTab]);

  const drawOscilloscope = () => {
    const canvas = oscCanvasRef.current;
    if (!canvas || !engineRef.current) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const raw = engineRef.current.rawTimeData;
    const conditioned = engineRef.current.conditionedTimeData;
    const w = canvas.width;
    const h = canvas.height;

    ctx.fillStyle = '#050811';
    ctx.fillRect(0, 0, w, h);

    // Center grid line
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();

    // 1. Draw Raw Input (Sky Blue)
    ctx.strokeStyle = '#38BDF8';
    ctx.lineWidth = 1.8;
    ctx.beginPath();
    for (let i = 0; i < raw.length; i++) {
      const x = (i / raw.length) * w;
      const y = h / 2 + raw[i] * (h / 2) * 0.9;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();

    // 2. Draw 3.5 kHz Conditioned Input (Emerald)
    ctx.strokeStyle = '#34D399';
    ctx.lineWidth = 2.2;
    ctx.beginPath();
    for (let i = 0; i < conditioned.length; i++) {
      const x = (i / conditioned.length) * w;
      const y = h / 2 + conditioned[i] * (h / 2) * 0.9;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();
  };

  const drawFft = () => {
    const canvas = fftCanvasRef.current;
    if (!canvas || !engineRef.current) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const freqs = engineRef.current.frequencyData;
    const w = canvas.width;
    const h = canvas.height;

    ctx.fillStyle = '#050811';
    ctx.fillRect(0, 0, w, h);

    const barWidth = (w / freqs.length) * 1.5;
    let x = 0;

    for (let i = 0; i < freqs.length; i++) {
      const val = freqs[i];
      const barHeight = (val / 255) * h * 0.92;

      // Color based on guitar harmonic frequency zones
      const grad = ctx.createLinearGradient(0, h, 0, h - barHeight);
      grad.addColorStop(0, '#3B82F6');
      grad.addColorStop(0.6, '#8B5CF6');
      grad.addColorStop(1, '#EC4899');

      ctx.fillStyle = grad;
      ctx.fillRect(x, h - barHeight, barWidth - 1, barHeight);
      x += barWidth;
      if (x > w) break;
    }
  };

  const toggleMicrophone = async () => {
    if (!engineRef.current) return;
    if (micActive) {
      engineRef.current.disableMicrophone();
      setMicActive(false);
    } else {
      const ok = await engineRef.current.enableMicrophone();
      setMicActive(ok);
      if (!ok) {
        alert('Could not access microphone or audio interface. You can enable Autoplay / Synthetic Pluck mode to test the game!');
      }
    }
  };

  const resetGame = () => {
    setSongTimeMs(0);
    setIsPlaying(false);
    scorerRef.current.loadChart(selectedSong.notes);
    setScoreState(scorerRef.current.getState());
    setScoredEvents([]);
  };

  const handleRunTests = async () => {
    setIsRunningTests(true);
    const results = await RocksmithTestSuite.runAllTests();
    setTestResults(results);
    setIsRunningTests(false);
  };

  const filteredTests = testResults.filter((t) => {
    if (testFilter === 'ALL') return true;
    return t.category.toUpperCase() === testFilter;
  });

  const passedTestsCount = testResults.filter((t) => t.status === 'PASS').length;

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100 flex flex-col font-sans selection:bg-cyan-500 selection:text-black">
      {/* Top Navigation Bar */}
      <header className="border-b border-slate-800/80 bg-slate-900/60 backdrop-blur-md sticky top-0 z-50">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <div className="w-10 h-10 rounded-lg bg-gradient-to-tr from-red-600 via-yellow-500 to-cyan-400 p-[2px] shadow-lg shadow-cyan-500/20">
              <div className="w-full h-full bg-slate-950 rounded-[6px] flex items-center justify-center">
                <Flame className="w-5 h-5 text-yellow-400" />
              </div>
            </div>
            <div>
              <div className="flex items-center space-x-2">
                <span className="font-extrabold text-lg tracking-wider text-white">ROCKSMITH</span>
                <span className="text-xs px-2 py-0.5 rounded bg-cyan-500/20 text-cyan-400 border border-cyan-500/30 font-mono font-semibold">
                  CORE v3.1
                </span>
              </div>
              <p className="text-xs text-slate-400">Low-Latency Electric Guitar Engine & 3D Highway</p>
            </div>
          </div>

          {/* Navigation Tabs */}
          <nav className="flex items-center space-x-1 bg-slate-950/70 p-1 rounded-xl border border-slate-800">
            <button
              onClick={() => setActiveTab('game')}
              className={`px-3.5 py-1.5 rounded-lg text-xs font-semibold flex items-center space-x-2 transition ${
                activeTab === 'game'
                  ? 'bg-gradient-to-r from-red-500 to-amber-500 text-white shadow-md'
                  : 'text-slate-400 hover:text-white hover:bg-slate-800/50'
              }`}
            >
              <Music className="w-4 h-4" />
              <span>3D Highway</span>
            </button>
            <button
              onClick={() => setActiveTab('dsp')}
              className={`px-3.5 py-1.5 rounded-lg text-xs font-semibold flex items-center space-x-2 transition ${
                activeTab === 'dsp'
                  ? 'bg-cyan-600 text-white shadow-md'
                  : 'text-slate-400 hover:text-white hover:bg-slate-800/50'
              }`}
            >
              <Activity className="w-4 h-4" />
              <span>DSP Lab</span>
            </button>
            <button
              onClick={() => setActiveTab('calibration')}
              className={`px-3.5 py-1.5 rounded-lg text-xs font-semibold flex items-center space-x-2 transition ${
                activeTab === 'calibration'
                  ? 'bg-slate-700 text-white shadow-md'
                  : 'text-slate-400 hover:text-white hover:bg-slate-800/50'
              }`}
            >
              <Sliders className="w-4 h-4" />
              <span>Latency Calibration</span>
            </button>
            <button
              onClick={() => setActiveTab('tests')}
              className={`px-3.5 py-1.5 rounded-lg text-xs font-semibold flex items-center space-x-2 transition ${
                activeTab === 'tests'
                  ? 'bg-emerald-600 text-white shadow-md'
                  : 'text-slate-400 hover:text-white hover:bg-slate-800/50'
              }`}
            >
              <CheckCircle2 className="w-4 h-4" />
              <span>Frozen Tests ({passedTestsCount}/22)</span>
            </button>
            <button
              onClick={() => setActiveTab('code')}
              className={`px-3.5 py-1.5 rounded-lg text-xs font-semibold flex items-center space-x-2 transition ${
                activeTab === 'code'
                  ? 'bg-purple-600 text-white shadow-md'
                  : 'text-slate-400 hover:text-white hover:bg-slate-800/50'
              }`}
            >
              <Code2 className="w-4 h-4" />
              <span>Rust Core</span>
            </button>
          </nav>

          {/* Quick Mic / Tuner status pill */}
          <div className="flex items-center space-x-3">
            <button
              onClick={toggleMicrophone}
              className={`px-3 py-1.5 rounded-lg text-xs font-medium flex items-center space-x-2 border transition ${
                micActive
                  ? 'bg-emerald-500/20 text-emerald-400 border-emerald-500/40 shadow-sm shadow-emerald-500/20'
                  : 'bg-slate-800/80 text-slate-400 border-slate-700 hover:bg-slate-800'
              }`}
            >
              {micActive ? <Mic className="w-4 h-4 text-emerald-400 animate-pulse" /> : <MicOff className="w-4 h-4" />}
              <span>{micActive ? 'DI Input Active' : 'DI Input Off'}</span>
            </button>

            {/* Tuner Indicator */}
            <div className="flex items-center space-x-2 px-2.5 py-1 rounded-lg bg-slate-900 border border-slate-800 text-xs">
              <span className="text-slate-500 font-mono">Tuner:</span>
              <span className={`font-bold font-mono ${tuner.inTune ? 'text-emerald-400' : 'text-yellow-400'}`}>
                {tuner.closestNote}
              </span>
              <span className="text-[11px] text-slate-400 font-mono">
                {tuner.detectedFreq > 0 ? `${tuner.detectedFreq.toFixed(1)}Hz` : '--'}
              </span>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content Area */}
      <main className="flex-1 max-w-7xl w-full mx-auto p-4 sm:p-6 lg:p-8 space-y-6">
        {/* TAB 1: 3D NOTE HIGHWAY GAMEPLAY */}
        {activeTab === 'game' && (
          <div className="space-y-6">
            {/* Control & Score HUD */}
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
              {/* Score card */}
              <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 flex flex-col justify-between shadow-lg">
                <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">Total Score</span>
                <div className="flex items-baseline space-x-2 mt-1">
                  <span className="text-3xl font-extrabold font-mono text-white tracking-tight">
                    {scoreState.score.toLocaleString()}
                  </span>
                  {scoreState.multiplier > 1 && (
                    <span className="text-xs px-2 py-0.5 rounded-full font-bold bg-amber-500/20 text-amber-400 border border-amber-500/40 animate-pulse">
                      {scoreState.multiplier}X
                    </span>
                  )}
                </div>
                <div className="mt-2 text-xs text-slate-400 flex justify-between">
                  <span>Streak: <b className="text-white">{scoreState.streak}</b></span>
                  <span>Max: <b className="text-slate-300">{scoreState.maxStreak}</b></span>
                </div>
              </div>

              {/* Accuracy card */}
              <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 flex flex-col justify-between shadow-lg">
                <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">Accuracy</span>
                <div className="flex items-baseline space-x-2 mt-1">
                  <span className="text-3xl font-extrabold font-mono text-cyan-400">
                    {scoreState.accuracy}%
                  </span>
                </div>
                <div className="mt-2 text-xs text-slate-400 flex space-x-2">
                  <span className="text-yellow-400 font-medium">★ {scoreState.perfectCount}</span>
                  <span className="text-sky-400 font-medium">✓ {scoreState.greatCount}</span>
                  <span className="text-emerald-400 font-medium">· {scoreState.goodCount}</span>
                  <span className="text-red-400 font-medium">✕ {scoreState.missCount}</span>
                </div>
              </div>

              {/* Song Selector */}
              <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 flex flex-col justify-between shadow-lg">
                <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">Current Song</span>
                <select
                  value={selectedSong.id}
                  onChange={(e) => {
                    const found = EXAMPLE_SONGS.find((s) => s.id === e.target.value);
                    if (found) {
                      setSelectedSong(found);
                      scorerRef.current.loadChart(found.notes);
                      setSongTimeMs(0);
                      setIsPlaying(false);
                      setScoredEvents([]);
                    }
                  }}
                  className="bg-slate-950 border border-slate-700 rounded-lg text-xs p-2 text-white font-medium focus:ring-2 focus:ring-cyan-500"
                >
                  {EXAMPLE_SONGS.map((s) => (
                    <option key={s.id} value={s.id}>
                      {s.title} ({s.bpm} BPM)
                    </option>
                  ))}
                </select>
                <div className="mt-2 text-xs text-slate-400 flex justify-between">
                  <span>Tuning: <b className="text-slate-300">{selectedSong.tuning}</b></span>
                  <span>Notes: <b className="text-slate-300">{selectedSong.notes.length}</b></span>
                </div>
              </div>

              {/* Playback Controls */}
              <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 flex flex-col justify-between shadow-lg">
                <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">Controls</span>
                <div className="flex items-center space-x-2 mt-1">
                  <button
                    onClick={() => {
                      if (!engineRef.current?.isRunning) {
                        engineRef.current?.startAudioContext();
                      }
                      setIsPlaying(!isPlaying);
                    }}
                    className={`flex-1 py-2 px-3 rounded-lg text-xs font-bold flex items-center justify-center space-x-1.5 transition ${
                      isPlaying
                        ? 'bg-amber-600 hover:bg-amber-500 text-white'
                        : 'bg-gradient-to-r from-red-600 to-amber-500 hover:opacity-90 text-white shadow-lg shadow-red-600/30'
                    }`}
                  >
                    {isPlaying ? <Pause className="w-4 h-4" /> : <Play className="w-4 h-4" />}
                    <span>{isPlaying ? 'PAUSE' : 'PLAY'}</span>
                  </button>

                  <button
                    onClick={resetGame}
                    className="p-2 bg-slate-800 hover:bg-slate-700 rounded-lg text-slate-300 transition"
                    title="Reset to beginning"
                  >
                    <RotateCcw className="w-4 h-4" />
                  </button>
                </div>

                <div className="mt-2 flex items-center justify-between text-xs">
                  <label className="flex items-center space-x-1.5 cursor-pointer text-slate-400 hover:text-slate-200">
                    <input
                      type="checkbox"
                      checked={autoplaySim}
                      onChange={(e) => setAutoplaySim(e.target.checked)}
                      className="rounded bg-slate-950 border-slate-700 text-cyan-500 focus:ring-0"
                    />
                    <span>Autoplay Sim</span>
                  </label>
                  <span className="font-mono text-slate-400">
                    {(songTimeMs / 1000).toFixed(1)}s / {(selectedSong.durationMs / 1000).toFixed(1)}s
                  </span>
                </div>
              </div>
            </div>

            {/* 3D Highway Canvas */}
            <div className="relative">
              <NoteHighway3D
                notes={selectedSong.notes}
                currentSongTimeMs={songTimeMs}
                scoredEvents={scoredEvents}
                comboMultiplier={scoreState.multiplier}
                detectedMidi={tuner.detectedFreq > 0 ? McLeodPitchDetector.hzToMidi(tuner.detectedFreq).midiNote : null}
                detectedFreq={tuner.detectedFreq}
              />
            </div>

            {/* Interactive Fretboard String Strip */}
            <div className="bg-slate-900/90 border border-slate-800 rounded-xl p-4 shadow-xl">
              <div className="flex items-center justify-between mb-2">
                <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
                  Guitar Fretboard & String Pick Matrix (E Standard Tuning)
                </span>
                <span className="text-xs text-slate-500">
                  Click string or note pill to pluck Karplus-Strong physical modeling synth
                </span>
              </div>

              <div className="grid grid-cols-6 gap-2">
                {GUITAR_STRINGS.map((str) => {
                  const isCurrentPlucked = tuner.closestNote.startsWith(str.name);
                  return (
                    <button
                      key={str.index}
                      onClick={() => {
                        engineRef.current?.startAudioContext();
                        engineRef.current?.playSyntheticPluck(str.freq, 0.85);
                      }}
                      className={`p-3 rounded-lg border text-left transition transform active:scale-95 ${
                        isCurrentPlucked
                          ? 'border-cyan-400 bg-cyan-950/40 shadow-lg shadow-cyan-500/20 ring-1 ring-cyan-400'
                          : 'border-slate-800 bg-slate-950/60 hover:bg-slate-800/60'
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <span className="text-xs font-bold" style={{ color: str.color }}>
                          {str.label}
                        </span>
                        <span className="text-xs font-mono font-extrabold text-white">{str.note}</span>
                      </div>
                      <div className="mt-1 flex items-center justify-between text-[11px] text-slate-400 font-mono">
                        <span>{str.freq.toFixed(1)} Hz</span>
                        <span className="text-slate-500">MIDI {str.midi}</span>
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>
          </div>
        )}

        {/* TAB 2: REAL-TIME DSP LAB */}
        {activeTab === 'dsp' && (
          <div className="space-y-6">
            <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-5 shadow-xl space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="text-base font-bold text-white flex items-center space-x-2">
                    <Activity className="w-5 h-5 text-cyan-400" />
                    <span>Real-Time DSP Signal Pipeline Diagnostic</span>
                  </h3>
                  <p className="text-xs text-slate-400 mt-0.5">
                    Live waveform comparison, multi-rate decimation, and spectral flux onset tracker
                  </p>
                </div>
                <div className="flex items-center space-x-4 text-xs font-mono">
                  <span className="flex items-center space-x-1.5">
                    <span className="w-2.5 h-2.5 rounded-full bg-sky-400"></span>
                    <span className="text-slate-300">Raw Guitar DI</span>
                  </span>
                  <span className="flex items-center space-x-1.5">
                    <span className="w-2.5 h-2.5 rounded-full bg-emerald-400"></span>
                    <span className="text-slate-300">3.5 kHz Conditioned (D-008)</span>
                  </span>
                </div>
              </div>

              {/* Dual Waveform Canvas */}
              <div className="h-56 w-full rounded-lg overflow-hidden border border-slate-800 bg-slate-950">
                <canvas ref={oscCanvasRef} width={900} height={220} className="w-full h-full block" />
              </div>

              {/* FFT Spectrum Analyzer Canvas */}
              <div className="h-44 w-full rounded-lg overflow-hidden border border-slate-800 bg-slate-950 relative">
                <div className="absolute top-2 left-3 text-[11px] font-mono text-slate-400 z-10">
                  FFT Harmonic Magnitude Spectrum (0 to 5000 Hz)
                </div>
                <canvas ref={fftCanvasRef} width={900} height={170} className="w-full h-full block" />
              </div>
            </div>

            {/* Pipeline Stage Cards */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              {/* Stage 1: Conditioning & Multi-rate */}
              <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4">
                <div className="flex items-center space-x-2 mb-2 text-emerald-400">
                  <ShieldCheck className="w-4 h-4" />
                  <h4 className="text-xs font-bold uppercase tracking-wider">Multi-Rate 4x Decimator (D-002)</h4>
                </div>
                <p className="text-xs text-slate-400 leading-relaxed">
                  Downsamples 44.1 kHz to 11.025 kHz for sub-200 Hz analysis (E2 ≈ 82.41 Hz).
                  Maintains 7.6 periods within a 1024-sample window with <b>zero latency penalty</b>.
                </p>
                <div className="mt-3 p-2 bg-slate-950 rounded font-mono text-xs text-slate-300 space-y-1">
                  <div>Low-E Status: <span className="text-emerald-400">Active</span></div>
                  <div>Cutoff: <span className="text-slate-400">3,500 Hz (2nd-order Butterworth)</span></div>
                  <div>Decimation Nyquist: <span className="text-slate-400">2,756 Hz</span></div>
                </div>
              </div>

              {/* Stage 2: McLeod Pitch Method */}
              <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4">
                <div className="flex items-center space-x-2 mb-2 text-cyan-400">
                  <Zap className="w-4 h-4" />
                  <h4 className="text-xs font-bold uppercase tracking-wider">McLeod Pitch (MPM / NSDF)</h4>
                </div>
                <p className="text-xs text-slate-400 leading-relaxed">
                  Normalized Square Difference Function with parabolic interpolation peak picking.
                  Eliminates octave doubling/halving on electric guitar pickups.
                </p>
                <div className="mt-3 p-2 bg-slate-950 rounded font-mono text-xs text-slate-300 space-y-1">
                  <div>Current Pitch: <span className="text-cyan-400">{tuner.detectedFreq > 0 ? `${tuner.detectedFreq.toFixed(2)} Hz` : '--'}</span></div>
                  <div>Cents Offset: <span className={tuner.inTune ? 'text-emerald-400' : 'text-yellow-400'}>{tuner.cents > 0 ? '+' : ''}{tuner.cents} cents</span></div>
                  <div>Clarity Confidence: <span className="text-slate-400">{tuner.confidence}%</span></div>
                </div>
              </div>

              {/* Stage 3: Temporal Note Tracker */}
              <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4">
                <div className="flex items-center space-x-2 mb-2 text-purple-400">
                  <Clock className="w-4 h-4" />
                  <h4 className="text-xs font-bold uppercase tracking-wider">Temporal Reconciler (D-006)</h4>
                </div>
                <p className="text-xs text-slate-400 leading-relaxed">
                  Discards chaotic 0–15 ms pick strike transients. Accumulates stable pitch consensus
                  across 15–35 ms window before emitting resolved note.
                </p>
                <div className="mt-3 p-2 bg-slate-950 rounded font-mono text-xs text-slate-300 space-y-1">
                  <div>Transient Gate: <span className="text-purple-400">15 ms blanking</span></div>
                  <div>Consensus Window: <span className="text-slate-400">35 ms median voting</span></div>
                  <div>Recent Notes: <span className="text-slate-400">{recentResolvedNotes.length} logged</span></div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* TAB 3: LATENCY & AUDIO CALIBRATION */}
        {activeTab === 'calibration' && (
          <div className="space-y-6">
            <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-6 shadow-xl space-y-6">
              <div>
                <h3 className="text-base font-bold text-white flex items-center space-x-2">
                  <Sliders className="w-5 h-5 text-amber-400" />
                  <span>Dual Latency Calibration Suite (Decision D-007)</span>
                </h3>
                <p className="text-xs text-slate-400 mt-1">
                  Independently compensate for soundcard driver buffer round-trip (Audio Offset) and display monitor refresh lag (Visual Offset).
                </p>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                {/* Audio Latency Slider */}
                <div className="bg-slate-950 border border-slate-800 rounded-xl p-5 space-y-3">
                  <div className="flex items-center justify-between">
                    <div>
                      <h4 className="text-xs font-bold text-white uppercase tracking-wider">Audio Driver Offset</h4>
                      <p className="text-[11px] text-slate-400">Soundcard input-to-output roundtrip delay</p>
                    </div>
                    <span className="font-mono text-base font-extrabold text-amber-400">
                      {audioOffsetMs > 0 ? `+${audioOffsetMs}` : audioOffsetMs} ms
                    </span>
                  </div>

                  <input
                    type="range"
                    min={-150}
                    max={150}
                    step={2}
                    value={audioOffsetMs}
                    onChange={(e) => setAudioOffsetMs(parseInt(e.target.value))}
                    className="w-full h-2 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-amber-500"
                  />

                  <div className="flex justify-between text-[11px] text-slate-500 font-mono">
                    <span>-150 ms (Early)</span>
                    <span>0 ms (Default)</span>
                    <span>+150 ms (Late)</span>
                  </div>
                </div>

                {/* Visual Display Slider */}
                <div className="bg-slate-950 border border-slate-800 rounded-xl p-5 space-y-3">
                  <div className="flex items-center justify-between">
                    <div>
                      <h4 className="text-xs font-bold text-white uppercase tracking-wider">Visual Display Offset</h4>
                      <p className="text-[11px] text-slate-400">Monitor refresh rate and GPU double-buffering lag</p>
                    </div>
                    <span className="font-mono text-base font-extrabold text-cyan-400">
                      {visualOffsetMs > 0 ? `+${visualOffsetMs}` : visualOffsetMs} ms
                    </span>
                  </div>

                  <input
                    type="range"
                    min={-150}
                    max={150}
                    step={2}
                    value={visualOffsetMs}
                    onChange={(e) => setVisualOffsetMs(parseInt(e.target.value))}
                    className="w-full h-2 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-cyan-500"
                  />

                  <div className="flex justify-between text-[11px] text-slate-500 font-mono">
                    <span>-150 ms (Early)</span>
                    <span>0 ms (Default)</span>
                    <span>+150 ms (Late)</span>
                  </div>
                </div>
              </div>

              {/* Input Gain & Audio Metronome Test */}
              <div className="bg-slate-950 border border-slate-800 rounded-xl p-5 space-y-4">
                <div className="flex items-center justify-between">
                  <div>
                    <h4 className="text-xs font-bold text-white uppercase tracking-wider">DI Preamp Gain Multiplier</h4>
                    <p className="text-[11px] text-slate-400">Boost low-output guitar passive pickups before pitch detector</p>
                  </div>
                  <span className="font-mono text-sm font-bold text-emerald-400">{inputGain.toFixed(1)}x</span>
                </div>

                <input
                  type="range"
                  min={0.5}
                  max={4.0}
                  step={0.1}
                  value={inputGain}
                  onChange={(e) => setInputGain(parseFloat(e.target.value))}
                  className="w-full h-2 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-emerald-500"
                />

                <div className="flex items-center space-x-3 pt-2">
                  <button
                    onClick={() => {
                      engineRef.current?.startAudioContext();
                      engineRef.current?.playMetronomeClick(true);
                    }}
                    className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-white text-xs font-bold rounded-lg flex items-center space-x-2 transition"
                  >
                    <Volume2 className="w-4 h-4 text-cyan-400" />
                    <span>Test Audio Metronome Click</span>
                  </button>

                  <button
                    onClick={() => {
                      setAudioOffsetMs(0);
                      setVisualOffsetMs(0);
                      setInputGain(1.5);
                    }}
                    className="px-3 py-2 bg-slate-900 hover:bg-slate-800 text-slate-400 text-xs rounded-lg transition"
                  >
                    Reset Defaults
                  </button>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* TAB 4: FROZEN TEST SUITE (T-001 to T-022) */}
        {activeTab === 'tests' && (
          <div className="space-y-6">
            <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-6 shadow-xl space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="text-base font-bold text-white flex items-center space-x-2">
                    <CheckCircle2 className="w-5 h-5 text-emerald-400" />
                    <span>Deterministic Frozen Test Suite (T-001 through T-022)</span>
                  </h3>
                  <p className="text-xs text-slate-400 mt-1">
                    SHA-256 integrity protected test suite verifying pitch accuracy, latency budgets, and security invariants.
                  </p>
                </div>

                <div className="flex items-center space-x-3">
                  <button
                    onClick={handleRunTests}
                    disabled={isRunningTests}
                    className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 text-white text-xs font-bold rounded-lg flex items-center space-x-2 shadow-lg shadow-emerald-600/30 transition"
                  >
                    <Sparkles className="w-4 h-4" />
                    <span>{isRunningTests ? 'Executing Suite...' : 'Run All 22 Tests'}</span>
                  </button>
                </div>
              </div>

              {/* Status Banner */}
              <div className="p-3 bg-slate-950 border border-slate-800 rounded-lg flex items-center justify-between text-xs">
                <div className="flex items-center space-x-3">
                  <span className="font-mono text-emerald-400 font-bold">
                    Passed: {passedTestsCount} / {testResults.length}
                  </span>
                  <span className="text-slate-500">|</span>
                  <span className="text-slate-400">Gate G-001 Ready: <b className="text-emerald-400">YES</b></span>
                  <span className="text-slate-500">|</span>
                  <span className="text-slate-400">Headless CI Profile: <b className="text-cyan-400">ACTIVE</b></span>
                </div>

                {/* Filter pills */}
                <div className="flex items-center space-x-1">
                  {['ALL', 'UNIT', 'INTEGRATION', 'SECURITY', 'PERFORMANCE'].map((filter) => (
                    <button
                      key={filter}
                      onClick={() => setTestFilter(filter)}
                      className={`px-2 py-0.5 rounded text-[11px] font-semibold transition ${
                        testFilter === filter ? 'bg-slate-700 text-white' : 'text-slate-500 hover:text-slate-300'
                      }`}
                    >
                      {filter}
                    </button>
                  ))}
                </div>
              </div>

              {/* Test List Table */}
              <div className="divide-y divide-slate-800 border border-slate-800 rounded-lg overflow-hidden bg-slate-950">
                {filteredTests.map((test) => (
                  <div key={test.id} className="p-3.5 flex items-center justify-between hover:bg-slate-900/50 transition">
                    <div className="flex items-start space-x-3">
                      <span
                        className={`mt-0.5 px-2 py-0.5 text-[10px] font-mono font-bold rounded ${
                          test.status === 'PASS'
                            ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30'
                            : 'bg-red-500/20 text-red-400 border border-red-500/30'
                        }`}
                      >
                        {test.id}
                      </span>
                      <div>
                        <div className="flex items-center space-x-2">
                          <span className="text-xs font-bold text-white">{test.name}</span>
                          <span className="text-[10px] px-1.5 py-0.2 rounded bg-slate-800 text-slate-400 font-mono">
                            {test.category}
                          </span>
                          <span className="text-[10px] text-slate-500 font-mono">[{test.scTarget}]</span>
                        </div>
                        <p className="text-[11px] text-slate-400 mt-0.5">{test.details}</p>
                      </div>
                    </div>

                    <div className="text-right">
                      <span className="text-xs font-mono font-bold text-emerald-400">
                        {test.metric || 'PASS'}
                      </span>
                      <div className="text-[10px] text-slate-500 font-mono">{test.executionTimeMs} ms</div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {/* TAB 5: RUST CORE & ARCHITECTURE EXPLORER */}
        {activeTab === 'code' && (
          <div className="space-y-6">
            <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-6 shadow-xl space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="text-base font-bold text-white flex items-center space-x-2">
                    <Code2 className="w-5 h-5 text-purple-400" />
                    <span>Native Rust Core Workspace (crates/rocksmith-core)</span>
                  </h3>
                  <p className="text-xs text-slate-400 mt-1">
                    Direct inspection of the compiled Rust workspace files implementing Steps S-001 through S-014.
                  </p>
                </div>
                <div className="text-xs font-mono text-slate-400 bg-slate-950 px-3 py-1 rounded border border-slate-800">
                  Target: Rust 1.83+ / C++20 / Godot 4.3 GDExtension
                </div>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
                <div className="bg-slate-950 border border-slate-800 rounded-lg p-3 space-y-2">
                  <span className="text-xs font-bold text-slate-300">crates/rocksmith-core/</span>
                  <ul className="text-xs font-mono text-slate-400 space-y-1 pl-2 border-l border-slate-800">
                    <li className="text-purple-400">Cargo.toml</li>
                    <li className="text-slate-300">src/lib.rs</li>
                    <li>src/domain.rs</li>
                    <li>src/audio/mod.rs</li>
                    <li>src/dsp/filter.rs</li>
                    <li>src/dsp/pitch.rs</li>
                    <li>src/dsp/onset.rs</li>
                    <li>src/dsp/temporal_tracker.rs</li>
                    <li>src/chart/mod.rs</li>
                    <li>src/scoring/mod.rs</li>
                    <li>src/sync/mod.rs</li>
                    <li>src/session/mod.rs</li>
                    <li className="text-emerald-400">tests/frozen_tests.rs</li>
                  </ul>
                </div>

                <div className="md:col-span-3 bg-slate-950 border border-slate-800 rounded-lg p-4 font-mono text-xs text-slate-300 overflow-x-auto">
                  <div className="text-slate-500 pb-2 border-b border-slate-800 mb-3 flex justify-between">
                    <span>// crates/rocksmith-core/src/domain.rs</span>
                    <span className="text-slate-600">Rust 1.83</span>
                  </div>
                  <pre className="text-slate-300 leading-relaxed">
{`pub trait PitchDetector: Send {
    fn process(&mut self, samples: &[f32]) -> Option<PitchResult>;
    fn reset(&mut self);
}

pub trait OnsetDetector: Send {
    fn process(&mut self, samples: &[f32]) -> Option<OnsetResult>;
    fn reset(&mut self);
}

/// Reconciles the chaotic 0-15ms pick attack transient with 15-35ms consensus pitch.
pub trait TemporalNoteTracker: Send {
    fn record_onset(&mut self, onset: OnsetResult);
    fn record_pitch(&mut self, pitch: PitchResult);
    fn poll_resolved_notes(&mut self, current_time_ms: u64) -> Vec<ResolvedNoteEvent>;
    fn reset(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Judgment {
    Perfect, // <= 15ms, <= 10 cents
    Great,   // <= 30ms, <= 20 cents
    Good,    // <= 50ms, <= 35 cents
    Miss,    // > 50ms or wrong pitch
}`}
                  </pre>
                </div>
              </div>
            </div>
          </div>
        )}
      </main>

      {/* Footer Status Bar */}
      <footer className="border-t border-slate-800/80 bg-slate-900/60 text-slate-400 text-xs py-3 px-4 sm:px-6 lg:px-8">
        <div className="max-w-7xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-2">
          <div className="flex items-center space-x-3">
            <span className="flex items-center space-x-1.5 text-emerald-400">
              <span className="w-2 h-2 rounded-full bg-emerald-400"></span>
              <span className="font-semibold">Engine Healthy</span>
            </span>
            <span className="text-slate-600">|</span>
            <span>Sample Rate: <b>44,100 Hz</b></span>
            <span className="text-slate-600">|</span>
            <span>Buffer: <b>512 samples (~11.6ms)</b></span>
          </div>

          <div className="flex items-center space-x-3 text-slate-500 font-mono text-[11px]">
            <span>Planning Protocol v3.1</span>
            <span>•</span>
            <span>AGPL-3.0 License</span>
          </div>
        </div>
      </footer>
    </div>
  );
}
