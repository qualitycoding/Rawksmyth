#pragma once

#include <cmath>
#include <numbers>
#include <algorithm>
#include <cstdint>

namespace sitar::dsp {

constexpr float PI = std::numbers::pi_v<float>;
constexpr float TWO_PI = 2.0f * PI;

inline float midiToFreq(float midiNote, float a4Freq = 440.0f) noexcept {
    return a4Freq * std::pow(2.0f, (midiNote - 69.0f) / 12.0f);
}

inline float freqToMidi(float freq, float a4Freq = 440.0f) noexcept {
    if (freq <= 0.0f) return 0.0f;
    return 69.0f + 12.0f * std::log2(freq / a4Freq);
}

inline float centsDifference(float f1, float f2) noexcept {
    if (f1 <= 0.0f || f2 <= 0.0f) return 0.0f;
    return 1200.0f * std::log2(f2 / f1);
}

inline float clamp(float value, float minVal, float maxVal) noexcept {
    return std::max(minVal, std::min(value, maxVal));
}

inline float softClip(float x) noexcept {
    if (x > 3.0f) return 1.0f;
    if (x < -3.0f) return -1.0f;
    return x * (27.0f + x * x) / (27.0f + 9.0f * x * x);
}

// Hermite cubic interpolation for fractional delay lines
inline float interpolateHermite(float ym1, float y0, float y1, float y2, float frac) noexcept {
    const float c0 = y0;
    const float c1 = 0.5f * (y1 - ym1);
    const float c2 = ym1 - 2.5f * y0 + 2.0f * y1 - 0.5f * y2;
    const float c3 = 0.5f * (y2 - ym1) + 1.5f * (y0 - y1);
    return ((c3 * frac + c2) * frac + c1) * frac + c0;
}

// First order allpass fractional filter parameter calculation
// D is fractional delay in [0.5, 1.5]
inline float allpassCoeff(float fracDelay) noexcept {
    return (1.0f - fracDelay) / (1.0f + fracDelay);
}

} // namespace sitar::dsp
