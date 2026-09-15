#version 450

#extension GL_GOOGLE_include_directive : require

#include "flame/include/ray.glsl"

layout(set = 0, binding = 0) uniform FrameUBO {
    mat4 view;
    mat4 proj;
    vec4 camera_pos;
    vec4 light_pos;
    vec4 light_color;
} frame;

#include "lightning/include/component.glsl"
#include "lightning/include/integral.glsl"
#include "lightning/include/reference_quadrature.glsl"

layout(set = 1, binding = 2) uniform sampler2D sceneDepthSampler;

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(push_constant) uniform LightningPush {
    int shadingMode;
    int stepCount;
    int debugView;
} push;

const int LIGHTNING_MODE_CLOSED_FORM = 0;
const int LIGHTNING_MODE_REFERENCE_QUADRATURE = 1;
const int LIGHTNING_DEBUG_OFF = 0;
const int LIGHTNING_DEBUG_COVERAGE = 1;
const int LIGHTNING_DEBUG_SEGMENT_HITS = 2;
const int LIGHTNING_DEBUG_CORE_COVERAGE = 3;
const float SEGMENT_T_MAX = 1e4;

void main() {
    vec3 rayDir = reconstructRayDirection(fragTexCoord, lightning.invViewProj, frame.camera_pos.xyz);
    vec3 localOrigin = (lightning.inverseModel * vec4(frame.camera_pos.xyz, 1.0)).xyz;
    vec3 localDir = (lightning.inverseModel * vec4(rayDir, 0.0)).xyz;

    float tNear = 0.0;
    float tFar = SEGMENT_T_MAX;
    if (!clampToLightningSegments(localOrigin, localDir, tNear, tFar)) {
        if (push.debugView != LIGHTNING_DEBUG_COVERAGE) {
            discard;
        } else {
            outColor = vec4(0.0, 0.0, 0.0, 1.0);
            return;
        }
    }
    tNear = max(tNear, 0.0);
    if (!clampToSceneDepth(
            sceneDepthSampler, fragTexCoord, lightning.invViewProj, frame.camera_pos.xyz, rayDir, tFar, tNear)
        || tFar <= tNear) {
        if (push.debugView != LIGHTNING_DEBUG_COVERAGE) {
            discard;
        } else {
            outColor = vec4(0.0, 0.0, 0.0, 1.0);
            return;
        }
    }

    float coverage;
    int hitCount;
    float coreCoverage;
    vec3 scattered;
    if (push.shadingMode == LIGHTNING_MODE_REFERENCE_QUADRATURE) {
        scattered = lightningReferenceEmission(localOrigin, localDir, tNear, tFar, push.stepCount,
                                               coverage, hitCount, coreCoverage);
    } else {
        scattered = lightningEmission(localOrigin, localDir, tNear, tFar, coverage, hitCount, coreCoverage);
    }

    if (push.debugView == LIGHTNING_DEBUG_COVERAGE) {
        outColor = vec4(vec3(coverage), 1.0);
        return;
    }
    if (push.debugView == LIGHTNING_DEBUG_SEGMENT_HITS) {
        outColor = vec4(float(hitCount) / float(LIGHTNING_MAX_SEGMENTS), 0.0, 0.0, 1.0);
        return;
    }
    if (push.debugView == LIGHTNING_DEBUG_CORE_COVERAGE) {
        outColor = vec4(vec3(coreCoverage), 1.0);
        return;
    }

    outColor = vec4(scattered, 1.0);
}
