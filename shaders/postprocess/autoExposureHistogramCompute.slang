[[vk::binding(0, 0)]] Sampler2D hdrImage;
[[vk::binding(1, 0)]] RWStructuredBuffer<uint> histogram;

struct PushConstants {
    float minLogLuminance;
    float logLuminanceRange;
    uint pixelCount;
};

[[vk::push_constant]] ConstantBuffer<PushConstants> pc;

groupshared uint sharedHistogram[256];

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 globalInvocationId : SV_DispatchThreadID, uint localIndex : SV_GroupIndex) {
    sharedHistogram[localIndex] = 0;
    GroupMemoryBarrierWithGroupSync();

    uint2 pixelCoord = globalInvocationId.xy;
    int2 imageSize;
    hdrImage.GetDimensions(imageSize.x, imageSize.y);

    if (pixelCoord.x < imageSize.x && pixelCoord.y < imageSize.y) {
        float2 uv = (float2(pixelCoord) + 0.5) / float2(imageSize);
        float3 color = hdrImage.SampleLevel(uv, 0).rgb;
        float luminance = dot(color, float3(0.2126, 0.7152, 0.0722));

        uint binIndex = 0;
        if (luminance > 0.001) {
            float logLum = log2(luminance);
            float normalized = clamp(
                (logLum - pc.minLogLuminance) / pc.logLuminanceRange,
                0.0,
                1.0
            );
            binIndex = uint(normalized * 254.0 + 1.0);
        }

        InterlockedAdd(sharedHistogram[binIndex], 1);
    }

    GroupMemoryBarrierWithGroupSync();

    InterlockedAdd(histogram[localIndex], sharedHistogram[localIndex]);
}
