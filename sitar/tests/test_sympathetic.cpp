#include "test_harness.hpp"
#include "../src/dsp/SympatheticResonator.hpp"
#include "../src/dsp/KarplusStrongString.hpp"
#include <vector>
#include <cmath>

using namespace sitar::dsp;
using namespace sitar::test;

// T-003: Sympathetic resonance excitation
bool test_T003_sympathetic_excitation() {
    constexpr float sampleRate = 44100.0f;
    SympatheticBank bank;
    bank.prepare(sampleRate);
    bank.setCouplingAmount(0.8f);
    bank.setRagaTuning(0, 220.0f); // Sa = 220 Hz (A3)

    // Excite with matching sine wave at 220 Hz for 2000 samples
    for (int i = 0; i < 2000; ++i) {
        const float t = static_cast<float>(i) / sampleRate;
        const float excitation = std::sin(TWO_PI * 220.0f * t) * 0.5f;
        bank.process(excitation);
    }

    // Now input is zero; measure ringing energy from sympathetic string bank
    float ringingEnergy = 0.0f;
    for (int i = 0; i < 2000; ++i) {
        const float s = bank.process(0.0f);
        ringingEnergy += s * s;
    }

    if (ringingEnergy < 0.01f) {
        std::cout << "[T-003 FAIL] Insufficient sympathetic ringing: " << ringingEnergy << " ";
        return false;
    }
    return true;
}
REGISTER_TEST(T003, "Tarab Sympathetic String Resonant Ringing upon Harmonic Excitation", test_T003_sympathetic_excitation)

// T-007: Tarab resonance Q factor bandwidth & decay length
bool test_T007_sympathetic_q_decay() {
    constexpr float sampleRate = 44100.0f;

    TarabString highQString;
    highQString.setFrequency(440.0f, sampleRate, 600.0f); // High Q

    TarabString lowQString;
    lowQString.setFrequency(440.0f, sampleRate, 80.0f);  // Low Q

    for (int i = 0; i < 50; ++i) {
        const float t = static_cast<float>(i) / sampleRate;
        const float x = std::sin(TWO_PI * 440.0f * t);
        highQString.process(x);
        lowQString.process(x);
    }

    float highEarly = 0.0f, highLate = 0.0f;
    float lowEarly  = 0.0f, lowLate  = 0.0f;

    for (int i = 0; i < 1500; ++i) {
        const float sHigh = highQString.process(0.0f);
        const float sLow  = lowQString.process(0.0f);
        if (i < 200) {
            highEarly += sHigh * sHigh;
            lowEarly  += sLow * sLow;
        } else if (i >= 800 && i < 1200) {
            highLate += sHigh * sHigh;
            lowLate  += sLow * sLow;
        }
    }

    const float highRatio = highLate / std::max(1e-9f, highEarly);
    const float lowRatio  = lowLate  / std::max(1e-9f, lowEarly);

    if (highRatio <= lowRatio * 1.2f) {
        std::cout << "[T-007 FAIL] highRatio=" << highRatio << ", lowRatio=" << lowRatio << " ";
        return false;
    }
    return true;
}
REGISTER_TEST(T007, "Tarab Q Factor Ringing Decay Scaling", test_T007_sympathetic_q_decay)

// T-014: Tarab coupling isolation (zero bleed when coupling is 0)
bool test_T014_sympathetic_isolation() {
    constexpr float sampleRate = 44100.0f;
    SympatheticBank bank;
    bank.prepare(sampleRate);
    bank.setCouplingAmount(0.0f); // Off

    float outputSum = 0.0f;
    for (int i = 0; i < 1000; ++i) {
        outputSum += std::abs(bank.process(0.8f));
    }

    if (outputSum > 1e-6f) {
        std::cout << "[T-014 FAIL] Non-zero output with coupling=0: " << outputSum << " ";
        return false;
    }
    return true;
}
REGISTER_TEST(T014, "Tarab Coupling Complete Isolation at Zero", test_T014_sympathetic_isolation)
