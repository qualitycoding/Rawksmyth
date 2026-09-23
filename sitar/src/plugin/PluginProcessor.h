#pragma once

#ifdef JUCE_MODULE_AVAILABLE
#include <juce_audio_processors/juce_audio_processors.h>
#endif

#include "../dsp/SitarSynth.hpp"
#include "../dsp/Parameters.hpp"
#include <memory>

#ifdef JUCE_MODULE_AVAILABLE

class SitarAudioProcessor : public juce::AudioProcessor {
public:
    SitarAudioProcessor();
    ~SitarAudioProcessor() override = default;

    void prepareToPlay(double sampleRate, int samplesPerBlock) override;
    void releaseResources() override;
    bool isBusesLayoutSupported(const BusesLayout& layouts) const override;
    void processBlock(juce::AudioBuffer<float>&, juce::MidiBuffer&) override;

    juce::AudioProcessorEditor* createEditor() override;
    bool hasEditor() const override { return true; }

    const juce::String getName() const override { return "Sitar VST"; }
    bool acceptsMidi() const override { return true; }
    bool producesMidi() const override { return false; }
    bool isMidiEffect() const override { return false; }
    double getTailLengthSeconds() const override { return 3.0; }

    int getNumPrograms() override { return 1; }
    int getCurrentProgram() override { return 0; }
    void setCurrentProgram(int) override {}
    const juce::String getProgramName(int) override { return {}; }
    void changeProgramName(int, const juce::String&) override {}

    void getStateInformation(juce::MemoryBlock& destData) override;
    void setStateInformation(const void* data, int sizeInBytes) override;

    sitar::dsp::SitarSynth& getSynth() noexcept { return synth_; }
    juce::AudioProcessorValueTreeState& getAPVTS() noexcept { return apvts_; }

private:
    juce::AudioProcessorValueTreeState::ParameterLayout createParameterLayout();

    sitar::dsp::SitarSynth synth_;
    juce::AudioProcessorValueTreeState apvts_;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(SitarAudioProcessor)
};

#else

// Standalone fallback when compiling without full JUCE framework
class SitarAudioProcessor {
public:
    SitarAudioProcessor() {
        synth_.prepare(44100.0f);
    }
    sitar::dsp::SitarSynth& getSynth() noexcept { return synth_; }
private:
    sitar::dsp::SitarSynth synth_;
};

#endif
