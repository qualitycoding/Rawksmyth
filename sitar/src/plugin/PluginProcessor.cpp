#include "PluginProcessor.h"

#ifdef JUCE_MODULE_AVAILABLE
#include "PluginEditor.h"

SitarAudioProcessor::SitarAudioProcessor()
    : AudioProcessor(BusesProperties().withOutput("Output", juce::AudioChannelSet::stereo(), true)),
      apvts_(*this, nullptr, "Parameters", createParameterLayout())
{
}

juce::AudioProcessorValueTreeState::ParameterLayout SitarAudioProcessor::createParameterLayout() {
    std::vector<std::unique_ptr<juce::RangedAudioParameter>> params;

    for (const auto& p : sitar::dsp::PARAM_REGISTRY) {
        if (p.isDiscrete) {
            params.push_back(std::make_unique<juce::AudioParameterInt>(
                juce::ParameterID(juce::String(p.tag.data(), p.tag.size()), 1),
                juce::String(p.name.data(), p.name.size()),
                static_cast<int>(p.minValue),
                static_cast<int>(p.maxValue),
                static_cast<int>(p.defaultValue)
            ));
        } else {
            params.push_back(std::make_unique<juce::AudioParameterFloat>(
                juce::ParameterID(juce::String(p.tag.data(), p.tag.size()), 1),
                juce::String(p.name.data(), p.name.size()),
                juce::NormalisableRange<float>(p.minValue, p.maxValue),
                p.defaultValue
            ));
        }
    }

    return { params.begin(), params.end() };
}

void SitarAudioProcessor::prepareToPlay(double sampleRate, int /*samplesPerBlock*/) {
    synth_.prepare(static_cast<float>(sampleRate));
}

void SitarAudioProcessor::releaseResources() {
    synth_.reset();
}

bool SitarAudioProcessor::isBusesLayoutSupported(const BusesLayout& layouts) const {
    if (layouts.getMainOutputChannelSet() != juce::AudioChannelSet::mono()
     && layouts.getMainOutputChannelSet() != juce::AudioChannelSet::stereo())
        return false;

    return true;
}

void SitarAudioProcessor::processBlock(juce::AudioBuffer<float>& buffer, juce::MidiBuffer& midiMessages) {
    juce::ScopedNoDenormals noDenormals;
    const int totalNumInputChannels  = getTotalNumInputChannels();
    const int totalNumOutputChannels = getTotalNumOutputChannels();

    for (int i = totalNumInputChannels; i < totalNumOutputChannels; ++i)
        buffer.clear(i, 0, buffer.getNumSamples());

    // Sync parameters from APVTS
    for (const auto& p : sitar::dsp::PARAM_REGISTRY) {
        auto* rawParam = apvts_.getRawParameterValue(juce::String(p.tag.data(), p.tag.size()));
        if (rawParam) {
            synth_.setParameter(p.id, rawParam->load());
        }
    }

    // Process incoming MIDI
    for (const auto metadata : midiMessages) {
        const auto msg = metadata.getMessage();
        if (msg.isNoteOn()) {
            synth_.noteOn(msg.getNoteNumber(), msg.getFloatVelocity());
        } else if (msg.isNoteOff()) {
            synth_.noteOff(msg.getNoteNumber());
        } else if (msg.isPitchWheel()) {
            // Map 14-bit pitch wheel [0, 16383] with 8192 center to [-1.0, 1.0]
            const float normBend = (static_cast<float>(msg.getPitchWheelValue()) - 8192.0f) / 8192.0f;
            synth_.setPitchBend(normBend);
        } else if (msg.isController() && msg.getControllerNumber() == 64) { // Sustain pedal
            // Invert / damp or trigger chikari
            if (msg.getControllerValue() > 64) {
                synth_.strumJhala(0.85f);
            }
        }
    }

    float* leftChannel = buffer.getWritePointer(0);
    float* rightChannel = totalNumOutputChannels > 1 ? buffer.getWritePointer(1) : nullptr;

    synth_.processBlock(leftChannel, rightChannel, static_cast<size_t>(buffer.getNumSamples()));
}

juce::AudioProcessorEditor* SitarAudioProcessor::createEditor() {
    return new SitarAudioProcessorEditor(*this);
}

void SitarAudioProcessor::getStateInformation(juce::MemoryBlock& destData) {
    auto state = apvts_.copyState();
    std::unique_ptr<juce::XmlElement> xml(state.createXml());
    copyXmlToBinary(*xml, destData);
}

void SitarAudioProcessor::setStateInformation(const void* data, int sizeInBytes) {
    std::unique_ptr<juce::XmlElement> xmlState(getXmlFromBinary(data, sizeInBytes));
    if (xmlState != nullptr && xmlState->hasTagName(apvts_.state.getType())) {
        apvts_.replaceState(juce::ValueTree::fromXml(*xmlState));
    }
}

juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter() {
    return new SitarAudioProcessor();
}

#endif
