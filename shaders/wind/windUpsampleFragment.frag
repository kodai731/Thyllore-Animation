#version 450

#extension GL_GOOGLE_include_directive : require

#include "include/depth.glsl"

layout(set = 0, binding = 0) uniform sampler2D windColorSampler;
layout(set = 0, binding = 1) uniform sampler2D sceneDepthSampler;

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

const float DEPTH_WEIGHT_FALLOFF = 32.0;
const float MIN_TOTAL_WEIGHT = 1e-4;

// Reverse-Z raw depth is proportional to 1/z, so the relative difference is scale invariant.
float depthWeight(float centerDepth, float tapDepth) {
    if (centerDepth == DEPTH_FAR && tapDepth == DEPTH_FAR) {
        return 1.0;
    }
    float relative = abs(centerDepth - tapDepth) / max(max(centerDepth, tapDepth), 1e-6);
    return 1.0 / (1.0 + DEPTH_WEIGHT_FALLOFF * relative);
}

void main() {
    vec2 halfSize = vec2(textureSize(windColorSampler, 0));
    vec2 texelCoord = fragTexCoord * halfSize - 0.5;
    vec2 baseCoord = floor(texelCoord);
    vec2 fraction = texelCoord - baseCoord;

    float centerDepth = texture(sceneDepthSampler, fragTexCoord).r;
    vec2 tapOffsets[4] = vec2[4](vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(1.0, 1.0));
    vec4 bilinearWeights = vec4(
        (1.0 - fraction.x) * (1.0 - fraction.y),
        fraction.x * (1.0 - fraction.y),
        (1.0 - fraction.x) * fraction.y,
        fraction.x * fraction.y);

    vec4 accumulated = vec4(0.0);
    float totalWeight = 0.0;
    vec4 nearestColor = vec4(0.0);
    float nearestWeight = -1.0;
    for (int tap = 0; tap < 4; ++tap) {
        vec2 tapUv = (baseCoord + tapOffsets[tap] + 0.5) / halfSize;
        vec4 tapColor = texture(windColorSampler, tapUv);
        float tapDepth = texture(sceneDepthSampler, tapUv).r;
        float weight = bilinearWeights[tap] * depthWeight(centerDepth, tapDepth);

        accumulated += tapColor * weight;
        totalWeight += weight;
        if (bilinearWeights[tap] > nearestWeight) {
            nearestWeight = bilinearWeights[tap];
            nearestColor = tapColor;
        }
    }

    outColor = totalWeight > MIN_TOTAL_WEIGHT ? accumulated / totalWeight : nearestColor;
}
