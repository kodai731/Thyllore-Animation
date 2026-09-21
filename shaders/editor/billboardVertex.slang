struct UniformBufferObject {
    float4x4 model;
    float4x4 view;
    float4x4 proj;
};

[[vk::binding(0, 0)]] ConstantBuffer<UniformBufferObject> ubo;

struct VSOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] float2 fragTexCoord : TEXCOORD0;
    [[vk::location(1)]] float3 fragBillboardWorldPos : TEXCOORD1;
    [[vk::location(2)]] float fragBillboardViewDepth : TEXCOORD2;
    [[vk::location(3)]] float4 fragClipPos : TEXCOORD3;
};

[shader("vertex")]
VSOutput main(
    [[vk::location(0)]] float3 inPosition : POSITION,
    [[vk::location(1)]] float2 inTexCoord : TEXCOORD) {
    VSOutput output;

    float4 worldPos = mul(ubo.model, float4(inPosition, 1.0));

    float4 viewPos = mul(ubo.view, worldPos);

    float4 clipPos = mul(ubo.proj, viewPos);

    output.position = clipPos;
    output.fragTexCoord = inTexCoord;
    output.fragBillboardWorldPos = worldPos.xyz;
    output.fragBillboardViewDepth = -viewPos.z;
    output.fragClipPos = clipPos;
    return output;
}
