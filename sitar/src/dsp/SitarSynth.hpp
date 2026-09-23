#pragma once

#include "MathUtils.hpp"
#include "KarplusStrongString.hpp"
#include "SympatheticResonator.hpp"
#include "ChikariStrings.hpp"
#include "MeendController.hpp"
#include "Parameters.hpp"
#include <array>
#include <vector>

namespace sitar::dsp {

class SitarSynth {
public:
    static constexpr size_t MAX_VOICES = 4;

    SitarSynth() = default;

    void prepare(float sampleRate) {
        sampleRate_ = sampleRate;

        for (auto& v : voices_) {
            v.prepare(sampleRate, 40.0f);
        }
        tarabBank_.prepare(sampleRate);
        chikari_.prepare(sampleRate, currentTonic_);
        meend_.prepare(sampleRate);

        applyParameters();
        reset();
    }

    void reset() noexcept {
        for (auto& v : voices_) {
            v.reset();
        }
        tarabBank_.reset();
        chikari_.reset();
        meend_.reset();
        lastMidiNote_ = -1;
    }

    void setSampleRate(float sampleRate) {
        prepare(sampleRate);
    }

    void noteOn(int midiNote, float velocity) {
        if (velocity <= 0.0f) {
            noteOff(midiNote);
            return;
        }

        const float freq = midiToFreq(static_cast<float>(midiNote));
        lastMidiNote_ = midiNote;

        // Indian classical sitar is predominantly monophonic with continuous meend glides on the Baj Tar
        // Voice 0 is the primary Baj Tar string
        auto& voice = voices_[0];
        voice.setFrequency(freq);
        voice.setPluckPosition(paramPluckPos_);
        voice.setDamping(paramDamping_);
        voice.setDecay(paramDecaySec_);
        voice.setJawari(paramJawariBuzz_, paramJawariCurve_);
        voice.pluck(velocity, paramPluckHardness_);
    }

    void noteOff(int midiNote) {
        // Real sitar strings continue ringing freely until damped or new note plucked
        if (midiNote == lastMidiNote_) {
            lastMidiNote_ = -1;
        }
    }

    void setPitchBend(float normalizedBend) {
        // normalizedBend in [-1.0, 1.0]
        const float semitones = normalizedBend * paramMeendRange_;
        meend_.setBendSemitones(semitones);
    }

    void triggerChikari(size_t index, float velocity) {
        chikari_.triggerChikari(index, velocity);
    }

    void strumJhala(float velocity) {
        chikari_.strumJhala(velocity);
    }

    void setParameter(ParamId id, float value) {
        switch (id) {
            case ParamId::TonicFreq:
                currentTonic_ = clamp(value, 110.0f, 440.0f);
                chikari_.prepare(sampleRate_, currentTonic_);
                tarabBank_.setRagaTuning(static_cast<int>(paramRagaScale_), currentTonic_);
                break;
            case ParamId::PluckPosition:
                paramPluckPos_ = clamp(value, 0.05f, 0.45f);
                break;
            case ParamId::PluckHardness:
                paramPluckHardness_ = clamp(value, 0.1f, 1.0f);
                break;
            case ParamId::JawariBuzz:
                paramJawariBuzz_ = clamp(value, 0.0f, 1.0f);
                for (auto& v : voices_) v.setJawari(paramJawariBuzz_, paramJawariCurve_);
                break;
            case ParamId::JawariCurvature:
                paramJawariCurve_ = clamp(value, 0.0f, 1.0f);
                for (auto& v : voices_) v.setJawari(paramJawariBuzz_, paramJawariCurve_);
                break;
            case ParamId::DecayTime:
                paramDecaySec_ = clamp(value, 0.5f, 12.0f);
                for (auto& v : voices_) v.setDecay(paramDecaySec_);
                break;
            case ParamId::Damping:
                paramDamping_ = clamp(value, 0.0f, 1.0f);
                for (auto& v : voices_) v.setDamping(paramDamping_);
                break;
            case ParamId::SympatheticCoupling:
                tarabBank_.setCouplingAmount(value);
                break;
            case ParamId::SympatheticQ:
                tarabBank_.setResonanceQ(value);
                break;
            case ParamId::RagaScale:
                paramRagaScale_ = value;
                tarabBank_.setRagaTuning(static_cast<int>(value), currentTonic_);
                break;
            case ParamId::MeendTime:
                meend_.setGlideTime(value * 0.001f);
                break;
            case ParamId::MeendRange:
                paramMeendRange_ = value;
                break;
            case ParamId::ChikariVolume:
                paramChikariVol_ = clamp(value, 0.0f, 1.0f);
                break;
            case ParamId::MasterGain:
                masterGainLin_ = std::pow(10.0f, value / 20.0f);
                break;
            default:
                break;
        }
    }

    void processBlock(float* outL, float* outR, size_t numSamples) noexcept {
        for (size_t i = 0; i < numSamples; ++i) {
            // Update Meend pitch multiplier
            const float bendMult = meend_.process();
            if (lastMidiNote_ >= 0) {
                const float baseFreq = midiToFreq(static_cast<float>(lastMidiNote_));
                voices_[0].setFrequency(baseFreq * bendMult);
            }

            // Process main string voice
            float mainString = voices_[0].processSample();

            // Process Tarab sympathetic resonance excited by main string
            float tarab = tarabBank_.process(mainString);

            // Process Chikari rhythm strings
            float chikari = chikari_.process() * paramChikariVol_;

            // Combine audio with spatial stereo width
            // Main string center, Tarab wide diffuse stereo, Chikari right-leaning
            const float dryLeft = mainString * 0.7f + chikari * 0.5f;
            const float dryRight = mainString * 0.7f + chikari * 0.8f;
            const float wetLeft = tarab * 0.65f;
            const float wetRight = tarab * 0.85f;

            const float sampleL = (dryLeft + wetLeft) * masterGainLin_;
            const float sampleR = (dryRight + wetRight) * masterGainLin_;

            if (outL) outL[i] = softClip(sampleL);
            if (outR) outR[i] = softClip(sampleR);
        }
    }

    [[nodiscard]] float getSampleRate() const noexcept { return sampleRate_; }
    [[nodiscard]] KarplusStrongString& getVoice(size_t index) noexcept { return voices_[index % MAX_VOICES]; }
    [[nodiscard]] SympatheticBank& getTarabBank() noexcept { return tarabBank_; }
    [[nodiscard]] ChikariStrings& getChikari() noexcept { return chikari_; }
    [[nodiscard]] MeendController& getMeend() noexcept { return meend_; }

private:
    void applyParameters() noexcept {
        for (const auto& p : PARAM_REGISTRY) {
            setParameter(p.id, p.defaultValue);
        }
    }

    float sampleRate_{44100.0f};
    float currentTonic_{220.0f};
    int lastMidiNote_{-1};

    float paramPluckPos_{0.18f};
    float paramPluckHardness_{0.75f};
    float paramJawariBuzz_{0.65f};
    float paramJawariCurve_{0.50f};
    float paramDecaySec_{4.0f};
    float paramDamping_{0.30f};
    float paramRagaScale_{0.0f};
    float paramMeendRange_{5.0f};
    float paramChikariVol_{0.70f};
    float masterGainLin_{1.0f};

    std::array<KarplusStrongString, MAX_VOICES> voices_{};
    SympatheticBank tarabBank_{};
    ChikariStrings chikari_{};
    MeendController meend_{};
};

} // namespace sitar::dsp
