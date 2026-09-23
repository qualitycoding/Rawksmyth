#pragma once

#include "MathUtils.hpp"
#include <vector>
#include <cmath>
#include <cstdint>

namespace sitar::dsp {

class DelayLine {
public:
    DelayLine() = default;

    void prepare(size_t maxDelaySamples) {
        // Allocate next power of two for efficient bitwise wrapping
        size_t size = 1;
        while (size < maxDelaySamples + 16) {
            size <<= 1;
        }
        buffer_.assign(size, 0.0f);
        mask_ = size - 1;
        writeIndex_ = 0;
        allpassState_ = 0.0f;
    }

    void reset() noexcept {
        std::fill(buffer_.begin(), buffer_.end(), 0.0f);
        writeIndex_ = 0;
        allpassState_ = 0.0f;
    }

    void push(float sample) noexcept {
        buffer_[writeIndex_] = sample;
        writeIndex_ = (writeIndex_ + 1) & mask_;
    }

    // Read with cubic Hermite fractional interpolation
    [[nodiscard]] float readHermite(float delaySamples) const noexcept {
        const float readPos = static_cast<float>(writeIndex_) - delaySamples;
        const int32_t i0 = static_cast<int32_t>(std::floor(readPos));
        const float frac = readPos - static_cast<float>(i0);

        const float ym1 = buffer_[(i0 - 1) & mask_];
        const float y0  = buffer_[i0 & mask_];
        const float y1  = buffer_[(i0 + 1) & mask_];
        const float y2  = buffer_[(i0 + 2) & mask_];

        return interpolateHermite(ym1, y0, y1, y2, frac);
    }

    // Read with first-order allpass interpolation (unitary magnitude, zero lowpass attenuation)
    // delaySamples must be >= 1.0
    [[nodiscard]] float readAllpass(float delaySamples) noexcept {
        const float totalDelay = std::max(1.0f, delaySamples);
        const float intPart = std::floor(totalDelay);
        float frac = totalDelay - intPart;

        // Condition frac into [0.5, 1.5] for optimal allpass stability and phase linearity
        int32_t baseDelay = static_cast<int32_t>(intPart);
        if (frac < 0.5f) {
            frac += 1.0f;
            baseDelay -= 1;
        }

        const float eta = (1.0f - frac) / (1.0f + frac);
        const int32_t idx = (writeIndex_ - baseDelay) & mask_;
        const float x = buffer_[idx];

        // Allpass difference equation: y[n] = eta * x[n] + x[n-1] - eta * y[n-1]
        const float y = eta * x + allpassState_;
        allpassState_ = x - eta * y;
        return y;
    }

    [[nodiscard]] size_t getCapacity() const noexcept {
        return buffer_.size();
    }

private:
    std::vector<float> buffer_{};
    size_t mask_{0};
    size_t writeIndex_{0};
    float allpassState_{0.0f};
};

} // namespace sitar::dsp
