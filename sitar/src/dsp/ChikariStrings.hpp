#pragma once

#include "KarplusStrongString.hpp"
#include <array>

namespace sitar::dsp {

/**
 * High-pitched rhythm and drone strings (Chikari).
 * Typically 3 strings tuned to tonic (Sa) and dominant (Pa) over high octaves.
 */
class ChikariStrings {
public:
    static constexpr size_t NUM_CHIKARI = 3;

    ChikariStrings() = default;

    void prepare(float sampleRate, float tonicHz = 220.0f) {
        sampleRate_ = sampleRate;
        tonicHz_ = tonicHz;

        // Chikari tunings:
        // 0: Sa (Tonic, 1 octave above fundamental = 440 Hz for A3)
        // 1: Pa (Fifth above = 660 Hz)
        // 2: Tar Sa (Higher octave = 880 Hz)
        const std::array<float, NUM_CHIKARI> ratios = { 2.0f, 3.0f, 4.0f };

        for (size_t i = 0; i < NUM_CHIKARI; ++i) {
            strings_[i].prepare(sampleRate, 100.0f);
            strings_[i].setFrequency(tonicHz_ * ratios[i]);
            strings_[i].setDamping(0.2f);
            strings_[i].setDecay(1.8f);
            strings_[i].setJawari(0.5f, 0.4f);
        }
    }

    void reset() noexcept {
        for (auto& s : strings_) {
            s.reset();
        }
    }

    void triggerChikari(size_t index, float velocity) {
        if (index < NUM_CHIKARI) {
            strings_[index].pluck(velocity, 0.85f);
        }
    }

    // Trigger full Chikari strum (Jhala stroke)
    void strumJhala(float velocity) {
        for (size_t i = 0; i < NUM_CHIKARI; ++i) {
            strings_[i].pluck(velocity * (0.8f + 0.1f * static_cast<float>(i)), 0.9f);
        }
    }

    [[nodiscard]] float process() noexcept {
        float out = 0.0f;
        for (auto& s : strings_) {
            out += s.processSample();
        }
        return out * 0.5f;
    }

private:
    float sampleRate_{44100.0f};
    float tonicHz_{220.0f};
    std::array<KarplusStrongString, NUM_CHIKARI> strings_{};
};

} // namespace sitar::dsp
