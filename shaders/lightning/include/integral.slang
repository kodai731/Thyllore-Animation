// Closed-form emission of the lightning bolt along a ray: the tapered capsule integral
// (include/volume_capsule.slang) of core + rim, combined over segments as a union (max) so the
// overlap of neighbouring capsules at joints and branch roots does not add up.
module lightning_integral;

import "lightning/include/component.slang";
import "include/volume_capsule.slang";

public static const int LIGHTNING_MAX_SEGMENTS = 256;

public float lightningRimRatio() {
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

public float3 lightningRimColor() {
    return lightning.rim.rgb;
}

public float lightningRimIntensity() {
    return lightning.rim.a;
}

public bool lightningSegmentReject(float3 origin, float3 dir, float tNear, float tFar,
                                   float3 a, float3 b, float r0, float r1, out float tEntry) {
    float3 center = (a + b) * 0.5;
    float radius = length(b - a) * 0.5 + max(r0, r1) * lightningRimRatio();
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
            + max(segments.seg_a_r0[i].w, segments.seg_b_r1[i].w) * lightningRimRatio();

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

public float3 lightningCombineEmission(float coreEmission, float rimEmission) {
    return lightningCoreIntensity() * lightningCoreColor() * coreEmission
         + lightningRimIntensity() * lightningRimColor() * rimEmission;
}

public void lightningAccumulateSegment(
    int i, float3 origin, float3 dir, float tNear, float tFar,
    inout float coreEmission, inout float rimEmission,
    inout float coverage, inout int hitCount, inout float coreCoverage) {
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

    float rimRatio = lightningRimRatio();
    VolumeCapsule rimCapsule;
    rimCapsule.a = a;
    rimCapsule.b = b;
    rimCapsule.radiusStart = r0 * rimRatio;
    rimCapsule.radiusEnd = r1 * rimRatio;
    rimCapsule.edgeWidthQ = edgeWidthQ * rimRatio * rimRatio;

    float eRim = capsuleRayEmission(rimCapsule, origin, dir, tNear, tFar);

    coreEmission = max(coreEmission, segIntensity * eCore);
    rimEmission = max(rimEmission, segIntensity * eRim);

    if (eRim > 0.0) {
        coverage = 1.0;
    }
    hitCount++;
    if (eCore > 0.0) {
        coreCoverage = 1.0;
    }
}

public float3 lightningEmission(float3 origin, float3 dir, float tNear, float tFar,
                                out float coverage, out int hitCount, out float coreCoverage) {
    float coreEmission = 0.0;
    float rimEmission = 0.0;
    coverage = 0.0;
    hitCount = 0;
    coreCoverage = 0.0;
    int count = lightningSegmentCount();
    for (int i = 0; i < LIGHTNING_MAX_SEGMENTS; ++i) {
        if (i >= count) break;
        lightningAccumulateSegment(i, origin, dir, tNear, tFar, coreEmission, rimEmission,
                                   coverage, hitCount, coreCoverage);
    }
    return lightningCombineEmission(coreEmission, rimEmission);
}
