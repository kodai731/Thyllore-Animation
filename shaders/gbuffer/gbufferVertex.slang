import "include/frame.slang";

struct ObjectUBO {
    float4x4 model;
};

[[vk::binding(0, 2)]] ConstantBuffer<ObjectUBO> object;

struct VSOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] float3 worldPos : TEXCOORD0;
    [[vk::location(1)]] float3 worldNormal : TEXCOORD1;
    [[vk::location(2)]] float2 texCoord : TEXCOORD2;
    [[vk::location(3)]] float4 color : TEXCOORD3;
};

[shader("vertex")]
VSOutput main(
    [[vk::location(0)]] float3 inPosition : POSITION,
    [[vk::location(1)]] float4 inColor : COLOR,
    [[vk::location(2)]] float2 inTexCoord : TEXCOORD,
    [[vk::location(3)]] float3 inNormal : NORMAL) {
    VSOutput output;

    float4 worldPos = mul(object.model, float4(inPosition, 1.0));
    output.worldPos = worldPos.xyz;

    output.worldNormal = normalize(mul(float3x3(object.model), inNormal));

    output.texCoord = inTexCoord;
    output.color = inColor;

    output.position = mul(frame.proj, mul(frame.view, worldPos));
    return output;
}
