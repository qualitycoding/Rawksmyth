#include "PluginEditor.h"

#ifdef JUCE_MODULE_AVAILABLE
SitarAudioProcessorEditor::SitarAudioProcessorEditor(SitarAudioProcessor& p)
    : AudioProcessorEditor(&p), audioProcessor(p)
{
    setSize(680, 420);

    // Setup UI sliders & attachments
    auto& apvts = audioProcessor.getAPVTS();

    jawariBuzzSlider.setSliderStyle(juce::Slider::RotaryHorizontalVerticalDrag);
    jawariBuzzSlider.setTextBoxStyle(juce::Slider::TextBoxBelow, false, 60, 20);
    addAndMakeVisible(jawariBuzzSlider);
    buzzAttachment = std::make_unique<juce::AudioProcessorValueTreeState::SliderAttachment>(apvts, "jawari_buzz", jawariBuzzSlider);

    jawariCurveSlider.setSliderStyle(juce::Slider::RotaryHorizontalVerticalDrag);
    jawariCurveSlider.setTextBoxStyle(juce::Slider::TextBoxBelow, false, 60, 20);
    addAndMakeVisible(jawariCurveSlider);
    curveAttachment = std::make_unique<juce::AudioProcessorValueTreeState::SliderAttachment>(apvts, "jawari_curve", jawariCurveSlider);

    tarabCouplingSlider.setSliderStyle(juce::Slider::RotaryHorizontalVerticalDrag);
    tarabCouplingSlider.setTextBoxStyle(juce::Slider::TextBoxBelow, false, 60, 20);
    addAndMakeVisible(tarabCouplingSlider);
    couplingAttachment = std::make_unique<juce::AudioProcessorValueTreeState::SliderAttachment>(apvts, "tarab_coupling", tarabCouplingSlider);

    meendTimeSlider.setSliderStyle(juce::Slider::LinearHorizontal);
    meendTimeSlider.setTextBoxStyle(juce::Slider::TextBoxRight, false, 60, 20);
    addAndMakeVisible(meendTimeSlider);
    meendAttachment = std::make_unique<juce::AudioProcessorValueTreeState::SliderAttachment>(apvts, "meend_time", meendTimeSlider);

    ragaComboBox.addItem("Raga Yaman (Kalyan)", 1);
    ragaComboBox.addItem("Raga Bhairav", 2);
    ragaComboBox.addItem("Raga Kafi", 3);
    ragaComboBox.addItem("Raga Darbari Kanada", 4);
    ragaComboBox.addItem("Raga Bilawal", 5);
    addAndMakeVisible(ragaComboBox);
    ragaAttachment = std::make_unique<juce::AudioProcessorValueTreeState::ComboBoxAttachment>(apvts, "raga_scale", ragaComboBox);
}

void SitarAudioProcessorEditor::paint(juce::Graphics& g) {
    g.fillAll(juce::Colour(0xff181716));

    g.setColour(juce::Colour(0xffd4af37)); // Gold
    g.setFont(24.0f);
    g.drawText("SITAR VST — PHYSICAL MODELING SYNTHESIZER", 20, 16, getWidth() - 40, 30, juce::Justification::centredLeft);

    g.setColour(juce::Colour(0xffa8a29e));
    g.setFont(12.0f);
    g.drawText("Jawari Bridge", 30, 70, 100, 20, juce::Justification::centred);
    g.drawText("Curvature", 140, 70, 100, 20, juce::Justification::centred);
    g.drawText("Tarab Sympathetic", 250, 70, 120, 20, juce::Justification::centred);
    g.drawText("Meend Portamento", 30, 210, 120, 20, juce::Justification::centredLeft);
    g.drawText("Raga Scale Tuning", 30, 290, 120, 20, juce::Justification::centredLeft);
}

void SitarAudioProcessorEditor::resized() {
    jawariBuzzSlider.setBounds(30, 95, 90, 90);
    jawariCurveSlider.setBounds(140, 95, 90, 90);
    tarabCouplingSlider.setBounds(260, 95, 90, 90);
    meendTimeSlider.setBounds(30, 235, 340, 30);
    ragaComboBox.setBounds(30, 315, 240, 30);
}
#endif
