// Reference raymarch integrators (debug / A-B comparison only, push.mode 1 and 3).
// Deliberately quarantined from the analytic includes: these are the only
// sample-based integrators allowed to exist, and the closed-form guard checks
// that no other file grows a t-lattice loop. They call the same point
// evaluators as the analytic path so the pair stays comparable.
module flame_reference_march;

import "flame/include/component.slang";
import "flame/include/ray.slang";
import "flame/include/noise_field.slang";
import "flame/include/radial_integral.slang";

public float integrateEmissionRaymarch(FlameRaySegment segment, int stepCount) {
    float dt = (segment.tFar - segment.tNear) / float(stepCount);
    if (dt <= 0.0) {
        return 0.0;
    }
    float sum = 0.0;
    for (int i = 0; i < stepCount; ++i) {
        float t = segment.tNear + (float(i) + 0.5) * dt;
        float3 p = segment.localOrigin + t * segment.localDir;
        float h = clamp(
            evaluateHeightAlongRay(t, segment.localOrigin.y, segment.localDir.y), 0.0, 1.0);
        float w = flameContourWiggle(p, h);
        if (segment.cylinderDomain && flame.noiseAmplitude != 0.0) {
            sum += flamePointOccupancyDensity(p, h, w);
        } else if (!segment.cylinderDomain && flame.trailMeta.sampleCount < 1.0) {
            sum += flamePointEmitterOccupancy(p, h, w);
        } else {
            float radial = segment.cylinderDomain ? flameRadialDensityFactor(float3(p.x / w, p.y, p.z / w), h) : 1.0;
            sum += evaluateHeightFalloff(h) * radial * flameNoiseErosionFactor(p, h);
        }
    }
    return sum * dt;
}

public float4 integrateRTERaymarch(FlameRaySegment segment, int stepCount) {
    float dt = (segment.tFar - segment.tNear) / float(stepCount);
    if (dt <= 0.0) {
        return float4(0.0);
    }
    float total = 0.0;
    float heightMean = 0.0;
    for (int i = 0; i < stepCount; ++i) {
        float t = segment.tNear + (float(i) + 0.5) * dt;
        float3 p = segment.localOrigin + t * segment.localDir;
        float h = clamp(evaluateHeightAlongRay(t, segment.localOrigin.y, segment.localDir.y), 0.0, 1.0);
        float w = flameContourWiggle(p, h);
        float rho;
        if (!segment.cylinderDomain) {
            rho = flamePointEmitterOccupancy(p, h, w);
        } else if (flame.noiseAmplitude != 0.0) {
            rho = flamePointOccupancyDensity(p, h, w);
        } else {
            rho = evaluateHeightFalloff(h)
                * flameRadialDensityFactor(float3(p.x / w, p.y, p.z / w), h)
                * flameNoiseErosionFactor(p, h);
        }
        total += rho * dt;
        heightMean += rho * dt * h;
    }
    heightMean = total > 1e-6 ? heightMean / total : 0.0;
    float tempNorm = clamp(total * 2.0, 0.0, 1.0) * (1.0 - 0.55 * heightMean);
    float boost = 1.0 + flame.edgeStyle.whiteBoost * tempNorm * tempNorm;

    float3 radiance = float3(0.0);
    float3 sigmaRgb = flame.sigmaT * lerp(float3(1.0), float3(1.0, 1.091, 1.333), clamp(flame.contourParams.sigmaDispersion, 0.0, 1.0));
    float3 transmittance = float3(1.0);
    for (int i = 0; i < stepCount; ++i) {
        float t = segment.tNear + (float(i) + 0.5) * dt;
        float3 p = segment.localOrigin + t * segment.localDir;
        float h = clamp(evaluateHeightAlongRay(t, segment.localOrigin.y, segment.localDir.y), 0.0, 1.0);
        float w = flameContourWiggle(p, h);
        float rho;
        if (!segment.cylinderDomain) {
            rho = flamePointEmitterOccupancy(p, h, w);
        } else if (flame.noiseAmplitude != 0.0) {
            rho = flamePointOccupancyDensity(p, h, w);
        } else {
            rho = evaluateHeightFalloff(h)
                * flameRadialDensityFactor(float3(p.x / w, p.y, p.z / w), h)
                * flameNoiseErosionFactor(p, h);
        }
        float3 tau = sigmaRgb * rho * dt;
        radiance += transmittance * flameRampColor(h) * flame.intensity * boost * (float3(1.0) - exp(-tau));
        transmittance *= exp(-tau);
    }
    return float4(radiance, 1.0 - dot(transmittance, float3(1.0 / 3.0)));
}
