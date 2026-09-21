struct VSOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] float2 fragUV : TEXCOORD0;
    [[vk::location(1)]] float4 fragColor : TEXCOORD1;
};

[[vk::binding(0, 0)]] Sampler2D texSampler;

[shader("fragment")]
float4 main(VSOutput input) : SV_Target0 {
    return input.fragColor * texSampler.Sample(input.fragUV);
}
