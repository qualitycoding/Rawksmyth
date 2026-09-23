#include "test_harness.hpp"
#include "../src/dsp/MeendController.hpp"
#include <vector>
#include <cmath>

using namespace sitar::dsp;
using namespace sitar::test;

// T-005: Meend pitch glide continuity & monotonicity
// Verifies pitch deflection glides smoothly without discontinuities or zipper noise
bool test_T005_meend_continuity() {
    constexpr float sampleRate = 44100.0f;
    MeendController meend;
    meend.prepare(sampleRate);
    meend.setGlideTime(0.04f); // 40 ms time constant
    meend.setBendSemitones(4.0f); // Pull string 4 semitones

    const size_t numSteps = 10000; // ~226 ms (over 5 time constants)
    float prevMultiplier = 1.0f;

    for (size_t i = 0; i < numSteps; ++i) {
        const float mult = meend.process();

        // Monotonic increase during positive bend pull
        if (mult < prevMultiplier - 1e-6f) {
            std::cout << "[T-005 FAIL] Non-monotonic glide at step " << i << " ";
            return false;
        }

        // Check continuity (no zipper step > 0.005)
        if (std::abs(mult - prevMultiplier) > 0.005f) {
            std::cout << "[T-005 FAIL] Discontinuous step detected: " << (mult - prevMultiplier) << " ";
            return false;
        }

        prevMultiplier = mult;
    }

    // Expected multiplier for +4 semitones is 2^(4/12) ~= 1.25992
    const float expectedMult = std::pow(2.0f, 4.0f / 12.0f);
    if (std::abs(prevMultiplier - expectedMult) > 0.01f) {
        std::cout << "[T-005 FAIL] Did not reach target: got " << prevMultiplier 
                  << ", expected " << expectedMult << " ";
        return false;
    }

    return true;
}
REGISTER_TEST(T005, "Meend Continuous Pitch Deflection Monotonicity & Anti-Zipper", test_T005_meend_continuity)

// T-017: Meend slew rate ordering
// Fast meend reaches 90% of target significantly faster than slow meend
bool test_T017_meend_rate_ordering() {
    constexpr float sampleRate = 44100.0f;

    MeendController fastMeend, slowMeend;
    fastMeend.prepare(sampleRate);
    fastMeend.setGlideTime(0.02f); // 20 ms fast meend
    fastMeend.setBendSemitones(3.0f);

    slowMeend.prepare(sampleRate);
    slowMeend.setGlideTime(0.15f); // 150 ms slow meend
    slowMeend.setBendSemitones(3.0f);

    size_t fastStep = 0, slowStep = 0;
    const float targetThreshold = 2.7f; // 90% of 3 semitones

    for (size_t i = 0; i < 40000; ++i) {
        fastMeend.process();
        slowMeend.process();

        if (fastStep == 0 && fastMeend.getCurrentBendSemitones() >= targetThreshold) {
            fastStep = i;
        }
        if (slowStep == 0 && slowMeend.getCurrentBendSemitones() >= targetThreshold) {
            slowStep = i;
        }
        if (fastStep > 0 && slowStep > 0) break;
    }

    if (fastStep >= slowStep || fastStep == 0 || slowStep == 0) {
        std::cout << "[T-017 FAIL] fastStep=" << fastStep << ", slowStep=" << slowStep << " ";
        return false;
    }
    return true;
}
REGISTER_TEST(T017, "Meend Slew Rate Ordering (Fast vs Slow Portamento Glide Times)", test_T017_meend_rate_ordering)
