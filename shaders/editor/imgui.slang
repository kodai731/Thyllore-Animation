struct PushConstants {
    float2 scale;
    float2 translate;
};

[[vk::push_constant]] ConstantBuffer<PushConstants> pushConstants;
[[vk::binding(0, 0)]] Sampler2D texSampler;

struct Varyings {
    float4 position : SV_Position;
    [[vk::location(0)]] float2 fragUV : TEXCOORD0;
    [[vk::location(1)]] float4 fragColor : TEXCOORD1;
};

[shader("vertex")]
Varyings vertexMain(
    [[vk::location(0)]] float2 inPosition : POSITION,
    [[vk::location(1)]] float2 inUV : TEXCOORD,
    [[vk::location(2)]] float4 inColor : COLOR) {
    Varyings output;

    output.fragUV = inUV;
    output.fragColor = inColor;
    output.position = float4(inPosition * pushConstants.scale + pushConstants.translate, 0.0, 1.0);
    return output;
}

[shader("fragment")]
float4 fragmentMain(Varyings input) : SV_Target0 {
    return input.fragColor * texSampler.Sample(input.fragUV);
}
