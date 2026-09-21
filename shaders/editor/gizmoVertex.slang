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

    float2 gizmoOffset = float2(0.85, -0.75);
    float gizmoScale = 1.0;

    float aspect = abs(frame.proj[1][1]) / frame.proj[0][0];

    float3x3 viewRotation = float3x3(frame.view);
    float3 rotatedPos = mul(viewRotation, inPosition);

    float3 flippedPos = float3(rotatedPos.x / aspect, -rotatedPos.y, rotatedPos.z);
    float4 position = float4(flippedPos * gizmoScale, 1.0);
    position.xy += gizmoOffset;
    position.z = 0.0;

    output.position = position;
    output.fragColor = inColor;
    return output;
}
