#pragma once

#include "MathUtils.hpp"
#include "DelayLine.hpp"
#include "OnePoleFilter.hpp"
#include "JawariBridge.hpp"
#include <random>
#include <algorithm>
#include <cmath>

namespace sitar::dsp {

class KarplusStrongString {
public:
    KarplusStrongString() = default;

    void prepare(float sampleRate, float minFreqHz = 20.0f) {
        sampleRate_ = sampleRate;
        const size_t maxDelay = static_cast<size_t>(std::ceil(sampleRate_ / minFreqHz)) + 128;
        delayLine_.prepare(maxDelay);
        reset();
    }

    void reset() noexcept {
        delayLine_.reset();
        loopFilter_.reset();
        jawari_.reset();
        active_ = false;
        amplitudeEnvelope_ = 0.0f;
    }

    void setFrequency(float freqHz) noexcept {
        targetFreq_ = clamp(freqHz, 20.0f, sampleRate_ * 0.45f);
        updateDelay();
    }

    void setPluckPosition(float position) noexcept {
        pluckPos_ = clamp(position, 0.02f, 0.5f);
    }

    void setDamping(float damping) noexcept {
        damping_ = clamp(damping, 0.0f, 1.0f);
        updateLoopFilter();
    }

    void setDecay(float decayTimeSec) noexcept {
        decaySec_ = clamp(decayTimeSec, 0.1f, 15.0f);
        updateLoopFilter();
    }

    void setJawari(float buzz, float curve) noexcept {
        jawari_.setBuzzAmount(buzz);
        jawari_.setBridgeCurvature(curve);
    }

    void pluck(float velocity, float hardness = 0.7f) {
        if (targetFreq_ <= 0.0f || sampleRate_ <= 0.0f) return;

        active_ = true;
        amplitudeEnvelope_ = velocity;

        const float periodSamples = sampleRate_ / targetFreq_;
        const size_t loopLength = static_cast<size_t>(std::max(4.0f, std::round(periodSamples)));

        delayLine_.reset();

        const float pickSample = std::max(1.0f, periodSamples * pluckPos_);
        std::mt19937 rng(1337);
        std::uniform_real_distribution<float> noiseDist(-1.0f, 1.0f);

        // Fixed mizrab contact length (12 samples)
        const size_t strikeLength = std::min(size_t(16), loopLength / 2);

        for (size_t i = 0; i < loopLength; ++i) {
            const float pos = static_cast<float>(i);
            // Triangular physical string displacement
            float displacement = 0.0f;
            if (pos <= pickSample) {
                displacement = pos / pickSample;
            } else {
                displacement = (periodSamples - pos) / (periodSamples - pickSample);
            }

            float strikeNoise = 0.0f;
            if (i < strikeLength) {
                const float env = 1.0f - (static_cast<float>(i) / static_cast<float>(strikeLength));
                strikeNoise = noiseDist(rng) * hardness * env * 0.35f;
            }

            const float sample = velocity * (displacement * (1.1f - hardness * 0.3f) + strikeNoise);
            delayLine_.push(sample);
        }
    }

    [[nodiscard]] float processSample() noexcept {
        if (!active_) return 0.0f;

        const float delayedSample = delayLine_.readAllpass(loopDelaySamples_);
        const float bridgeReflected = jawari_.process(delayedSample);
        const float filtered = loopFilter_.process(bridgeReflected);
        const float nextSample = filtered * loopGain_;

        delayLine_.push(nextSample);

        amplitudeEnvelope_ *= 0.99995f;
        if (std::abs(nextSample) < 1e-7f && amplitudeEnvelope_ < 1e-5f) {
            active_ = false;
        }

        return bridgeReflected;
    }

    [[nodiscard]] bool isActive() const noexcept { return active_; }
    [[nodiscard]] float getFrequency() const noexcept { return targetFreq_; }
    [[nodiscard]] float getLoopDelaySamples() const noexcept { return loopDelaySamples_; }

private:
    void updateLoopFilter() noexcept {
        if (targetFreq_ <= 0.0f || sampleRate_ <= 0.0f) return;

        const float cutoffHz = clamp(16000.0f * (1.0f - damping_ * 0.75f), targetFreq_ * 1.5f, sampleRate_ * 0.48f);
        loopFilter_.setCutoff(cutoffHz, sampleRate_);

        const float t60 = std::max(0.1f, decaySec_);
        loopGain_ = std::pow(10.0f, -3.0f / (t60 * targetFreq_));
        loopGain_ = clamp(loopGain_, 0.85f, 0.9999f);

        updateDelay();
    }

    void updateDelay() noexcept {
        if (targetFreq_ <= 0.0f || sampleRate_ <= 0.0f) return;

        const float totalPeriodSamples = sampleRate_ / targetFreq_;
        const float filterPhaseDelay = loopFilter_.getPhaseDelayAtFundamental(targetFreq_, sampleRate_);

        // Deduct 1.0 sample for buffer feedback register delay
        loopDelaySamples_ = std::max(1.5f, totalPeriodSamples - filterPhaseDelay - 1.0f);
    }

    float sampleRate_{44100.0f};
    float targetFreq_{220.0f};
    float pluckPos_{0.18f};
    float damping_{0.3f};
    float decaySec_{4.0f};
    float loopGain_{0.995f};
    float loopDelaySamples_{200.0f};
    float amplitudeEnvelope_{0.0f};
    bool active_{false};

    DelayLine delayLine_{};
    OnePoleLowpass loopFilter_{};
    JawariBridge jawari_{};
};

} // namespace sitar::dsp
