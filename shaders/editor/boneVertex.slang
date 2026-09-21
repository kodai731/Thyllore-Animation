import "include/frame.slang";

struct ObjectUBO {
    float4x4 model;
};

[[vk::binding(0, 2)]] ConstantBuffer<ObjectUBO> object;

struct VSOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] float3 fragColor : TEXCOORD0;
};

[shader("vertex")]
VSOutput main(
    [[vk::location(0)]] float3 inPosition : POSITION,
    [[vk::location(1)]] float3 inColor : COLOR) {
    VSOutput output;

    float4 worldPos = mul(object.model, float4(inPosition, 1.0));
    output.position = mul(mul(frame.proj, frame.view), worldPos);
    output.fragColor = inColor;
    return output;
}
