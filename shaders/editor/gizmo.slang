import "include/frame.slang";

struct ObjectUBO {
    float4x4 model;
};

[[vk::binding(0, 2)]] ConstantBuffer<ObjectUBO> object;

struct Varyings {
    float4 position : SV_Position;
    [[vk::location(0)]] float3 fragColor : TEXCOORD0;
};

[shader("vertex")]
Varyings vertexMain(
    [[vk::location(0)]] float3 inPosition : POSITION,
    [[vk::location(1)]] float3 inColor : COLOR) {
    Varyings output;

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

[shader("fragment")]
float4 fragmentMain(Varyings input) : SV_Target0 {
    return float4(input.fragColor, 1.0);
}
