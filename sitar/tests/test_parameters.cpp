#include "test_harness.hpp"
#include "../src/dsp/Parameters.hpp"
#include <unordered_set>
#include <string>

using namespace sitar::dsp;
using namespace sitar::test;

// T-012: Parameter bounds validity
bool test_T012_parameter_bounds() {
    for (const auto& p : PARAM_REGISTRY) {
        if (p.minValue >= p.maxValue) {
            std::cout << "[T-012 FAIL] Parameter " << p.tag << " has min >= max ";
            return false;
        }
        if (p.defaultValue < p.minValue || p.defaultValue > p.maxValue) {
            std::cout << "[T-012 FAIL] Parameter " << p.tag << " default out of bounds ";
            return false;
        }
        if (p.isDiscrete && p.numSteps <= 1) {
            std::cout << "[T-012 FAIL] Discrete parameter " << p.tag << " numSteps <= 1 ";
            return false;
        }
    }
    return true;
}
REGISTER_TEST(T012, "Parameter Metadata Registry Bounds & Range Invariants", test_T012_parameter_bounds)

// T-018: Parameter tag uniqueness and serialization format
bool test_T018_parameter_uniqueness() {
    std::unordered_set<std::string_view> tags;
    for (const auto& p : PARAM_REGISTRY) {
        if (p.tag.empty()) {
            std::cout << "[T-018 FAIL] Empty parameter tag ";
            return false;
        }
        if (tags.find(p.tag) != tags.end()) {
            std::cout << "[T-018 FAIL] Duplicate parameter tag: " << p.tag << " ";
            return false;
        }
        tags.insert(p.tag);
    }
    return true;
}
REGISTER_TEST(T018, "Parameter Tag Identifier Uniqueness across Registry", test_T018_parameter_uniqueness)
