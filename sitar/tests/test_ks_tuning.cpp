#include "test_harness.hpp"
#include "../src/dsp/KarplusStrongString.hpp"
#include <vector>
#include <cmath>

using namespace sitar::dsp;
using namespace sitar::test;

// T-001: Pitch tracking & tuning accuracy across standard sitar playable range (MIDI 36 to 72)
// Render >= 1.5 seconds for low notes (per continuation session decision #1)
// Tolerance: +-25 cents (Pre-decided human fallback decision D-020)
bool test_T001_pitch_tuning() {
    constexpr float sampleRate = 44100.0f;
    KarplusStrongString string;
    string.prepare(sampleRate);
    string.setDamping(0.10f);
    string.setDecay(8.0f);
    string.setJawari(0.0f, 0.5f); // Base string calibration without non-linear contact pitch shift

    const std::vector<int> testNotes = { 36, 43, 48, 55, 60, 67, 72 };

    for (int midi : testNotes) {
        const float expectedFreq = midiToFreq(static_cast<float>(midi));
        string.setFrequency(expectedFreq);
        string.pluck(0.9f, 0.7f);

        // Discard initial transient (at least 2000 samples)
        for (int i = 0; i < 2000; ++i) string.processSample();

        // Render buffer: 16384 samples for low notes (< 150 Hz) for accurate autocorrelation window
        const size_t bufSize = (expectedFreq < 150.0f) ? 16384 : 4096;
        std::vector<float> buffer(bufSize);
        for (size_t i = 0; i < bufSize; ++i) {
            buffer[i] = string.processSample();
        }

        const float measuredFreq = estimatePitchAutocorr(buffer.data(), buffer.size(), sampleRate, expectedFreq);
        const float errorCents = std::abs(centsDifference(expectedFreq, measuredFreq));

        if (errorCents > 25.0f) {
            std::cout << "\n[T-001 FAIL] MIDI " << midi << ": expected " << expectedFreq 
                      << " Hz, got " << measuredFreq << " Hz (err: " << errorCents << " cents) ";
            return false;
        }
    }
    return true;
}
REGISTER_TEST(T001, "Karplus-Strong Pitch Tuning Accuracy across MIDI 36-72 (within +-25 cents, D-020)", test_T001_pitch_tuning)

// T-011: Sample Rate Invariance (44.1 kHz, 48 kHz, 96 kHz)
bool test_T011_sample_rate_invariance() {
    const std::vector<float> sampleRates = { 44100.0f, 48000.0f, 96000.0f };
    const float testFreq = 220.0f; // A3

    for (float sr : sampleRates) {
        KarplusStrongString string;
        string.prepare(sr);
        string.setDamping(0.2f);
        string.setDecay(4.0f);
        string.setFrequency(testFreq);
        string.pluck(0.9f, 0.8f);

        for (int i = 0; i < static_cast<int>(sr * 0.05f); ++i) string.processSample();

        const size_t bufSize = static_cast<size_t>(sr * 0.15f);
        std::vector<float> buffer(bufSize);
        for (size_t i = 0; i < bufSize; ++i) buffer[i] = string.processSample();

        const float measured = estimatePitchAutocorr(buffer.data(), buffer.size(), sr, testFreq);
        const float errorCents = std::abs(centsDifference(testFreq, measured));

        if (errorCents > 20.0f) {
            std::cout << "\n[T-011 FAIL] SR " << sr << ": err " << errorCents << " cents ";
            return false;
        }
    }
    return true;
}
REGISTER_TEST(T011, "Sample Rate Invariance at 44.1k, 48k, and 96k", test_T011_sample_rate_invariance)
