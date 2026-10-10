import "include/frame.slang";

struct ObjectUBO {
    float4x4 model;
};

struct PushConstants {
    float alpha;
};

[[vk::binding(0, 2)]] ConstantBuffer<ObjectUBO> object;
[[vk::push_constant]] ConstantBuffer<PushConstants> pc;

struct Varyings {
    float4 position : SV_Position;
    [[vk::location(0)]] float3 fragColor : TEXCOORD0;
};

[shader("vertex")]
Varyings vertexMain(
    [[vk::location(0)]] float3 inPosition : POSITION,
    [[vk::location(1)]] float3 inColor : COLOR) {
    Varyings output;

    float4 worldPos = mul(object.model, float4(inPosition, 1.0));
    output.position = mul(mul(frame.proj, frame.view), worldPos);
    output.fragColor = inColor;
    return output;
}

[shader("fragment")]
float4 fragmentMain(Varyings input) : SV_Target0 {
    return float4(input.fragColor, pc.alpha);
}
