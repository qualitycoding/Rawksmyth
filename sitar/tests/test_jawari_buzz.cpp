#include "test_harness.hpp"
#include "../src/dsp/KarplusStrongString.hpp"
#include "../src/dsp/JawariBridge.hpp"
#include <vector>
#include <cmath>

using namespace sitar::dsp;
using namespace sitar::test;

// T-002: Jawari buzz harmonic generation
// Inputting a pure fundamental sine wave through JawariBridge generates rich upper harmonics
bool test_T002_jawari_buzz_harmonics() {
    JawariBridge bridge;
    bridge.setBuzzAmount(0.85f);
    bridge.setBridgeCurvature(0.6f);

    constexpr float sampleRate = 44100.0f;
    constexpr float f0 = 220.0f;
    constexpr size_t N = 2048;

    float outputHarmonicEnergy = 0.0f;

    // Run pure sine wave through bridge
    for (size_t i = 0; i < N; ++i) {
        const float t = static_cast<float>(i) / sampleRate;
        const float pureSine = 0.5f * std::sin(TWO_PI * f0 * t);
        const float buzzyOut = bridge.process(pureSine);

        // Difference between output and input represents non-linear generated harmonics
        const float generatedHarmonics = buzzyOut - pureSine;
        outputHarmonicEnergy += generatedHarmonics * generatedHarmonics;
    }

    if (outputHarmonicEnergy < 0.1f) {
        std::cout << "[T-002 FAIL] Insufficient harmonic distortion energy: " << outputHarmonicEnergy << " ";
        return false;
    }
    return true;
}
REGISTER_TEST(T002, "Jawari Buzz Harmonic Generation (High-Frequency Enrichment)", test_T002_jawari_buzz_harmonics)

// T-006: Bridge curvature non-linearity & asymmetric displacement
bool test_T006_bridge_curvature() {
    JawariBridge bridge;
    bridge.setBuzzAmount(0.8f);
    bridge.setBridgeCurvature(0.7f);

    const float posIn = 0.5f;
    const float negIn = -0.5f;

    const float posOut = bridge.process(posIn);
    bridge.reset();
    const float negOut = bridge.process(negIn);

    const float asymmetry = std::abs(posOut) - std::abs(negOut);
    if (std::abs(asymmetry) < 0.05f) {
        std::cout << "[T-006 FAIL] Asymmetry too low: " << asymmetry << " ";
        return false;
    }
    return true;
}
REGISTER_TEST(T006, "Jawari Bridge Asymmetric Boundary Condition & Curvature", test_T006_bridge_curvature)

// T-013: Mizrab hardness dynamic brightness
bool test_T013_mizrab_hardness() {
    constexpr float sampleRate = 44100.0f;

    KarplusStrongString softString;
    softString.prepare(sampleRate);
    softString.setFrequency(220.0f);
    softString.pluck(0.8f, 0.2f); // Soft mizrab

    KarplusStrongString hardString;
    hardString.prepare(sampleRate);
    hardString.setFrequency(220.0f);
    hardString.pluck(0.8f, 0.95f); // Hard wire mizrab

    float softDiffSum = 0.0f, hardDiffSum = 0.0f;
    float prevS = 0.0f, prevH = 0.0f;

    for (int i = 0; i < 512; ++i) {
        const float s = softString.processSample();
        const float h = hardString.processSample();

        softDiffSum += (s - prevS) * (s - prevS);
        hardDiffSum += (h - prevH) * (h - prevH);

        prevS = s;
        prevH = h;
    }

    if (hardDiffSum <= softDiffSum * 1.05f) {
        std::cout << "[T-013 FAIL] soft=" << softDiffSum << ", hard=" << hardDiffSum << " ";
        return false;
    }
    return true;
}
REGISTER_TEST(T013, "Mizrab Hardness Dynamic Brightness & Spectral Envelope", test_T013_mizrab_hardness)
