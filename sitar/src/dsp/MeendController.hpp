#pragma once

#include "MathUtils.hpp"
#include <cmath>
#include <algorithm>

namespace sitar::dsp {

/**
 * Meend (Pitch Bend / Lateral String Pull) Controller.
 * Slew-rate smoothed microtonal deflection modeling the physical deflection
 * of a sitar string across high curved frets.
 */
class MeendController {
public:
    MeendController() = default;

    void prepare(float sampleRate) noexcept {
        sampleRate_ = sampleRate;
        currentBendSemitones_ = 0.0f;
        targetBendSemitones_ = 0.0f;
        setGlideTime(0.08f); // 80 ms default glide
    }

    void reset() noexcept {
        currentBendSemitones_ = 0.0f;
        targetBendSemitones_ = 0.0f;
    }

    void setGlideTime(float timeSeconds) noexcept {
        glideTimeSec_ = clamp(timeSeconds, 0.001f, 1.5f);
        if (sampleRate_ > 0.0f) {
            // Slew smoothing coefficient
            slewRate_ = 1.0f - std::exp(-1.0f / (glideTimeSec_ * sampleRate_));
        }
    }

    void setBendSemitones(float semitones) noexcept {
        // Indian sitar meend allows pulling up to +5 or +7 semitones
        targetBendSemitones_ = clamp(semitones, -12.0f, 7.0f);
    }

    // Process single audio sample step, returns current frequency multiplier
    [[nodiscard]] float process() noexcept {
        currentBendSemitones_ += (targetBendSemitones_ - currentBendSemitones_) * slewRate_;
        return std::pow(2.0f, currentBendSemitones_ / 12.0f);
    }

    [[nodiscard]] float getCurrentBendSemitones() const noexcept {
        return currentBendSemitones_;
    }

    [[nodiscard]] float getTargetBendSemitones() const noexcept {
        return targetBendSemitones_;
    }

private:
    float sampleRate_{44100.0f};
    float currentBendSemitones_{0.0f};
    float targetBendSemitones_{0.0f};
    float glideTimeSec_{0.08f};
    float slewRate_{0.01f};
};

} // namespace sitar::dsp
