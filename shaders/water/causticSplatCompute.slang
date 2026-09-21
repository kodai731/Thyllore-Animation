import "include/torus_intersect.slang";
import "water/include/component.slang";
import "water/include/flow.slang";
import "water/include/surface.slang";
import "water/include/lb.slang";

struct SceneData {
    float4 lightPosition;
    float4 lightColor;
    float4x4 view;
    float4x4 proj;
    int debugMode;
    float shadowStrength;
    int enableDistanceAttenuation;
    float exposureValue;
};

[[vk::binding(0, 0)]] [format("r32ui")] RWTexture2D<uint> causticAccumImage;
[[vk::binding(1, 0)]] [format("rgba32f")] RWTexture2D<float4> positionImage;
[[vk::binding(2, 0)]] RaytracingAccelerationStructure topLevelAS;
[[vk::binding(3, 0)]] ConstantBuffer<SceneData> sceneData;
[[vk::binding(4, 0)]] ConstantBuffer<WaterUBO> waterBlock;

static const int CAUSTIC_GRID_SIZE = 512;
static const int WAVE_MODE_COUNT = 8;
static const float2 WAVE_FOOTPRINT = float2(0.002);
static const int CAUSTIC_DEBUG_OFF = 0;
static const int CAUSTIC_DEBUG_SPLAT_MARKERS = 1;
static const int CAUSTIC_DEBUG_SKIP_DEPTH_MATCH = 2;
static const int CAUSTIC_DEBUG_FACTOR_PROBE = 3;
static const int CAUSTIC_DEBUG_DEPTH_PROBE = 4;
static const int CAUSTIC_DEBUG_HIT_PROBE = 5;
static const int CAUSTIC_DEBUG_FACTOR_NAN_PROBE = 6;
static const int CAUSTIC_DEBUG_DEPTH_PACKED = 7;
static const int CAUSTIC_DEBUG_OFFSET_PACKED = 8;

// Aqoole reflectance P/S average, etaRel = n_transmitted / n_incident
float fresnelAverage(float cosThetaI, float etaRel) {
    float sinThetaT2 = (1.0 - cosThetaI * cosThetaI) / (etaRel * etaRel);
    float cosThetaT = sqrt(max(1.0 - sinThetaT2, 0.0));
    float rPar = (etaRel * cosThetaI - cosThetaT) / (etaRel * cosThetaI + cosThetaT);
    float rPerp = (cosThetaI - etaRel * cosThetaT) / (cosThetaI + etaRel * cosThetaT);
    return (rPar * rPar + rPerp * rPerp) * 0.5;
}

float3 waveNormalAt(float3 pLocal, float rHat) {
    float2 uv = torusUV(pLocal);
    float h, hu, hv, slopeVariance;
    waterHeightAndGradient(uv, water.flow.z, water.flow.xy, WAVE_MODE_COUNT, WAVE_FOOTPRINT, h, hu, hv, slopeVariance);
    waterLbHeightAndGradient(uv, water.flow.z, water.flow.xy, h, hu, hv);
    return waterPerturbedNormal(uv.x, uv.y, h, hu, hv, rHat);
}

int2 positionImageSize() {
    uint width, height;
    positionImage.GetDimensions(width, height);
    return int2(width, height);
}

void accumulate(int2 pixel, uint value) {
    InterlockedAdd(causticAccumImage[pixel], value);
}

void accumulateClamped(int2 pixel, uint value) {
    accumulate(clamp(pixel, int2(0), positionImageSize() - int2(1)), value);
}

void exchangeClamped(int2 pixel, uint value) {
    uint original;
    InterlockedExchange(causticAccumImage[clamp(pixel, int2(0), positionImageSize() - int2(1))], value, original);
}

bool isDebugMode(int mode) {
    return int(water.composite.w) == mode;
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 globalInvocationId : SV_DispatchThreadID) {
    water = waterBlock;
    int2 gridCoord = int2(globalInvocationId.xy);
    if (gridCoord.x >= CAUSTIC_GRID_SIZE || gridCoord.y >= CAUSTIC_GRID_SIZE) {
        return;
    }

    float3 center = mul(water.model, float4(0.0, 0.0, 0.0, 1.0)).xyz;
    float gridRadius = water.radii.x + water.radii.y * 2.0;

    float3 axis = normalize(center - sceneData.lightPosition.xyz);
    float3 up = abs(axis.y) < 0.9999 ? float3(0.0, 1.0, 0.0) : float3(1.0, 0.0, 0.0);
    float3 basisU = normalize(cross(axis, up));
    float3 basisV = cross(axis, basisU);

    float offsetU = (float(gridCoord.x) / float(CAUSTIC_GRID_SIZE - 1) - 0.5) * 2.0 * gridRadius;
    float offsetV = (float(gridCoord.y) / float(CAUSTIC_GRID_SIZE - 1) - 0.5) * 2.0 * gridRadius;
    float3 gridPoint = center + offsetU * basisU + offsetV * basisV;

    float3 origin = sceneData.lightPosition.xyz;
    float3 dir = normalize(gridPoint - origin);

    float3 localOrigin = mul(water.inverseModel, float4(origin, 1.0)).xyz / water.radii.x;
    float3 localDir = normalize(mul(water.inverseModel, float4(dir, 0.0)).xyz);
    float rHat = water.radii.y / water.radii.x;

    float entryRoots[4];
    bool entryFallback;
    if (intersectTorus(localOrigin, localDir, rHat, entryRoots, entryFallback) <= 0) {
        return;
    }
    float3 entryLocal = localOrigin + entryRoots[0] * localDir;

    if (isDebugMode(CAUSTIC_DEBUG_SPLAT_MARKERS)) {
        float3 entryWorld = mul(water.model, float4(entryLocal * water.radii.x, 1.0)).xyz;
        float4 entryClip = mul(mul(sceneData.proj, sceneData.view), float4(entryWorld, 1.0));
        if (entryClip.w > 0.0) {
            float2 entryUV = (entryClip.xy / entryClip.w) * 0.5 + 0.5;
            if (all(entryUV >= float2(0.0)) && all(entryUV < float2(1.0))) {
                accumulate(int2(entryUV * float2(positionImageSize())), 512u);
            }
        }
    }

    float ior = water.absorption.w;
    float3 entryNormal = waveNormalAt(entryLocal, rHat);
    float3 insideDir = refract(localDir, entryNormal, 1.0 / ior);
    if (length(insideDir) < 1e-4) {
        return;
    }
    float fresnelIn = fresnelAverage(-dot(localDir, entryNormal), ior);

    float exitRoots[4];
    bool exitFallback;
    if (intersectTorus(entryLocal + insideDir * 1e-3, insideDir, rHat, exitRoots, exitFallback) <= 0) {
        return;
    }
    float3 exitLocal = entryLocal + insideDir * (1e-3 + exitRoots[0]);
    float chord = (1e-3 + exitRoots[0]) * water.radii.x;

    float3 exitNormal = waveNormalAt(exitLocal, rHat);
    float3 exitDir = refract(insideDir, -exitNormal, ior);
    if (length(exitDir) < 1e-4) {
        return;
    }
    float fresnelOut = fresnelAverage(-dot(insideDir, -exitNormal), 1.0 / ior);

    float3 worldExitPos = mul(water.model, float4(exitLocal * water.radii.x, 1.0)).xyz;
    float3 worldExitDir = normalize(mul(float3x3(water.model), exitDir));

    RayDesc ray;
    ray.Origin = worldExitPos;
    ray.TMin = 0.01;
    ray.Direction = worldExitDir;
    ray.TMax = 1e30;
    RayQuery<RAY_FLAG_NONE> rayQuery;
    rayQuery.TraceRayInline(topLevelAS, RAY_FLAG_FORCE_OPAQUE | RAY_FLAG_SKIP_CLOSEST_HIT_SHADER, 0xFF, ray);

    // AABB candidates are the water itself: never generate, so only triangles commit
    while (rayQuery.Proceed()) {
    }

    if (rayQuery.CommittedStatus() != COMMITTED_TRIANGLE_HIT) {
        return;
    }

    float hitDistance = rayQuery.CommittedRayT();
    float3 hitPosition = worldExitPos + hitDistance * worldExitDir;

    float4 clip = mul(mul(sceneData.proj, sceneData.view), float4(hitPosition, 1.0));
    if (clip.w <= 0.0) {
        return;
    }
    float2 screenUV = (clip.xy / clip.w) * 0.5 + 0.5;
    if (any(screenUV < float2(0.0)) || any(screenUV >= float2(1.0))) {
        return;
    }

    int2 pixel = int2(screenUV * float2(positionImageSize()));

    if (isDebugMode(CAUSTIC_DEBUG_DEPTH_PROBE)) {
        float3 gbufferPosition = positionImage[pixel].xyz;
        float mismatch = length(hitPosition - gbufferPosition) / (0.05 * water.radii.x);
        accumulate(pixel, uint(clamp(mismatch, 0.0, 8.0) * 256.0));
        accumulateClamped(pixel + int2(60, 0), uint(clamp(length(gbufferPosition), 0.0, 8.0) * 256.0));
        accumulateClamped(pixel + int2(120, 0), 2048u);
        return;
    }

    if (isDebugMode(CAUSTIC_DEBUG_DEPTH_PACKED)) {
        float4 gbufferSample = positionImage[pixel];
        float mismatchR = length(hitPosition - gbufferSample.xyz) / water.radii.x;
        uint mismatchBits = uint(clamp(mismatchR * 100.0, 0.0, 65535.0));
        uint gbufferBits = uint(clamp(length(gbufferSample.xyz) * 64.0, 0.0, 4095.0));
        uint packed = mismatchBits | (gbufferBits << 16) | (1u << 28) | (gbufferSample.w > 0.5 ? (1u << 29) : 0u);
        exchangeClamped(pixel, packed);
        return;
    }

    if (isDebugMode(CAUSTIC_DEBUG_OFFSET_PACKED)) {
        float3 delta = hitPosition - positionImage[pixel].xyz;
        uint dx = uint(clamp(delta.x * 100.0 + 512.0, 0.0, 1023.0));
        uint dy = uint(clamp(delta.y * 100.0 + 512.0, 0.0, 1023.0));
        uint dz = uint(clamp(delta.z * 100.0 + 512.0, 0.0, 1023.0));
        exchangeClamped(pixel, dx | (dy << 10) | (dz << 20) | (1u << 30));
        return;
    }

    if (isDebugMode(CAUSTIC_DEBUG_HIT_PROBE)) {
        bool bad = any(isnan(hitPosition)) || any(isinf(hitPosition)) || any(isnan(worldExitDir)) || isnan(hitDistance);
        accumulateClamped(pixel, uint(clamp(hitPosition.x + 8.0, 0.0, 16.0) * 256.0));
        accumulateClamped(pixel + int2(60, 0), uint(clamp(hitPosition.y + 8.0, 0.0, 16.0) * 256.0));
        accumulateClamped(pixel + int2(120, 0), uint(clamp(hitPosition.z + 8.0, 0.0, 16.0) * 256.0));
        accumulateClamped(pixel + int2(180, 0), uint(clamp(hitDistance, 0.0, 16.0) * 256.0));
        accumulateClamped(pixel + int2(240, 0), bad ? 4096u : 2048u);
        return;
    }

    if (isDebugMode(CAUSTIC_DEBUG_SPLAT_MARKERS)) {
        accumulate(pixel, 2048u);
        return;
    }

    if (isDebugMode(CAUSTIC_DEBUG_OFF)) {
        float3 gbufferPosition = positionImage[pixel].xyz;
        if (length(hitPosition - gbufferPosition) > 0.1 * water.radii.x) {
            return;
        }
    }

    float absorptionMean = dot(water.absorption.rgb, float3(1.0 / 3.0));
    float transmittance = exp(-absorptionMean * chord);

    float majorRadius = water.radii.x;
    float cosV = clamp((length(exitLocal.xz) - 1.0) / rHat, -1.0, 1.0);
    float kappa1 = 1.0 / (rHat * majorRadius);
    float kappa2 = cosV / ((1.0 + rHat * cosV) * majorRadius);
    float k1t = ior * kappa1;
    float k2t = ior * kappa2;
    float focus = 1.0 / (abs((1.0 - hitDistance * k1t) * (1.0 - hitDistance * k2t)) + 0.05);

    if (isDebugMode(CAUSTIC_DEBUG_FACTOR_PROBE)) {
        accumulateClamped(pixel, uint(clamp(transmittance, 0.0, 1.0) * 2048.0));
        accumulateClamped(pixel + int2(60, 0), uint(clamp(1.0 - fresnelIn, 0.0, 1.0) * 2048.0));
        accumulateClamped(pixel + int2(120, 0), uint(clamp(1.0 - fresnelOut, 0.0, 1.0) * 2048.0));
        accumulateClamped(pixel + int2(180, 0), uint(clamp(focus, 0.0, 1.0) * 2048.0));
        return;
    }

    float weight = (1.0 - fresnelIn) * (1.0 - fresnelOut) * transmittance * focus;

    if (isDebugMode(CAUSTIC_DEBUG_FACTOR_NAN_PROBE)) {
        uint bits = 64u;
        if (isnan(transmittance) || isinf(transmittance)) bits |= 1u;
        if (isnan(fresnelIn) || isinf(fresnelIn)) bits |= 2u;
        if (isnan(fresnelOut) || isinf(fresnelOut)) bits |= 4u;
        if (isnan(focus) || isinf(focus)) bits |= 8u;
        if (isnan(chord) || isinf(chord)) bits |= 16u;
        if (!(weight * 256.0 >= 1.0)) bits |= 32u;
        if (transmittance <= 0.0) bits |= 128u;
        if (focus <= 0.0) bits |= 256u;
        if (fresnelIn >= 1.0) bits |= 512u;
        if (fresnelOut >= 1.0) bits |= 1024u;
        InterlockedOr(causticAccumImage[clamp(pixel, int2(0), positionImageSize() - int2(1))], bits);
        return;
    }

    accumulate(pixel, uint(clamp(weight, 0.0, 16.0) * 256.0));
}
