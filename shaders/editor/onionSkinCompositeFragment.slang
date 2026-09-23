[[vk::binding(0, 0)]] Sampler2D ghostSampler;

[shader("fragment")]
float4 main([[vk::location(0)]] float2 fragTexCoord : TEXCOORD0) : SV_Target0 {
    return ghostSampler.Sample(fragTexCoord);
}
