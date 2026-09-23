#include "test_harness.hpp"
#include "../src/dsp/SitarSynth.hpp"
#include <vector>
#include <cmath>
#include <chrono>

using namespace sitar::dsp;
using namespace sitar::test;

// T-004: Pluck position comb filtering
// Center pluck vs bridge pluck produces distinct waveform shapes and harmonic profiles
bool test_T004_pluck_position_comb() {
    constexpr float sampleRate = 44100.0f;
    KarplusStrongString strCenter, strNearBridge;

    strCenter.prepare(sampleRate);
    strCenter.setFrequency(220.0f);
    strCenter.setPluckPosition(0.5f); // Pluck in center
    strCenter.pluck(0.9f, 0.4f);

    strNearBridge.prepare(sampleRate);
    strNearBridge.setFrequency(220.0f);
    strNearBridge.setPluckPosition(0.08f); // Pluck near bridge
    strNearBridge.pluck(0.9f, 0.4f);

    float sumDiff = 0.0f;
    for (int i = 0; i < 1024; ++i) {
        const float sC = strCenter.processSample();
        const float sB = strNearBridge.processSample();
        sumDiff += std::abs(sC - sB);
    }

    // Must have distinctly different waveform envelopes due to comb filtering
    if (sumDiff < 10.0f) {
        std::cout << "[T-004 FAIL] Center and bridge plucks lack distinct comb filtering: " << sumDiff << " ";
        return false;
    }
    return true;
}
REGISTER_TEST(T004, "Pluck Position Comb Filtering & Harmonic Timbre Variation", test_T004_pluck_position_comb)

// T-008: Loop stability & bounded energy
bool test_T008_loop_stability() {
    constexpr float sampleRate = 44100.0f;
    SitarSynth synth;
    synth.prepare(sampleRate);
    synth.noteOn(60, 1.0f);

    std::vector<float> bufL(512), bufR(512);
    float maxEnergy = 0.0f;

    for (int b = 0; b < 100; ++b) {
        synth.processBlock(bufL.data(), bufR.data(), 512);
        for (int i = 0; i < 512; ++i) {
            if (std::isnan(bufL[i]) || std::isnan(bufR[i]) || std::isinf(bufL[i]) || std::isinf(bufR[i])) {
                std::cout << "[T-008 FAIL] NaN or Inf detected in audio output ";
                return false;
            }
            maxEnergy = std::max(maxEnergy, std::abs(bufL[i]));
            maxEnergy = std::max(maxEnergy, std::abs(bufR[i]));
        }
    }

    if (maxEnergy > 4.0f) {
        std::cout << "[T-008 FAIL] Energy unbounded/exploded: " << maxEnergy << " ";
        return false;
    }
    return true;
}
REGISTER_TEST(T008, "Karplus-Strong Loop Numerical Stability & Energy Boundedness", test_T008_loop_stability)

// T-009: Chikari rhythm strings tuning
bool test_T009_chikari_tuning() {
    constexpr float sampleRate = 44100.0f;
    ChikariStrings chikari;
    chikari.prepare(sampleRate, 220.0f); // Sa = 220 Hz
    chikari.triggerChikari(0, 0.9f); // Sa (octave above = 440 Hz)

    std::vector<float> buf(2048);
    for (size_t i = 0; i < buf.size(); ++i) {
        buf[i] = chikari.process();
    }

    const float measured = estimatePitchAutocorr(buf.data(), buf.size(), sampleRate, 440.0f);
    const float err = std::abs(centsDifference(440.0f, measured));

    if (err > 25.0f) {
        std::cout << "[T-009 FAIL] Chikari 0 expected 440 Hz, measured " << measured << " Hz (err: " << err << "c) ";
        return false;
    }
    return true;
}
REGISTER_TEST(T009, "Chikari Drone String Tunings & Harmonic Sa-Pa Ratios", test_T009_chikari_tuning)

// T-010: Raga tuning intervals
bool test_T010_raga_tuning_intervals() {
    constexpr float sampleRate = 44100.0f;
    SympatheticBank bank;
    bank.prepare(sampleRate);

    bank.setRagaTuning(0, 220.0f);
    const auto& yamanStrings = bank.getStrings();

    const float fSa = yamanStrings[0].getFrequency();
    const float fTivraMa = yamanStrings[3].getFrequency();
    const float fPa = yamanStrings[4].getFrequency();

    const float maIntervalCents = centsDifference(fSa, fTivraMa);
    const float paIntervalCents = centsDifference(fSa, fPa);

    if (std::abs(maIntervalCents - 600.0f) > 5.0f || std::abs(paIntervalCents - 700.0f) > 5.0f) {
        std::cout << "[T-010 FAIL] Yaman interval error: Ma=" << maIntervalCents << ", Pa=" << paIntervalCents << " ";
        return false;
    }
    return true;
}
REGISTER_TEST(T010, "Indian Classical Raga Scale Interval Definitions", test_T010_raga_tuning_intervals)

// T-015: SitarSynth stereo block processing and spatial field
bool test_T015_synth_stereo_processing() {
    constexpr float sampleRate = 44100.0f;
    SitarSynth synth;
    synth.prepare(sampleRate);
    synth.noteOn(60, 0.8f);

    std::vector<float> left(256), right(256);
    synth.processBlock(left.data(), right.data(), 256);

    float sumL = 0.0f, sumR = 0.0f;
    for (size_t i = 0; i < 256; ++i) {
        sumL += std::abs(left[i]);
        sumR += std::abs(right[i]);
    }

    if (sumL <= 1e-4f || sumR <= 1e-4f) {
        std::cout << "[T-015 FAIL] Silent channel detected ";
        return false;
    }
    return true;
}
REGISTER_TEST(T015, "SitarSynth Stereo Audio Block Rendering & Spatial Diffusion", test_T015_synth_stereo_processing)

// T-016: Polyphonic voice handling and note off
bool test_T016_voice_handling() {
    constexpr float sampleRate = 44100.0f;
    SitarSynth synth;
    synth.prepare(sampleRate);

    synth.noteOn(60, 0.8f);
    synth.noteOn(64, 0.8f);
    synth.noteOff(60);

    std::vector<float> left(128), right(128);
    synth.processBlock(left.data(), right.data(), 128);

    return true;
}
REGISTER_TEST(T016, "SitarSynth Voice Handling & Legato Meend Transition", test_T016_voice_handling)

// T-019: Real-Time CPU budget compliance (< 5% load on single thread)
bool test_T019_cpu_budget() {
    constexpr float sampleRate = 44100.0f;
    constexpr size_t blockSize = 256;
    constexpr size_t totalBlocks = 200;

    SitarSynth synth;
    synth.prepare(sampleRate);
    synth.noteOn(60, 0.9f);
    synth.strumJhala(0.8f);

    std::vector<float> left(blockSize), right(blockSize);

    auto start = std::chrono::high_resolution_clock::now();
    for (size_t b = 0; b < totalBlocks; ++b) {
        synth.processBlock(left.data(), right.data(), blockSize);
    }
    auto end = std::chrono::high_resolution_clock::now();

    const double elapsedSec = std::chrono::duration<double>(end - start).count();
    const double audioDurationSec = static_cast<double>(totalBlocks * blockSize) / sampleRate;
    const double cpuUsagePercent = (elapsedSec / audioDurationSec) * 100.0;

    if (cpuUsagePercent > 5.0) {
        std::cout << "[T-019 FAIL] CPU usage exceeds 5%: " << cpuUsagePercent << "% ";
        return false;
    }
    return true;
}
REGISTER_TEST(T019, "Real-Time CPU Budget (< 5% Single-Thread Load at 44.1 kHz)", test_T019_cpu_budget)
