import "water/include/component.slang";

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
[[vk::binding(1, 0)]] [format("rgba16f")] RWTexture2D<float4> hdrColorImage;
[[vk::binding(2, 0)]] ConstantBuffer<SceneData> sceneData;
[[vk::binding(3, 0)]] ConstantBuffer<WaterUBO> waterBlock;

static const uint TENT_WEIGHTS[5] = { 1u, 2u, 3u, 2u, 1u };

uint sampleTentFiltered(int2 pixel, int2 size) {
    uint weightedSum = 0u;
    for (int dy = -2; dy <= 2; ++dy) {
        for (int dx = -2; dx <= 2; ++dx) {
            int2 tap = clamp(pixel + int2(dx, dy), int2(0), size - int2(1));
            weightedSum += causticAccumImage[tap] * TENT_WEIGHTS[dy + 2] * TENT_WEIGHTS[dx + 2];
        }
    }
    return weightedSum;
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 globalInvocationId : SV_DispatchThreadID) {
    water = waterBlock;
    int2 pixel = int2(globalInvocationId.xy);
    uint width, height;
    hdrColorImage.GetDimensions(width, height);
    int2 size = int2(width, height);
    if (any(pixel >= size)) {
        return;
    }

    float caustic = min(float(sampleTentFiltered(pixel, size)) * 0.25 / (81.0 * 256.0), 1.5);
    float3 addition = caustic * clamp(sceneData.lightColor.rgb, float3(0.0), float3(1.0))
        * clamp(water.radii.z, 0.0, 2.0) * 3.0;

    float4 current = hdrColorImage[pixel];
    hdrColorImage[pixel] = float4(current.rgb + addition, current.a);
}
