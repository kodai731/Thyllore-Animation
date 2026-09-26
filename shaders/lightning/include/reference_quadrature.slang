// Sample-based reference for the closed-form integral (debug mode only): midpoint
// quadrature of the pointwise density. Never part of the product path.
module lightning_reference_quadrature;

import "lightning/include/component.slang";
import "lightning/include/integral.slang";
import "include/volume_capsule.slang";

public float3 lightningReferenceEmission(float3 origin, float3 dir, float tNear, float tFar, int stepCount,
                                         out float coverage, out int hitCount, out float coreCoverage) {
    int steps = max(stepCount, 1);
    float coreEmission = 0.0;
    float rimEmission = 0.0;
    coverage = 0.0;
    hitCount = 0;
    coreCoverage = 0.0;
    int count = lightningSegmentCount();
    for (int i = 0; i < LIGHTNING_MAX_SEGMENTS; ++i) {
        if (i >= count) break;
        float3 a = segments.seg_a_r0[i].xyz;
        float r0 = segments.seg_a_r0[i].w;
        float3 b = segments.seg_b_r1[i].xyz;
        float r1 = segments.seg_b_r1[i].w;
        float edgeWidthQ = segments.seg_misc[i].x;
        float segIntensity = segments.seg_misc[i].y;

        float3 center = (a + b) * 0.5;
        float radius = length(b - a) * 0.5 + max(r0, r1) * lightningRimRatio();
        float3 oc = origin - center;
        float halfB = dot(oc, dir);
        float c = dot(oc, oc) - radius * radius;

        float segTEnter, segTExit;
        if (c <= 0.0) {
            segTEnter = tNear;
            segTExit = tFar;
        } else {
            float disc = halfB * halfB - c;
            if (disc < 0.0) continue;
            float sqrtDisc = sqrt(disc);
            float t0 = -halfB - sqrtDisc;
            float t1 = -halfB + sqrtDisc;
            if (t1 < tNear || t0 > tFar) continue;
            segTEnter = max(t0, tNear);
            segTExit = min(t1, tFar);
        }

        float stepLen = (segTExit - segTEnter) / float(steps);
        float segCore = 0.0;
        float segRim = 0.0;
        for (int s = 0; s < steps; ++s) {
            float t = segTEnter + (float(s) + 0.5) * stepLen;
            float3 p = origin + dir * t;

            VolumeCapsule capsule;
            capsule.a = a;
            capsule.b = b;
            capsule.radiusStart = r0;
            capsule.radiusEnd = r1;
            capsule.edgeWidthQ = edgeWidthQ;

            float coreDensity = capsuleDensityAt(capsule, p);

            float rimRatio = lightningRimRatio();
            VolumeCapsule rimCapsule;
            rimCapsule.a = a;
            rimCapsule.b = b;
            rimCapsule.radiusStart = r0 * rimRatio;
            rimCapsule.radiusEnd = r1 * rimRatio;
            rimCapsule.edgeWidthQ = edgeWidthQ * rimRatio * rimRatio;

            float rimDensity = capsuleDensityAt(rimCapsule, p);

            if (rimDensity > 0.0 || coreDensity > 0.0) {
                hitCount++;
            }
            if (rimDensity > 0.0) {
                coverage = 1.0;
            }
            if (coreDensity > 0.0) {
                coreCoverage = 1.0;
            }

            segCore += stepLen * coreDensity;
            segRim += stepLen * rimDensity;
        }
        coreEmission = max(coreEmission, segIntensity * segCore);
        rimEmission = max(rimEmission, segIntensity * segRim);
    }
    return lightningCombineEmission(coreEmission, rimEmission);
}
