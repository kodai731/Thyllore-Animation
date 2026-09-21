import "include/frame.slang";

struct MaterialUBO {
    float4 base_color;
    float metallic;
    float roughness;
    float2 _padding;
};

[[vk::binding(1, 1)]] ConstantBuffer<MaterialUBO> material;

[[vk::binding(0, 1)]] Sampler2D texSampler;

struct VSOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] float4 color : TEXCOORD0;
    [[vk::location(1)]] float2 texCoord : TEXCOORD1;
    [[vk::location(2)]] float3 worldPos : TEXCOORD2;
    [[vk::location(3)]] float3 normal : TEXCOORD3;
};

[shader("fragment")]
float4 main(VSOutput input) : SV_Target0 {
    float4 texColor = texSampler.Sample(input.texCoord);
    if (input.color.a < 0.5) discard;

    float3 lightDir = normalize(frame.light_pos.xyz - input.worldPos);
    float3 normal = normalize(input.normal);

    float ambient = 0.2;
    float diffuse = max(dot(normal, lightDir), 0.0);
    float lighting = ambient + diffuse * 0.8;

    float3 finalColor = texColor.rgb * input.color.rgb * material.base_color.rgb * lighting * frame.light_color.rgb;
    return float4(finalColor, 1.0);
}
