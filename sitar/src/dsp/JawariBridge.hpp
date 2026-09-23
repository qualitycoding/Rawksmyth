#pragma once

#include "MathUtils.hpp"
#include <cmath>

namespace sitar::dsp {

class JawariBridge {
public:
    JawariBridge() = default;

    void reset() noexcept {
        previousSample_ = 0.0f;
    }

    void setBuzzAmount(float amount) noexcept {
        buzzAmount_ = clamp(amount, 0.0f, 1.0f);
    }

    void setBridgeCurvature(float curve) noexcept {
        curvature_ = clamp(curve, 0.1f, 1.0f);
    }

    void setThreadPosition(float pos) noexcept {
        threadPosition_ = clamp(pos, 0.0f, 1.0f);
    }

    [[nodiscard]] float process(float stringDisplacement) noexcept {
        if (buzzAmount_ <= 0.001f) {
            previousSample_ = stringDisplacement;
            return stringDisplacement;
        }

        const float x = stringDisplacement;
        const float threshold = 0.02f * (1.0f - threadPosition_ * 0.5f);
        float output = x;

        // Bridge boundary condition: string rests on wide, curved bone surface (Jawari).
        // For downward excursion (x < threshold), string makes rolling contact with the curved bridge.
        if (x < threshold) {
            const float penetration = threshold - x;
            // Soft compressive contact force with parabolic curvature profile
            const float contactDamping = 1.0f / (1.0f + 2.5f * buzzAmount_ * penetration * curvature_);
            output = threshold - penetration * contactDamping 
                   + 0.12f * buzzAmount_ * std::sin(TWO_PI * penetration * 4.0f);
        }

        // Contact impulse / dynamic friction
        const float contactDiff = output - previousSample_;
        output += 0.05f * buzzAmount_ * std::abs(x) * contactDiff;
        previousSample_ = output;

        return output;
    }

private:
    float buzzAmount_{0.65f};
    float curvature_{0.5f};
    float threadPosition_{0.5f};
    float previousSample_{0.0f};
};

} // namespace sitar::dsp
