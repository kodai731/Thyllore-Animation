struct MaterialUBO {
    float4 base_color;
    float metallic;
    float roughness;
    float2 _padding;
};

[[vk::binding(1, 1)]] ConstantBuffer<MaterialUBO> material;

[[vk::binding(0, 1)]] Sampler2D texSampler;

struct WindPush {
    uint objectID;
    uint heatmapMode;
};

[[vk::push_constant]] ConstantBuffer<WindPush> pc;

struct VSOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] float3 worldPos : TEXCOORD0;
    [[vk::location(1)]] float3 worldNormal : TEXCOORD1;
    [[vk::location(2)]] float2 texCoord : TEXCOORD2;
    [[vk::location(3)]] float4 color : TEXCOORD3;
};

[shader("fragment")]
void main(VSOutput input, out float4 outPosition : SV_Target0, out float4 outNormal : SV_Target1, out float4 outAlbedo : SV_Target2, out uint outObjectID : SV_Target3) {
    float4 texColor = texSampler.Sample(input.texCoord);
    if (input.color.a < 0.5) discard;

    float3 albedoRGB;
    if (pc.heatmapMode == 1u) {
        albedoRGB = input.color.rgb;
    } else {
        albedoRGB = texColor.rgb * input.color.rgb * material.base_color.rgb;
    }

    outPosition = float4(input.worldPos, 1.0);
    outNormal = float4(normalize(input.worldNormal), material.roughness);
    outAlbedo = float4(albedoRGB, material.metallic);
    outObjectID = pc.objectID;
}
