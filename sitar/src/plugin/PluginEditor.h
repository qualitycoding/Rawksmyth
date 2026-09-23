#pragma once

#include "PluginProcessor.h"

#ifdef JUCE_MODULE_AVAILABLE
#include <juce_gui_basics/juce_gui_basics.h>

class SitarAudioProcessorEditor : public juce::AudioProcessorEditor {
public:
    explicit SitarAudioProcessorEditor(SitarAudioProcessor&);
    ~SitarAudioProcessorEditor() override = default;

    void paint(juce::Graphics&) override;
    void resized() override;

private:
    SitarAudioProcessor& audioProcessor;

    juce::Slider jawariBuzzSlider;
    juce::Slider jawariCurveSlider;
    juce::Slider tarabCouplingSlider;
    juce::Slider meendTimeSlider;
    juce::ComboBox ragaComboBox;

    std::unique_ptr<juce::AudioProcessorValueTreeState::SliderAttachment> buzzAttachment;
    std::unique_ptr<juce::AudioProcessorValueTreeState::SliderAttachment> curveAttachment;
    std::unique_ptr<juce::AudioProcessorValueTreeState::SliderAttachment> couplingAttachment;
    std::unique_ptr<juce::AudioProcessorValueTreeState::SliderAttachment> meendAttachment;
    std::unique_ptr<juce::AudioProcessorValueTreeState::ComboBoxAttachment> ragaAttachment;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(SitarAudioProcessorEditor)
};
#endif
