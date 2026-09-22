import "include/frame.slang";

struct ObjectUBO {
    float4x4 model;
};

[[vk::binding(0, 2)]] ConstantBuffer<ObjectUBO> object;

struct Varyings {
    float4 position : SV_Position;
    [[vk::location(0)]] float3 fragColor : TEXCOORD0;
    [[vk::location(1)]] float worldY : TEXCOORD1;
};

[shader("vertex")]
Varyings vertexMain(
    [[vk::location(0)]] float3 inPosition : POSITION,
    [[vk::location(1)]] float3 inColor : COLOR) {
    Varyings output;

    float4 worldPos = mul(object.model, float4(inPosition, 1.0));
    output.position = mul(mul(frame.proj, frame.view), worldPos);
    output.fragColor = inColor;
    output.worldY = worldPos.y;
    return output;
}

[shader("fragment")]
float4 fragmentMain(Varyings input) : SV_Target0 {
    bool isYAxisGrid = (input.fragColor.g > 0.9 && input.fragColor.r < 0.1 && input.fragColor.b < 0.1);

    if (isYAxisGrid) {
        if (input.worldY >= 0.0) {
            return float4(1.0, 1.0, 0.0, 1.0);
        } else {
            return float4(0.0, 1.0, 1.0, 1.0);
        }
    } else {
        return float4(input.fragColor, 1.0);
    }
}
