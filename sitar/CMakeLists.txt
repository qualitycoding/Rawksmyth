cmake_minimum_required(VERSION 3.20)
project(SitarVST VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Compiler optimization flags
if (CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    add_compile_options(-O3 -Wall -Wextra -Wpedantic -Wno-unused-parameter)
endif()

# Include directories
include_directories(src/dsp src/plugin)

# 1. Pure C++20 DSP Core Library (Zero-Dependency)
add_library(sitar_dsp STATIC
    src/dsp/MathUtils.hpp
    src/dsp/DelayLine.hpp
    src/dsp/OnePoleFilter.hpp
    src/dsp/JawariBridge.hpp
    src/dsp/KarplusStrongString.hpp
    src/dsp/SympatheticResonator.hpp
    src/dsp/ChikariStrings.hpp
    src/dsp/MeendController.hpp
    src/dsp/Parameters.hpp
    src/dsp/SitarSynth.hpp
)
set_target_properties(sitar_dsp PROPERTIES LINKER_LANGUAGE CXX)

# 2. Frozen DSP Test Suite Executable
add_executable(sitar_tests
    tests/main.cpp
    tests/test_ks_tuning.cpp
    tests/test_jawari_buzz.cpp
    tests/test_sympathetic.cpp
    tests/test_meend.cpp
    tests/test_parameters.cpp
    tests/test_synth_engine.cpp
)
target_include_directories(sitar_tests PRIVATE src tests)
target_link_libraries(sitar_tests PRIVATE sitar_dsp)

# 3. Optional JUCE Audio Plugin Target (if JUCE is installed or submodule exists)
if (EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/JUCE" OR DEFINED JUCE_DIR)
    if (DEFINED JUCE_DIR)
        add_subdirectory(${JUCE_DIR} JUCE)
    else()
        add_subdirectory(JUCE)
    endif()

    juce_add_plugin(SitarPlugin
        COMPANY_NAME "QualityCoding"
        IS_SYNTH TRUE
        NEEDS_MIDI_INPUT TRUE
        NEEDS_MIDI_OUTPUT FALSE
        IS_MIDI_EFFECT FALSE
        EDITOR_WANTS_KEYBOARD_FOCUS TRUE
        COPY_PLUGIN_AFTER_BUILD FALSE
        PLUGIN_MANUFACTURER_CODE QCod
        PLUGIN_CODE STr1
        FORMATS VST3 Standalone AU
        PRODUCT_NAME "Sitar VST"
    )

    target_compile_definitions(SitarPlugin PRIVATE JUCE_MODULE_AVAILABLE=1)
    target_sources(SitarPlugin PRIVATE
        src/plugin/PluginProcessor.cpp
        src/plugin/PluginEditor.cpp
    )
    target_include_directories(SitarPlugin PRIVATE src/dsp src/plugin)
    target_link_libraries(SitarPlugin PRIVATE
        sitar_dsp
        juce::juce_audio_utils
        juce::juce_audio_processors
        juce::juce_gui_basics
    )
endif()

enable_testing()
add_test(NAME AllDSPTests COMMAND sitar_tests)
