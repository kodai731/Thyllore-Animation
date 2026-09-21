module water_trace;

import "include/radiative_transfer.slang";
import "include/torus_intersect.slang";
import "water/include/component.slang";
import "water/include/flow.slang";
import "water/include/surface.slang";
import "water/include/lb.slang";
import "water/include/lighting.slang";

// Where a camera ray enters the water and where its refracted path leaves it, in world space.
public struct WaterSurfaceHit {
    public float3 entry;
    public float fresnel;
    public float3 reflectDir;
    public float chord;
    public float3 exitOrigin;
    public float slopeVariance;
    public float3 exitDir;
};

// oLocal / dLocal are the object-space ray with the origin divided by the major radius;
// roots are the ascending torus intersections of that ray.
public WaterSurfaceHit waterSurfaceHit(float3 oLocal, float3 dLocal, float roots[4], int hitCount) {
    float rHat = water.radii.y / water.radii.x;
    float chord = (hitCount >= 2) ? (roots[1] - roots[0]) * water.radii.x : 0.0;
    if (hitCount >= 4) {
        chord += (roots[3] - roots[2]) * water.radii.x;
    }

    float3 pLocal1 = oLocal + roots[0] * dLocal;
    float2 uv = torusUV(pLocal1);
    float h, hu, hv, slopeVariance;
    waterHeightAndGradient(uv, water.flow.z, water.flow.xy, int(water.composite.z), float2(0.002), h, hu, hv, slopeVariance);
    waterLbHeightAndGradient(uv, water.flow.z, water.flow.xy, h, hu, hv);
    float3 nLocal = waterPerturbedNormal(uv.x, uv.y, h, hu, hv, rHat);

    float eta = water.absorption.w;
    float3 rayDirWorld = normalize(mul(float3x3(water.model), dLocal));
    float3 n = normalize(mul(float3x3(water.model), nLocal));
    float cosThetaI = -dot(rayDirWorld, n);
    float sinThetaT2 = (1.0 - cosThetaI * cosThetaI) / (eta * eta);
    float cosThetaT = sqrt(max(1.0 - sinThetaT2, 0.0));
    float rPar = (eta * cosThetaI - cosThetaT) / (eta * cosThetaI + cosThetaT);
    float rPerp = (cosThetaI - eta * cosThetaT) / (cosThetaI + eta * cosThetaT);
    float F = (rPar * rPar + rPerp * rPerp) * 0.5;

    float3 dRefr = refract(dLocal, nLocal, 1.0 / eta);
    if (length(dRefr) < 1e-4) { dRefr = reflect(dLocal, nLocal); }
    float tExit = torusExitFromInside(pLocal1 + dRefr * 1e-3, dRefr, rHat);
    float3 pExitLocal = (tExit > 0.0) ? pLocal1 + dRefr * (1e-3 + tExit) : pLocal1;
    float3 nExit = normalize(torusGradient(pExitLocal, rHat));
    float3 dExit = refract(dRefr, -nExit, eta);
    if (length(dExit) < 1e-4) { dExit = reflect(dRefr, nExit); }

    WaterSurfaceHit hit;
    hit.entry = mul(water.model, float4(pLocal1 * water.radii.x, 1.0)).xyz;
    hit.fresnel = F;
    hit.reflectDir = normalize(mul(float3x3(water.model), reflect(dLocal, nLocal)));
    hit.chord = chord;
    hit.exitOrigin = mul(water.model, float4(pExitLocal * water.radii.x, 1.0)).xyz;
    hit.slopeVariance = slopeVariance;
    hit.exitDir = normalize(mul(float3x3(water.model), dExit));
    return hit;
}

// Reflection and transmission of one surface hit given what the two secondary rays saw.
public float3 waterTraceComposite(WaterSurfaceHit hit, bool reflectionHit, float3 reflectionColor, bool backgroundHit, float3 backgroundColor, float3 lightPos, float3 lightColor) {
    float3 reflection;
    if (reflectionHit) {
        reflection = reflectionColor;
    } else {
        float3 lightDir = normalize(lightPos - hit.entry);
        reflection = waterEnvironmentReflection(hit.reflectDir, lightDir, lightColor, hit.slopeVariance);
    }

    float3 background;
    if (backgroundHit) {
        background = backgroundColor;
    } else {
        background = float3(0.6, 0.7, 0.8) * water.lighting.z;
    }

    float3 transmission = lerp(background, water.tint.rgb, clamp(water.tint.a, 0.0, 1.0)) * rteTransmittance(waterExtinctionCoefficient(), hit.chord);
    float3 lightDirExit = normalize(lightPos - hit.exitOrigin);
    transmission += waterTransmittedHighlight(hit.exitDir, lightDirExit, lightColor, hit.chord);

    float scatterPath = length(hit.exitOrigin - hit.entry);
    if (scatterPath > 1e-6) {
        float3 viewDirWater = (hit.exitOrigin - hit.entry) / scatterPath;
        float ds = scatterPath / float(WATER_SCATTER_SAMPLES);
        for (int i = 0; i < WATER_SCATTER_SAMPLES; ++i) {
            WaterScatterSample smp = waterScatterSampleAt(hit.entry, hit.exitOrigin, i, lightPos);
            transmission += waterScatterSampleRadiance(smp, viewDirWater, lightColor, ds);
        }
    }
    return hit.fresnel * reflection * water.composite.x + (1.0 - hit.fresnel) * transmission * water.composite.y;
}
