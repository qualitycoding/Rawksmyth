#pragma once

#include "MathUtils.hpp"
#include <algorithm>

namespace sitar::dsp {

class OnePoleLowpass {
public:
    OnePoleLowpass() = default;

    void setCutoff(float cutoffHz, float sampleRate) noexcept {
        if (sampleRate <= 0.0f) return;
        const float normalizedFreq = clamp(cutoffHz / sampleRate, 0.0001f, 0.499f);
        // Standard bilinear / matched impulse invariant coefficient
        const float costh = std::cos(TWO_PI * normalizedFreq);
        b1_ = 2.0f - costh - std::sqrt((2.0f - costh) * (2.0f - costh) - 1.0f);
        a0_ = 1.0f - b1_;
    }

    void setCoefficient(float b1) noexcept {
        b1_ = clamp(b1, 0.0f, 0.9999f);
        a0_ = 1.0f - b1_;
    }

    void reset() noexcept {
        z1_ = 0.0f;
    }

    [[nodiscard]] float process(float in) noexcept {
        z1_ = in * a0_ + z1_ * b1_;
        return z1_;
    }

    [[nodiscard]] float getPhaseDelayAtFundamental(float f0, float sampleRate) const noexcept {
        if (sampleRate <= 0.0f || f0 <= 0.0f) return 0.0f;
        const float omega = TWO_PI * f0 / sampleRate;
        // H(z) = a0 / (1 - b1 * z^-1)
        // Phase response phi(w) = -atan(b1 * sin(w) / (1 - b1 * cos(w)))
        const float num = b1_ * std::sin(omega);
        const float den = 1.0f - b1_ * std::cos(omega);
        const float phase = -std::atan2(num, den);
        // Phase delay in samples = -phase / omega
        return -phase / omega;
    }

private:
    float a0_{1.0f};
    float b1_{0.0f};
    float z1_{0.0f};
};

} // namespace sitar::dsp
