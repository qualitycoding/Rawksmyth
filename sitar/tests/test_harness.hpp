#pragma once

#include <iostream>
#include <string>
#include <string_view>
#include <vector>
#include <cmath>
#include <chrono>
#include <functional>

namespace sitar::test {

struct TestCase {
    std::string id;
    std::string name;
    std::function<bool()> run;
};

class TestRunner {
public:
    static TestRunner& instance() {
        static TestRunner r;
        return r;
    }

    void addTest(std::string id, std::string name, std::function<bool()> testFunc) {
        tests_.push_back({ std::move(id), std::move(name), std::move(testFunc) });
    }

    int runAll() {
        std::cout << "=========================================================\n";
        std::cout << "  SITAR VST — VERIFIED FROZEN DSP TEST SUITE (C++20)     \n";
        std::cout << "=========================================================\n\n";

        size_t passed = 0;
        size_t failed = 0;

        for (const auto& t : tests_) {
            std::cout << "[" << t.id << "] " << t.name << " ... ";
            std::cout.flush();

            auto start = std::chrono::high_resolution_clock::now();
            bool ok = false;
            try {
                ok = t.run();
            } catch (const std::exception& e) {
                std::cout << "EXCEPTION: " << e.what() << " ";
                ok = false;
            } catch (...) {
                std::cout << "UNKNOWN EXCEPTION ";
                ok = false;
            }
            auto end = std::chrono::high_resolution_clock::now();
            auto durationUs = std::chrono::duration_cast<std::chrono::microseconds>(end - start).count();

            if (ok) {
                std::cout << "PASS (" << durationUs << " us)\n";
                passed++;
            } else {
                std::cout << "FAIL (" << durationUs << " us)\n";
                failed++;
            }
        }

        std::cout << "\n---------------------------------------------------------\n";
        std::cout << "Results: " << passed << " passed, " << failed << " failed, " << tests_.size() << " total\n";
        std::cout << "---------------------------------------------------------\n";

        return failed == 0 ? 0 : 1;
    }

private:
    std::vector<TestCase> tests_;
};

#define REGISTER_TEST(testId, testName, testFunc) \
    namespace { \
        const bool registered_##testId = []() { \
            ::sitar::test::TestRunner::instance().addTest(#testId, testName, testFunc); \
            return true; \
        }(); \
    }

// High-precision Pitch Estimator via windowed Autocorrelation with parabolic peak refinement
// Window is centered around the nominal period T0 to avoid harmonic octave trapping
inline float estimatePitchAutocorr(const float* buffer, size_t length, float sampleRate, float expectedFreq) {
    if (length < 64 || expectedFreq <= 0.0f) return 0.0f;

    const float expectedLag = sampleRate / expectedFreq;
    const size_t minLag = static_cast<size_t>(std::max(2.0f, expectedLag * 0.70f));
    const size_t maxLag = static_cast<size_t>(std::min(static_cast<float>(length / 2), expectedLag * 1.45f));

    if (maxLag <= minLag + 2) return 0.0f;

    std::vector<float> r(maxLag + 2, 0.0f);
    size_t bestLag = minLag;
    float maxVal = -1e9f;

    for (size_t lag = minLag; lag <= maxLag; ++lag) {
        float sum = 0.0f;
        for (size_t i = 0; i < length - lag; ++i) {
            sum += buffer[i] * buffer[i + lag];
        }
        r[lag] = sum;
        if (sum > maxVal) {
            maxVal = sum;
            bestLag = lag;
        }
    }

    // Parabolic interpolation for sub-sample lag accuracy:
    // delta = 0.5 * (alpha - gamma) / (alpha - 2*beta + gamma)
    if (bestLag > minLag && bestLag < maxLag) {
        const float alpha = r[bestLag - 1];
        const float beta  = r[bestLag];
        const float gamma = r[bestLag + 1];
        const float denom = alpha - 2.0f * beta + gamma;
        float refinedLag = static_cast<float>(bestLag);
        if (std::abs(denom) > 1e-12f) {
            const float delta = 0.5f * (alpha - gamma) / denom;
            refinedLag += delta;
        }
        return sampleRate / refinedLag;
    }

    return sampleRate / static_cast<float>(bestLag);
}

} // namespace sitar::test
