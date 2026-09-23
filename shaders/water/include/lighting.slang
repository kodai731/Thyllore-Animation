module water_lighting;

import "include/radiative_transfer.slang";
import "include/torus_intersect.slang";
import "water/include/component.slang";

public static const int WATER_SCATTER_SAMPLES = 4;

public float waterFresnelReflectance(float cosThetaI, float eta) {
    float sinThetaT2 = (1.0 - cosThetaI * cosThetaI) / (eta * eta);
    float cosThetaT = sqrt(max(1.0 - sinThetaT2, 0.0));
    float rPar = (eta * cosThetaI - cosThetaT) / (eta * cosThetaI + cosThetaT);
    float rPerp = (cosThetaI - eta * cosThetaT) / (cosThetaI + eta * cosThetaT);
    return (rPar * rPar + rPerp * rPerp) * 0.5;
}

public float3 waterScatteringCoefficient() {
    return water.tint.rgb * water.lighting.w;
}

public float3 waterExtinctionCoefficient() {
    return water.absorption.rgb + waterScatteringCoefficient();
}

public float3 waterEnvironmentReflection(float3 reflDir, float3 lightDir, float3 lightColor, float slopeVariance) {
    float sharpness = water.lighting.y / (1.0 + water.lighting.y * slopeVariance);
    float spec = pow(max(dot(reflDir, lightDir), 0.0), sharpness);
    return float3(0.6, 0.7, 0.8) * water.lighting.z + lightColor * water.lighting.x * spec;
}

public float3 waterTransmittedHighlight(float3 exitDir, float3 lightDir, float3 lightColor, float chord) {
    float spec = pow(max(dot(exitDir, lightDir), 0.0), water.lighting.y);
    return lightColor * water.lighting.x * spec * rteTransmittance(waterExtinctionCoefficient(), chord);
}

public struct WaterScatterSample {
    public float3 position;
    public float viewDistance;
    public float3 lightDir;
    public float3 lightExitPoint;
    public float waterDistance;
    public float surfaceTransmission;
};

// Light path from an interior point to the light: straight line, first exit through the surface,
// plus the ring chord if the line re-enters the torus (self-occlusion); Snell bending is ignored.
public WaterScatterSample waterScatterSampleAt(float3 entry, float3 exit, int index, float3 lightPos) {
    WaterScatterSample smp;
    float pathLength = length(exit - entry);
    smp.viewDistance = rteMidpointDistance(index, WATER_SCATTER_SAMPLES, pathLength);
    smp.position = entry + (exit - entry) * (smp.viewDistance / pathLength);
    smp.lightDir = normalize(lightPos - smp.position);

    float rHat = water.radii.y / water.radii.x;
    float3 originLocal = mul(water.inverseModel, float4(smp.position, 1.0)).xyz / water.radii.x;
    float3 dirLocal = normalize(mul(water.inverseModel, float4(smp.lightDir, 0.0)).xyz);
    float firstExit = torusExitFromInside(originLocal, dirLocal, rHat);
    float3 exitLocal = originLocal + dirLocal * firstExit;
    float lastExit = firstExit;
    float insideDistance = firstExit;

    float ringEntry = torusEntryFromOutside(exitLocal + dirLocal * 1e-3, dirLocal, rHat);
    if (ringEntry > 0.0) {
        float ringStart = firstExit + 1e-3 + ringEntry;
        float ringChord = torusExitFromInside(originLocal + dirLocal * (ringStart + 1e-3), dirLocal, rHat);
        insideDistance += ringChord;
        lastExit = ringStart + 1e-3 + ringChord;
    }
    smp.waterDistance = insideDistance * water.radii.x;
    smp.lightExitPoint = smp.position + smp.lightDir * (lastExit * water.radii.x);

    float3 nExit = normalize(mul(float3x3(water.model), torusGradient(exitLocal, rHat)));
    smp.surfaceTransmission = 1.0 - waterFresnelReflectance(max(dot(nExit, smp.lightDir), 0.0), water.absorption.w);
    return smp;
}

public float3 waterScatterSampleRadiance(WaterScatterSample smp, float3 viewDir, float3 lightColor, float ds) {
    float3 sigmaS = waterScatteringCoefficient();
    float3 sigmaT = waterExtinctionCoefficient();
    float3 lightRadiance = lightColor * water.lighting.x * smp.surfaceTransmission * rteTransmittance(sigmaT, smp.waterDistance);
    float phase = rteHenyeyGreenstein(dot(smp.lightDir, viewDir), water.scattering.x);
    return rteSingleScatterSample(sigmaS, sigmaT, phase, smp.viewDistance, ds, lightRadiance);
}
