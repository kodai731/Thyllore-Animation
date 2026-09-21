struct VSOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] float3 fragColor : TEXCOORD0;
};

[shader("fragment")]
float4 main(VSOutput input) : SV_Target0 {
    return float4(input.fragColor, 1.0);
}
