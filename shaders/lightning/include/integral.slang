// Closed-form emission of the lightning bolt along a ray: sum over surviving segments of
// the tapered capsule integral (include/volume_capsule.slang) for core + glow.
module lightning_integral;

import "lightning/include/component.slang";
import "include/volume_capsule.slang";

public static const int LIGHTNING_MAX_SEGMENTS = 256;

public float lightningGlowRatio() {
    return lightning.shape.x;
}

public int lightningSegmentCount() {
    return int(lightning.shape.z);
}

public float3 lightningCoreColor() {
    return lightning.core.rgb;
}

public float lightningCoreIntensity() {
    return lightning.core.a;
}

public float3 lightningGlowColor() {
    return lightning.glow.rgb;
}

public float lightningGlowIntensity() {
    return lightning.glow.a;
}

public bool lightningSegmentReject(float3 origin, float3 dir, float tNear, float tFar,
                                   float3 a, float3 b, float r0, float r1, out float tEntry) {
    float3 center = (a + b) * 0.5;
    float radius = length(b - a) * 0.5 + max(r0, r1) * lightningGlowRatio();
    float3 oc = origin - center;
    float b_coef = dot(oc, dir);
    float c = dot(oc, oc) - radius * radius;
    if (c <= 0.0) {
        tEntry = tNear;
        return false;
    }
    float disc = b_coef * b_coef - c;
    if (disc < 0.0) {
        tEntry = tFar + 1.0;
        return true;
    }
    float sqrtDisc = sqrt(disc);
    float t0 = (-b_coef - sqrtDisc);
    float t1 = (-b_coef + sqrtDisc);
    if (t1 < tNear || t0 > tFar) {
        tEntry = tFar + 1.0;
        return true;
    }
    tEntry = max(t0, tNear);
    return false;
}

public bool clampToLightningSegments(float3 origin, float3 dir, inout float tNear, inout float tFar) {
    bool hasInterval = false;
    float entry = tNear;
    float exit = tFar;
    int count = lightningSegmentCount();
    for (int i = 0; i < LIGHTNING_MAX_SEGMENTS; ++i) {
        if (i >= count) break;
        float3 a = segments.seg_a_r0[i].xyz;
        float3 b = segments.seg_b_r1[i].xyz;
        float radius = length(b - a) * 0.5
            + max(segments.seg_a_r0[i].w, segments.seg_b_r1[i].w) * lightningGlowRatio();

        float3 oc = origin - (a + b) * 0.5;
        float halfB = dot(oc, dir);
        float c = dot(oc, oc) - radius * radius;
        float disc = halfB * halfB - c;
        if (disc < 0.0) {
            continue;
        }
        float sqrtDisc = sqrt(disc);
        float segmentEntry = max(-halfB - sqrtDisc, tNear);
        float segmentExit = min(-halfB + sqrtDisc, tFar);
        if (segmentExit <= segmentEntry) {
            continue;
        }

        entry = hasInterval ? min(entry, segmentEntry) : segmentEntry;
        exit = hasInterval ? max(exit, segmentExit) : segmentExit;
        hasInterval = true;
    }
    if (!hasInterval) {
        return false;
    }
    tNear = entry;
    tFar = exit;
    return true;
}

public void lightningAccumulateSegment(
    int i, float3 origin, float3 dir, float tNear, float tFar,
    inout float3 sum, inout float coverage, inout int hitCount, inout float coreCoverage) {
    float3 a = segments.seg_a_r0[i].xyz;
    float r0 = segments.seg_a_r0[i].w;
    float3 b = segments.seg_b_r1[i].xyz;
    float r1 = segments.seg_b_r1[i].w;
    float edgeWidthQ = segments.seg_misc[i].x;
    float segIntensity = segments.seg_misc[i].y;

    float tEntry;
    if (lightningSegmentReject(origin, dir, tNear, tFar, a, b, r0, r1, tEntry)) {
        return;
    }

    VolumeCapsule capsule;
    capsule.a = a;
    capsule.b = b;
    capsule.radiusStart = r0;
    capsule.radiusEnd = r1;
    capsule.edgeWidthQ = edgeWidthQ;

    float eCore = capsuleRayEmission(capsule, origin, dir, tNear, tFar);

    float glowRatio = lightningGlowRatio();
    VolumeCapsule glowCapsule;
    glowCapsule.a = a;
    glowCapsule.b = b;
    glowCapsule.radiusStart = r0 * glowRatio;
    glowCapsule.radiusEnd = r1 * glowRatio;
    // The glow decays from the axis over the full radius (no interior plateau),
    // so close-up framings read as a bright core with a fading halo.
    float glowRadiusMax = max(r0, r1) * glowRatio;
    glowCapsule.edgeWidthQ = glowRadiusMax * glowRadiusMax;

    float eGlow = capsuleRayEmission(glowCapsule, origin, dir, tNear, tFar);

    sum += segIntensity * (lightningCoreIntensity() * lightningCoreColor() * eCore
                         + lightningGlowIntensity() * lightningGlowColor() * eGlow);

    if (eGlow > 0.0) {
        coverage = 1.0;
    }
    hitCount++;
    if (eCore > 0.0) {
        coreCoverage = 1.0;
    }
}

public float3 lightningEmission(float3 origin, float3 dir, float tNear, float tFar,
                                out float coverage, out int hitCount, out float coreCoverage) {
    float3 sum = float3(0.0);
    coverage = 0.0;
    hitCount = 0;
    coreCoverage = 0.0;
    int count = lightningSegmentCount();
    for (int i = 0; i < LIGHTNING_MAX_SEGMENTS; ++i) {
        if (i >= count) break;
        lightningAccumulateSegment(i, origin, dir, tNear, tFar, sum, coverage, hitCount, coreCoverage);
    }
    return sum;
}
