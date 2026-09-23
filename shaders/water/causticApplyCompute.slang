import "include/resample.slang";
import "include/scene.slang";
import "water/include/caustic.slang";
import "water/include/component.slang";

[[vk::binding(0, 0)]] [format("r32ui")] RWTexture2D<uint> causticAccumImage;
[[vk::binding(1, 0)]] [format("rgba16f")] RWTexture2D<float4> hdrColorImage;
[[vk::binding(2, 0)]] ConstantBuffer<SceneData> sceneData;
[[vk::binding(3, 0)]] ConstantBuffer<WaterUBO> waterBlock;

uint sampleTentFiltered(int2 pixel, int2 size) {
    uint weightedSum = 0u;
    for (int dy = -CAUSTIC_FILTER_RADIUS; dy <= CAUSTIC_FILTER_RADIUS; ++dy) {
        for (int dx = -CAUSTIC_FILTER_RADIUS; dx <= CAUSTIC_FILTER_RADIUS; ++dx) {
            int2 tap = clamp(pixel + int2(dx, dy), int2(0), size - int2(1));
            weightedSum += causticAccumImage[tap]
                * tentWeight(dy, CAUSTIC_FILTER_RADIUS) * tentWeight(dx, CAUSTIC_FILTER_RADIUS);
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

    float normalization = float(tentWeightSum(CAUSTIC_FILTER_RADIUS)) * CAUSTIC_ACCUM_FIXED_POINT_SCALE;
    float caustic = min(float(sampleTentFiltered(pixel, size)) * CAUSTIC_INTENSITY / normalization, CAUSTIC_MAX_IRRADIANCE);
    float3 addition = caustic * clamp(sceneData.lightColor.rgb, float3(0.0), float3(1.0))
        * clamp(water.radii.z, 0.0, CAUSTIC_TORUS_RADIUS_GAIN_MAX) * CAUSTIC_LIGHT_GAIN;

    float4 current = hdrColorImage[pixel];
    hdrColorImage[pixel] = float4(current.rgb + addition, current.a);
}
