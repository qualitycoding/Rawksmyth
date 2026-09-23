#pragma once

#include <string_view>
#include <array>
#include <cstdint>

namespace sitar::dsp {

enum class ParamId : uint32_t {
    TonicFreq = 0,
    PluckPosition,
    PluckHardness,
    JawariBuzz,
    JawariCurvature,
    DecayTime,
    Damping,
    SympatheticCoupling,
    SympatheticQ,
    RagaScale,
    MeendTime,
    MeendRange,
    ChikariVolume,
    MasterGain,
    NumParams
};

struct ParameterDef {
    ParamId id;
    std::string_view tag;
    std::string_view name;
    std::string_view unit;
    float minValue;
    float maxValue;
    float defaultValue;
    bool isDiscrete;
    int numSteps;
};

inline constexpr std::array<ParameterDef, static_cast<size_t>(ParamId::NumParams)> PARAM_REGISTRY = {{
    { ParamId::TonicFreq,           "tonic_freq",       "Tonic (Sa) Frequency",     "Hz", 110.0f, 440.0f, 220.0f, false, 0 },
    { ParamId::PluckPosition,       "pluck_pos",        "Pluck Position",           "%",  0.05f,  0.45f,  0.18f,  false, 0 },
    { ParamId::PluckHardness,       "pluck_hardness",   "Mizrab Hardness",          "",   0.1f,   1.0f,   0.75f,  false, 0 },
    { ParamId::JawariBuzz,          "jawari_buzz",      "Jawari Buzz (Jivari)",     "%",  0.0f,   1.0f,   0.65f,  false, 0 },
    { ParamId::JawariCurvature,     "jawari_curve",     "Bridge Curvature",         "%",  0.0f,   1.0f,   0.50f,  false, 0 },
    { ParamId::DecayTime,           "decay_time",       "Decay Time",               "s",  0.5f,   12.0f,  4.0f,   false, 0 },
    { ParamId::Damping,             "damping",          "String Damping",           "%",  0.0f,   1.0f,   0.30f,  false, 0 },
    { ParamId::SympatheticCoupling, "tarab_coupling",   "Tarab Sympathetic Coupling","%", 0.0f,   1.0f,   0.45f,  false, 0 },
    { ParamId::SympatheticQ,        "tarab_q",          "Tarab Resonance Q",        "",   50.0f,  600.0f, 300.0f, false, 0 },
    { ParamId::RagaScale,           "raga_scale",       "Raga Scale Tuning",        "",   0.0f,   4.0f,   0.0f,   true,  5 },
    { ParamId::MeendTime,           "meend_time",       "Meend Glide Time",         "ms", 5.0f,   600.0f, 80.0f,  false, 0 },
    { ParamId::MeendRange,          "meend_range",      "Meend Bend Range",         "st", 1.0f,   7.0f,   5.0f,   true,  7 },
    { ParamId::ChikariVolume,       "chikari_vol",      "Chikari Volume",           "%",  0.0f,   1.0f,   0.70f,  false, 0 },
    { ParamId::MasterGain,          "master_gain",      "Master Gain",              "dB", -36.0f, 6.0f,   0.0f,   false, 0 }
}};

} // namespace sitar::dsp
