import "include/ray.slang";

[[vk::binding(0, 0)]] Sampler2D windColorSampler;
[[vk::binding(1, 0)]] Sampler2D sceneDepthSampler;

static const float DEPTH_WEIGHT_FALLOFF = 32.0;
static const float MIN_TOTAL_WEIGHT = 1e-4;

// Reverse-Z raw depth is proportional to 1/z, so the relative difference is scale invariant.
float depthWeight(float centerDepth, float tapDepth) {
    if (centerDepth == DEPTH_FAR && tapDepth == DEPTH_FAR) {
        return 1.0;
    }
    float relative = abs(centerDepth - tapDepth) / max(max(centerDepth, tapDepth), 1e-6);
    return 1.0 / (1.0 + DEPTH_WEIGHT_FALLOFF * relative);
}

[shader("fragment")]
float4 main([[vk::location(0)]] float2 fragTexCoord : TEXCOORD0) : SV_Target0 {
    uint halfWidth;
    uint halfHeight;
    windColorSampler.GetDimensions(halfWidth, halfHeight);
    float2 halfSize = float2(halfWidth, halfHeight);
    float2 texelCoord = fragTexCoord * halfSize - 0.5;
    float2 baseCoord = floor(texelCoord);
    float2 fraction = texelCoord - baseCoord;

    float centerDepth = sceneDepthSampler.Sample(fragTexCoord).r;
    float2 tapOffsets[4] = { float2(0.0, 0.0), float2(1.0, 0.0), float2(0.0, 1.0), float2(1.0, 1.0) };
    float4 bilinearWeights = float4(
        (1.0 - fraction.x) * (1.0 - fraction.y),
        fraction.x * (1.0 - fraction.y),
        (1.0 - fraction.x) * fraction.y,
        fraction.x * fraction.y);

    float4 accumulated = float4(0.0);
    float totalWeight = 0.0;
    float4 nearestColor = float4(0.0);
    float nearestWeight = -1.0;
    for (int tap = 0; tap < 4; ++tap) {
        float2 tapUv = (baseCoord + tapOffsets[tap] + 0.5) / halfSize;
        float4 tapColor = windColorSampler.Sample(tapUv);
        float tapDepth = sceneDepthSampler.Sample(tapUv).r;
        float weight = bilinearWeights[tap] * depthWeight(centerDepth, tapDepth);

        accumulated += tapColor * weight;
        totalWeight += weight;
        if (bilinearWeights[tap] > nearestWeight) {
            nearestWeight = bilinearWeights[tap];
            nearestColor = tapColor;
        }
    }

    return totalWeight > MIN_TOTAL_WEIGHT ? accumulated / totalWeight : nearestColor;
}
