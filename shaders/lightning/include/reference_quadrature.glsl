#ifndef LIGHTNING_REFERENCE_QUADRATURE_GLSL
#define LIGHTNING_REFERENCE_QUADRATURE_GLSL

#include "include/volume_capsule.glsl"

vec3 lightningReferenceEmission(vec3 o, vec3 d, float tNear, float tFar, int stepCount,
                                out float coverage, out int hitCount, out float coreCoverage) {
    int steps = max(stepCount, 1);
    float step = (tFar - tNear) / float(steps);
    vec3 sum = vec3(0.0);
    coverage = 0.0;
    hitCount = 0;
    coreCoverage = 0.0;
    int count = lightningSegmentCount();
    for (int s = 0; s < steps; ++s) {
        float t = tNear + (float(s) + 0.5) * step;
        vec3 p = o + d * t;
        for (int i = 0; i < LIGHTNING_MAX_SEGMENTS; ++i) {
            if (i >= count) break;
            vec3 a = segments.seg_a_r0[i].xyz;
            float r0 = segments.seg_a_r0[i].w;
            vec3 b = segments.seg_b_r1[i].xyz;
            float r1 = segments.seg_b_r1[i].w;
            float edgeWidthQ = segments.seg_misc[i].x;
            float segIntensity = segments.seg_misc[i].y;

            VolumeCapsule capsule;
            capsule.a = a;
            capsule.b = b;
            capsule.radiusStart = r0;
            capsule.radiusEnd = r1;
            capsule.edgeWidthQ = edgeWidthQ;

            float coreDensity = capsuleDensityAt(capsule, p);

            float glowRatio = lightningGlowRatio();
            VolumeCapsule glowCapsule;
            glowCapsule.a = a;
            glowCapsule.b = b;
            glowCapsule.radiusStart = r0 * glowRatio;
            glowCapsule.radiusEnd = r1 * glowRatio;
            glowCapsule.edgeWidthQ = edgeWidthQ * glowRatio * glowRatio;

            float glowDensity = capsuleDensityAt(glowCapsule, p);

            if (glowDensity > 0.0 || coreDensity > 0.0) {
                hitCount++;
            }
            if (glowDensity > 0.0) {
                coverage = 1.0;
            }
            if (coreDensity > 0.0) {
                coreCoverage = 1.0;
            }

            sum += segIntensity * step * (lightningCoreIntensity() * lightningCoreColor() * coreDensity
                                        + lightningGlowIntensity() * lightningGlowColor() * glowDensity);
        }
    }
    return sum;
}

#endif
