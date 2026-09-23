import "include/frame.slang";

struct ObjectUBO {
    float4x4 model;
};

struct MaterialUBO {
    float4 base_color;
    float metallic;
    float roughness;
    float2 _padding;
};

[[vk::binding(0, 2)]] ConstantBuffer<ObjectUBO> object;
[[vk::binding(1, 1)]] ConstantBuffer<MaterialUBO> material;
[[vk::binding(0, 1)]] Sampler2D texSampler;

struct Varyings {
    float4 position : SV_Position;
    [[vk::location(0)]] float4 color : TEXCOORD0;
    [[vk::location(1)]] float2 texCoord : TEXCOORD1;
    [[vk::location(2)]] float3 worldPos : TEXCOORD2;
    [[vk::location(3)]] float3 normal : TEXCOORD3;
};

[shader("vertex")]
Varyings vertexMain(
    [[vk::location(0)]] float3 inPosition : POSITION,
    [[vk::location(1)]] float4 inColor : COLOR,
    [[vk::location(2)]] float2 inTexCoord : TEXCOORD,
    [[vk::location(3)]] float3 inNormal : NORMAL) {
    Varyings output;

    float4 worldPos = mul(object.model, float4(inPosition, 1.0));
    output.position = mul(mul(frame.proj, frame.view), worldPos);

    output.color = inColor;
    output.texCoord = inTexCoord;
    output.worldPos = worldPos.xyz;
    output.normal = normalize(mul(float3x3(object.model), inNormal));

    return output;
}

[shader("fragment")]
float4 fragmentMain(Varyings input) : SV_Target0 {
    float4 texColor = texSampler.Sample(input.texCoord);
    if (input.color.a < 0.5) discard;

    float3 lightDir = normalize(frame.light_pos.xyz - input.worldPos);
    float3 normal = normalize(input.normal);

    float ambient = frame.lighting.x;
    float diffuse = max(dot(normal, lightDir), 0.0);
    float lighting = ambient + diffuse * (1.0 - ambient);

    float3 finalColor = texColor.rgb * input.color.rgb * material.base_color.rgb * lighting * frame.light_color.rgb;
    return float4(finalColor, 1.0);
}
