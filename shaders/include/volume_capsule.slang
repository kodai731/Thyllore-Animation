// Tapered capsule as a compact-support shell in the squared distance to the segment [a, b]:
//   density = 1 inside, faded by a smootherstep over the last `edgeWidthQ` of
//   delta = |p - nearest axis point|^2 - radius(lambda)^2.
// Along a ray delta is quadratic on each of the three regions (both caps and the body), so on
// every piece between knots the density is one polynomial integrated by exact power moments.
// Mirrored in thyllore-effect-core/src/volume/capsule.rs.
module volume_capsule;

import volume_shell;

public static const float CAPSULE_EMPTY_INTERVAL_EPSILON = 1e-6;

public static const int CAPSULE_REGION_CAP_START = 0;
public static const int CAPSULE_REGION_BODY = 1;
public static const int CAPSULE_REGION_CAP_END = 2;

public struct VolumeCapsule {
    public float3 a;
    public float3 b;
    public float radiusStart;
    public float radiusEnd;
    public float edgeWidthQ;
};

public bool capsuleAxis(VolumeCapsule capsule, out float3 axisDirection, out float axisLength) {
    float3 axis = capsule.b - capsule.a;
    axisLength = length(axis);
    if (axisLength < RAY_LINEAR_COEFFICIENT_EPSILON) {
        axisDirection = float3(0.0);
        return false;
    }
    axisDirection = axis / axisLength;
    return true;
}

public int capsuleRegionOfPoint(VolumeCapsule capsule, float3 point, float3 axisDirection, float axisLength) {
    float lambda = dot(point - capsule.a, axisDirection);
    if (lambda <= 0.0) {
        return CAPSULE_REGION_CAP_START;
    }
    if (lambda >= axisLength) {
        return CAPSULE_REGION_CAP_END;
    }
    return CAPSULE_REGION_BODY;
}

public void capsuleCapDeltaQuadratic(
    float3 offset, float3 direction, float radius,
    out float delta2, out float delta1, out float delta0) {
    delta2 = 1.0;
    delta1 = 2.0 * dot(offset, direction);
    delta0 = dot(offset, offset) - radius * radius;
}

public float capsuleDensityFromDelta(float delta, float edgeWidthQ) {
    if (delta >= 0.0) {
        return 0.0;
    }
    if (delta <= -edgeWidthQ) {
        return 1.0;
    }
    float v = (delta + edgeWidthQ) / edgeWidthQ;
    return 1.0 - v * v * v * (10.0 - v * (15.0 - 6.0 * v));
}

// delta(s) = |p(s) - nearest axis point|^2 - radius^2 of one region, as coefficients
// (s^2, s, 1) of the ray parameter; the direction must be normalised.
public void capsuleDeltaQuadratic(
    VolumeCapsule capsule, float3 origin, float3 direction, int region,
    float3 axisDirection, float axisLength,
    out float delta2, out float delta1, out float delta0) {
    if (region == CAPSULE_REGION_CAP_START) {
        capsuleCapDeltaQuadratic(origin - capsule.a, direction, capsule.radiusStart, delta2, delta1, delta0);
    } else if (region == CAPSULE_REGION_CAP_END) {
        capsuleCapDeltaQuadratic(origin - capsule.b, direction, capsule.radiusEnd, delta2, delta1, delta0);
    } else {
        float3 offset = origin - capsule.a;
        float lambda0 = dot(offset, axisDirection);
        float lambda1 = dot(direction, axisDirection);
        float taper = (capsule.radiusEnd - capsule.radiusStart) / axisLength;
        float radius0 = capsule.radiusStart + taper * lambda0;
        float radius1 = taper * lambda1;
        delta2 = 1.0 - lambda1 * lambda1 - radius1 * radius1;
        delta1 = 2.0 * (dot(offset, direction) - lambda0 * lambda1 - radius0 * radius1);
        delta0 = dot(offset, offset) - lambda0 * lambda0 - radius0 * radius0;
    }
}

public float capsuleDeltaAtPoint(VolumeCapsule capsule, float3 point, float3 axisDirection, float axisLength) {
    int region = capsuleRegionOfPoint(capsule, point, axisDirection, axisLength);
    float delta2, delta1, delta0;
    capsuleDeltaQuadratic(capsule, point, float3(0.0), region, axisDirection, axisLength, delta2, delta1, delta0);
    return delta0;
}

public float capsuleDensityAt(VolumeCapsule capsule, float3 point) {
    float3 axisDirection;
    float axisLength;
    if (!capsuleAxis(capsule, axisDirection, axisLength)) {
        return 0.0;
    }
    return capsuleDensityFromDelta(
        capsuleDeltaAtPoint(capsule, point, axisDirection, axisLength), capsule.edgeWidthQ);
}

// Region boundaries and support boundaries of the ray, appended unsorted.
public void capsuleCollectKnots(
    VolumeCapsule capsule, float3 origin, float3 direction, float tNear, float tFar,
    inout float knots[RAY_MAX_KNOTS], inout int count) {
    float3 axisDirection;
    float axisLength;
    if (!capsuleAxis(capsule, axisDirection, axisLength)) {
        return;
    }

    float lambda0 = dot(origin - capsule.a, axisDirection);
    float lambda1 = dot(direction, axisDirection);
    if (abs(lambda1) >= RAY_LINEAR_COEFFICIENT_EPSILON) {
        rayPushKnot(knots, count, -lambda0 / lambda1, tNear, tFar);
        rayPushKnot(knots, count, (axisLength - lambda0) / lambda1, tNear, tFar);
    }

    for (int region = CAPSULE_REGION_CAP_START; region <= CAPSULE_REGION_CAP_END; ++region) {
        float delta2, delta1, delta0;
        capsuleDeltaQuadratic(
            capsule, origin, direction, region, axisDirection, axisLength, delta2, delta1, delta0);
        rayPushQuadraticRoots(delta2, delta1, delta0, tNear, tFar, knots, count);
        rayPushQuadraticRoots(delta2, delta1, delta0 + capsule.edgeWidthQ, tNear, tFar, knots, count);
    }
}

public int capsuleKnots(
    VolumeCapsule capsule, float3 origin, float3 direction, float tNear, float tFar,
    out float knots[RAY_MAX_KNOTS]) {
    int count = rayBeginKnots(tNear, tFar, knots);
    capsuleCollectKnots(capsule, origin, direction, tNear, tFar, knots, count);
    raySortKnots(knots, count);
    return count;
}

// Integral of the density over the piece [s0, s1], which must not cross a knot.
public float capsulePieceIntegral(VolumeCapsule capsule, float3 origin, float3 direction, float s0, float s1) {
    float pieceLength = s1 - s0;
    if (pieceLength <= CAPSULE_EMPTY_INTERVAL_EPSILON) {
        return 0.0;
    }
    float3 axisDirection;
    float axisLength;
    if (!capsuleAxis(capsule, axisDirection, axisLength)) {
        return 0.0;
    }

    float sMid = s0 + 0.5 * pieceLength;
    float3 mid = origin + direction * sMid;
    float deltaMid = capsuleDeltaAtPoint(capsule, mid, axisDirection, axisLength);
    if (deltaMid >= 0.0) {
        return 0.0;
    }
    if (deltaMid <= -capsule.edgeWidthQ) {
        return pieceLength;
    }

    int region = capsuleRegionOfPoint(capsule, mid, axisDirection, axisLength);
    float delta2, delta1, delta0;
    capsuleDeltaQuadratic(
        capsule, origin, direction, region, axisDirection, axisLength, delta2, delta1, delta0);

    float invWidth = 1.0 / capsule.edgeWidthQ;
    float edge[POLY_TERMS];
    oneMinusSmootherstepQuadraticPoly(
        (delta2 * sMid * sMid + delta1 * sMid + delta0 + capsule.edgeWidthQ) * invWidth,
        (2.0 * delta2 * sMid + delta1) * pieceLength * invWidth,
        delta2 * pieceLength * pieceLength * invWidth,
        edge);
    return max(pieceLength * polySymmetricMoments(edge), 0.0);
}

public float capsuleRayEmission(VolumeCapsule capsule, float3 origin, float3 direction, float tNear, float tFar) {
    if (tFar <= tNear) {
        return 0.0;
    }
    float centerOffset = dot((capsule.a + capsule.b) * 0.5 - origin, direction);
    float3 centeredOrigin = origin + direction * centerOffset;
    float centeredNear = tNear - centerOffset;
    float centeredFar = tFar - centerOffset;

    float knots[RAY_MAX_KNOTS];
    int knotCount = capsuleKnots(capsule, centeredOrigin, direction, centeredNear, centeredFar, knots);
    float total = 0.0;
    for (int i = 1; i < knotCount; ++i) {
        total += capsulePieceIntegral(capsule, centeredOrigin, direction, knots[i - 1], knots[i]);
    }
    return total;
}
