import "include/resample.slang";

[[vk::binding(0, 0)]] Sampler2D inputSampler;

static const int UPSAMPLE_TENT_RADIUS = 1;

[shader("fragment")]
float4 main([[vk::location(0)]] float2 fragTexCoord : TEXCOORD0) : SV_Target0 {
    uint width;
    uint height;
    inputSampler.GetDimensions(width, height);
    float2 texelSize = 1.0 / float2(width, height);
    return float4(tentFilter(inputSampler, fragTexCoord, texelSize, UPSAMPLE_TENT_RADIUS), 1.0);
}
