#ifndef WATER_TRACE_GLSL
#define WATER_TRACE_GLSL

#include "include/torus_intersect.glsl"
#include "water/include/flow.glsl"
#include "water/include/surface.glsl"
#include "water/include/lb.glsl"
#include "water/include/lighting.glsl"

// Where a camera ray enters the water and where its refracted path leaves it, in world space.
struct WaterSurfaceHit {
    vec3 entry;
    float fresnel;
    vec3 reflectDir;
    float chord;
    vec3 exitOrigin;
    float slopeVariance;
    vec3 exitDir;
};

// oLocal / dLocal are the object-space ray with the origin divided by the major radius;
// roots are the ascending torus intersections of that ray.
WaterSurfaceHit waterSurfaceHit(vec3 oLocal, vec3 dLocal, float roots[4], int hitCount) {
    float rHat = water.radii.y / water.radii.x;
    float chord = (hitCount >= 2) ? (roots[1] - roots[0]) * water.radii.x : 0.0;
    if (hitCount >= 4) {
        chord += (roots[3] - roots[2]) * water.radii.x;
    }

    vec3 pLocal1 = oLocal + roots[0] * dLocal;
    vec2 uv = torusUV(pLocal1);
    float h, hu, hv, slopeVariance;
    waterHeightAndGradient(uv, water.flow.z, water.flow.xy, int(water.composite.z), vec2(0.002), h, hu, hv, slopeVariance);
    waterLbHeightAndGradient(uv, water.flow.z, water.flow.xy, h, hu, hv);
    vec3 nLocal = waterPerturbedNormal(uv.x, uv.y, h, hu, hv, rHat);

    float eta = water.absorption.w;
    vec3 rayDirWorld = normalize(mat3(water.model) * dLocal);
    vec3 n = normalize(mat3(water.model) * nLocal);
    float cosThetaI = -dot(rayDirWorld, n);
    float sinThetaT2 = (1.0 - cosThetaI * cosThetaI) / (eta * eta);
    float cosThetaT = sqrt(max(1.0 - sinThetaT2, 0.0));
    float rPar = (eta * cosThetaI - cosThetaT) / (eta * cosThetaI + cosThetaT);
    float rPerp = (cosThetaI - eta * cosThetaT) / (cosThetaI + eta * cosThetaT);
    float F = (rPar * rPar + rPerp * rPerp) * 0.5;

    vec3 dRefr = refract(dLocal, nLocal, 1.0 / eta);
    if (length(dRefr) < 1e-4) { dRefr = reflect(dLocal, nLocal); }
    float tExit = torusExitFromInside(pLocal1 + dRefr * 1e-3, dRefr, rHat);
    vec3 pExitLocal = (tExit > 0.0) ? pLocal1 + dRefr * (1e-3 + tExit) : pLocal1;
    vec3 nExit = normalize(torusGradient(pExitLocal, rHat));
    vec3 dExit = refract(dRefr, -nExit, eta);
    if (length(dExit) < 1e-4) { dExit = reflect(dRefr, nExit); }

    WaterSurfaceHit hit;
    hit.entry = (water.model * vec4(pLocal1 * water.radii.x, 1.0)).xyz;
    hit.fresnel = F;
    hit.reflectDir = normalize(mat3(water.model) * reflect(dLocal, nLocal));
    hit.chord = chord;
    hit.exitOrigin = (water.model * vec4(pExitLocal * water.radii.x, 1.0)).xyz;
    hit.slopeVariance = slopeVariance;
    hit.exitDir = normalize(mat3(water.model) * dExit);
    return hit;
}

// Reflection and transmission of one surface hit given what the two secondary rays saw.
vec3 waterTraceComposite(WaterSurfaceHit hit, bool reflectionHit, vec3 reflectionColor, bool backgroundHit, vec3 backgroundColor, vec3 lightPos, vec3 lightColor) {
    vec3 reflection;
    if (reflectionHit) {
        reflection = reflectionColor;
    } else {
        vec3 lightDir = normalize(lightPos - hit.entry);
        reflection = waterEnvironmentReflection(hit.reflectDir, lightDir, lightColor, hit.slopeVariance);
    }

    vec3 background;
    if (backgroundHit) {
        background = backgroundColor;
    } else {
        background = vec3(0.6, 0.7, 0.8) * water.lighting.z;
    }

    vec3 transmission = mix(background, water.tint.rgb, clamp(water.tint.a, 0.0, 1.0)) * rteTransmittance(waterExtinctionCoefficient(), hit.chord);
    vec3 lightDirExit = normalize(lightPos - hit.exitOrigin);
    transmission += waterTransmittedHighlight(hit.exitDir, lightDirExit, lightColor, hit.chord);

    float scatterPath = length(hit.exitOrigin - hit.entry);
    if (scatterPath > 1e-6) {
        vec3 viewDirWater = (hit.exitOrigin - hit.entry) / scatterPath;
        float ds = scatterPath / float(WATER_SCATTER_SAMPLES);
        for (int i = 0; i < WATER_SCATTER_SAMPLES; ++i) {
            WaterScatterSample smp = waterScatterSampleAt(hit.entry, hit.exitOrigin, i, lightPos);
            transmission += waterScatterSampleRadiance(smp, viewDirWater, lightColor, ds);
        }
    }
    return hit.fresnel * reflection * water.composite.x + (1.0 - hit.fresnel) * transmission * water.composite.y;
}

#endif
