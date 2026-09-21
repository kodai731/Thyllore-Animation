struct VSOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] float3 fragColor : TEXCOORD0;
    [[vk::location(1)]] float worldY : TEXCOORD1;
};

[shader("fragment")]
float4 main(VSOutput input) : SV_Target0 {
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
