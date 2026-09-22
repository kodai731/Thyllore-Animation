import "include/resample.slang";

[[vk::binding(0, 0)]] Sampler2D inputSampler;

struct PushConstants {
    float threshold;
    float knee;
    int isFirstPass;
};

[[vk::push_constant]] ConstantBuffer<PushConstants> pc;

float2 inputTexelSize() {
    uint width;
    uint height;
    inputSampler.GetDimensions(width, height);
    return 1.0 / float2(width, height);
}

float3 applyThreshold(float3 color) {
    float brightness = max(max(color.r, color.g), color.b);
    float soft = brightness - pc.threshold + pc.knee;
    soft = clamp(soft, 0.0, 2.0 * pc.knee);
    soft = soft * soft / (4.0 * pc.knee + 0.00001);
    float contribution = max(soft, brightness - pc.threshold);
    contribution /= max(brightness, 0.00001);
    return color * contribution;
}

[shader("fragment")]
float4 main([[vk::location(0)]] float2 fragTexCoord : TEXCOORD0) : SV_Target0 {
    if (pc.isFirstPass != 0) {
        float3 color = downsample13TapKaris(inputSampler, fragTexCoord, inputTexelSize());
        return float4(applyThreshold(color), 1.0);
    }
    return float4(downsample13Tap(inputSampler, fragTexCoord, inputTexelSize()), 1.0);
}
