#ifndef VOLUME_PUFFS_GLSL
#define VOLUME_PUFFS_GLSL

// Biweight spheres scattered through a medium: each adds its entry and exit as ray knots and is
// integrated in closed form only on the pieces between them. The including effect implements
// puffCount / puffSphere (xyz centre, w radius) from its own storage.
// Mirrored in thyllore-effect-core/src/volume/puffs.rs.

#include "include/compact_support.glsl"
#include "include/ray_knots.glsl"

const int PUFFS_PER_RAY = 20;
const float PUFFS_EMPTY_INTERVAL_EPSILON = 1e-6;

struct RayPuffs {
    int count;
    int index[PUFFS_PER_RAY];
    float enter[PUFFS_PER_RAY];
    float exit[PUFFS_PER_RAY];
};

int puffCount();
vec4 puffSphere(int index);

void puffsCollectKnots(
    vec3 o, vec3 d, float tNear, float tFar,
    inout float knots[RAY_MAX_KNOTS], inout int count, out RayPuffs puffs) {
    puffs.count = 0;
    float a = dot(d, d);
    for (int i = 0; i < puffCount(); ++i) {
        if (puffs.count >= PUFFS_PER_RAY) break;
        vec4 sphere = puffSphere(i);
        if (sphere.w <= 0.0) continue;
        vec3 dx = o - sphere.xyz;
        float b = 2.0 * dot(dx, d);
        float c = dot(dx, dx) - sphere.w * sphere.w;
        float discriminant = b * b - 4.0 * a * c;
        if (discriminant <= 0.0) continue;
        float sqrtDiscriminant = sqrt(discriminant);
        float t0 = (-b - sqrtDiscriminant) / (2.0 * a);
        float t1 = (-b + sqrtDiscriminant) / (2.0 * a);
        if (t1 <= tNear || t0 >= tFar) continue;
        rayPushKnot(knots, count, t0, tNear, tFar);
        rayPushKnot(knots, count, t1, tNear, tFar);
        puffs.index[puffs.count] = i;
        puffs.enter[puffs.count] = t0;
        puffs.exit[puffs.count] = t1;
        puffs.count += 1;
    }
}

bool puffsPieceHolds(RayPuffs puffs, float s0, float s1) {
    float sMid = 0.5 * (s0 + s1);
    for (int k = 0; k < puffs.count; ++k) {
        if (sMid > puffs.enter[k] && sMid < puffs.exit[k]) {
            return true;
        }
    }
    return false;
}

// Unit-strength integral of every puff enclosing the piece [s0, s1], which must not cross a knot.
float puffsPieceIntegral(RayPuffs puffs, vec3 o, vec3 d, float s0, float s1) {
    float pieceLength = s1 - s0;
    if (pieceLength <= PUFFS_EMPTY_INTERVAL_EPSILON || puffs.count == 0) {
        return 0.0;
    }
    float sMid = 0.5 * (s0 + s1);
    vec3 start = o + d * s0;

    float total = 0.0;
    for (int k = 0; k < puffs.count; ++k) {
        if (sMid <= puffs.enter[k] || sMid >= puffs.exit[k]) continue;
        vec4 sphere = puffSphere(puffs.index[k]);
        total += biweightSpherePieceIntegral(sphere.xyz, sphere.w, start, d, pieceLength);
    }
    return pieceLength * total;
}

#endif
