#pragma once

#include "MathUtils.hpp"
#include <vector>
#include <cmath>
#include <array>

namespace sitar::dsp {

/**
 * 2nd-order resonator model of a single sympathetic (Tarab) string.
 * Resonant frequency f_r with narrow bandwidth and high Q, excited
 * by bridge vibration coupling.
 */
class TarabString {
public:
    TarabString() = default;

    void setFrequency(float freqHz, float sampleRate, float q = 350.0f) noexcept {
        freq_ = freqHz;
        sampleRate_ = sampleRate;
        q_ = q;

        const float omega = TWO_PI * freq_ / sampleRate_;
        const float alpha = std::sin(omega) / (2.0f * q_);

        // Bandpass resonator coefficients (normalized peak gain = 1.0)
        const float a0 = 1.0f + alpha;
        b0_ = alpha / a0;
        b1_ = 0.0f;
        b2_ = -alpha / a0;
        a1_ = -2.0f * std::cos(omega) / a0;
        a2_ = (1.0f - alpha) / a0;

        reset();
    }

    void reset() noexcept {
        x1_ = 0.0f;
        x2_ = 0.0f;
        y1_ = 0.0f;
        y2_ = 0.0f;
    }

    [[nodiscard]] float process(float bridgeSignal) noexcept {
        const float y = b0_ * bridgeSignal + b1_ * x1_ + b2_ * x2_ - a1_ * y1_ - a2_ * y2_;
        x2_ = x1_;
        x1_ = bridgeSignal;
        y2_ = y1_;
        y1_ = y;
        return y;
    }

    [[nodiscard]] float getFrequency() const noexcept { return freq_; }

private:
    float freq_{440.0f};
    float sampleRate_{44100.0f};
    float q_{350.0f};
    float b0_{0.0f}, b1_{0.0f}, b2_{0.0f}, a1_{0.0f}, a2_{0.0f};
    float x1_{0.0f}, x2_{0.0f}, y1_{0.0f}, y2_{0.0f};
};

/**
 * Bank of 11 to 13 Sympathetic (Tarab) Strings.
 */
class SympatheticBank {
public:
    static constexpr size_t NUM_TARAB = 11;

    SympatheticBank() = default;

    void prepare(float sampleRate) {
        sampleRate_ = sampleRate;
        setRagaTuning(0, 220.0f); // Default Sa = A3 (220 Hz)
    }

    void reset() noexcept {
        for (auto& s : strings_) {
            s.reset();
        }
    }

    void setCouplingAmount(float coupling) noexcept {
        // coupling in [0.0, 1.0]
        coupling_ = clamp(coupling, 0.0f, 1.0f);
    }

    void setResonanceQ(float q) noexcept {
        q_ = clamp(q, 50.0f, 800.0f);
        updateTunings();
    }

    /**
     * Tune the 11 Tarab strings to a traditional Raga scale over 2 octaves.
     * ragaIndex: 0 = Yaman, 1 = Bhairav, 2 = Kafi, 3 = Darbari, 4 = Bilawal
     */
    void setRagaTuning(int ragaIndex, float tonicHz) noexcept {
        tonicHz_ = tonicHz;
        ragaIndex_ = ragaIndex;

        // Semitone intervals relative to tonic Sa for Indian Ragas
        // Yaman: Sa, Re (natural), Ga (shuddha), Ma (tivra/sharp), Pa, Dha, Ni
        // Bhairav: Sa, re (komal/flat), Ga, ma (shuddha), Pa, dha (komal), Ni
        // Kafi: Sa, Re, ga (komal), ma, Pa, Dha, ni (komal)
        // Darbari: Sa, Re, ga (komal), ma, Pa, dha (komal), ni (komal)
        const std::array<std::array<float, NUM_TARAB>, 5> ragaScales = {{
            // Yaman (Kalyan thaat): Sa, Re, Ga, tivra-Ma, Pa, Dha, Ni, Sa', Re', Ga', tivra-Ma'
            { 0.0f, 2.0f, 4.0f, 6.0f, 7.0f, 9.0f, 11.0f, 12.0f, 14.0f, 16.0f, 18.0f },
            // Bhairav thaat: Sa, komal-re, Ga, ma, Pa, komal-dha, Ni, Sa', komal-re', Ga', ma'
            { 0.0f, 1.0f, 4.0f, 5.0f, 7.0f, 8.0f, 11.0f, 12.0f, 13.0f, 16.0f, 17.0f },
            // Kafi thaat: Sa, Re, komal-ga, ma, Pa, Dha, komal-ni, Sa', Re', komal-ga', ma'
            { 0.0f, 2.0f, 3.0f, 5.0f, 7.0f, 9.0f, 10.0f, 12.0f, 14.0f, 15.0f, 17.0f },
            // Darbari Kanada (Asavari): Sa, Re, komal-ga, ma, Pa, komal-dha, komal-ni, Sa', Re', komal-ga', ma'
            { 0.0f, 2.0f, 3.0f, 5.0f, 7.0f, 8.0f, 10.0f, 12.0f, 14.0f, 15.0f, 17.0f },
            // Bilawal thaat (Natural major): Sa, Re, Ga, ma, Pa, Dha, Ni, Sa', Re', Ga', ma'
            { 0.0f, 2.0f, 4.0f, 5.0f, 7.0f, 9.0f, 11.0f, 12.0f, 14.0f, 16.0f, 17.0f }
        }};

        const size_t idx = static_cast<size_t>(std::clamp(ragaIndex, 0, 4));
        intervals_ = ragaScales[idx];
        updateTunings();
    }

    [[nodiscard]] float process(float bridgeSignal) noexcept {
        if (coupling_ <= 0.001f) return 0.0f;

        float sum = 0.0f;
        const float excitedSignal = bridgeSignal * coupling_;
        for (auto& s : strings_) {
            sum += s.process(excitedSignal);
        }
        return sum * 0.45f;
    }

    [[nodiscard]] const std::array<TarabString, NUM_TARAB>& getStrings() const noexcept {
        return strings_;
    }

private:
    void updateTunings() noexcept {
        for (size_t i = 0; i < NUM_TARAB; ++i) {
            const float freq = tonicHz_ * std::pow(2.0f, intervals_[i] / 12.0f);
            strings_[i].setFrequency(freq, sampleRate_, q_);
        }
    }

    float sampleRate_{44100.0f};
    float tonicHz_{220.0f};
    int ragaIndex_{0};
    float coupling_{0.45f};
    float q_{350.0f};
    std::array<float, NUM_TARAB> intervals_{};
    std::array<TarabString, NUM_TARAB> strings_{};
};

} // namespace sitar::dsp
