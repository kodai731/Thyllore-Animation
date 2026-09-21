[[vk::binding(0, 0)]] Sampler2D inputSampler;

struct PushConstants {
    float threshold;
    float knee;
    int isFirstPass;
};

[[vk::push_constant]] ConstantBuffer<PushConstants> pc;

float3 karisAverage(float3 c) {
    float luma = dot(c, float3(0.2126, 0.7152, 0.0722));
    return c / (1.0 + luma);
}

float3 downsample13Tap(float2 uv) {
    uint width;
    uint height;
    inputSampler.GetDimensions(width, height);
    float2 texelSize = 1.0 / float2(width, height);

    float3 a = inputSampler.Sample(uv + texelSize * float2(-1.0, -1.0)).rgb;
    float3 b = inputSampler.Sample(uv + texelSize * float2( 0.0, -1.0)).rgb;
    float3 c = inputSampler.Sample(uv + texelSize * float2( 1.0, -1.0)).rgb;
    float3 d = inputSampler.Sample(uv + texelSize * float2(-0.5, -0.5)).rgb;
    float3 e = inputSampler.Sample(uv).rgb;
    float3 f = inputSampler.Sample(uv + texelSize * float2( 0.5, -0.5)).rgb;
    float3 g = inputSampler.Sample(uv + texelSize * float2(-1.0,  0.0)).rgb;
    float3 h = inputSampler.Sample(uv + texelSize * float2( 1.0,  0.0)).rgb;
    float3 i = inputSampler.Sample(uv + texelSize * float2(-0.5,  0.5)).rgb;
    float3 j = inputSampler.Sample(uv + texelSize * float2( 0.5,  0.5)).rgb;
    float3 k = inputSampler.Sample(uv + texelSize * float2(-1.0,  1.0)).rgb;
    float3 l = inputSampler.Sample(uv + texelSize * float2( 0.0,  1.0)).rgb;
    float3 m = inputSampler.Sample(uv + texelSize * float2( 1.0,  1.0)).rgb;

    if (pc.isFirstPass != 0) {
        float3 g0 = (a + b + g + d) * 0.25;
        float3 g1 = (b + c + d + f) * 0.25;
        float3 g2 = (g + d + k + i) * 0.25;
        float3 g3 = (d + f + i + j) * 0.25;
        float3 g4 = e;

        g0 = karisAverage(g0);
        g1 = karisAverage(g1);
        g2 = karisAverage(g2);
        g3 = karisAverage(g3);
        g4 = karisAverage(g4);

        return g0 * 0.125 + g1 * 0.125 + g2 * 0.125 + g3 * 0.125 + g4 * 0.5;
    }

    float3 result = e * 0.125;
    result += (b + g + h + l) * 0.0625;
    result += (d + f + i + j) * 0.125;
    result += (a + c + k + m) * 0.03125;

    return result;
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
    float3 color = downsample13Tap(fragTexCoord);

    if (pc.isFirstPass != 0) {
        color = applyThreshold(color);
    }

    return float4(color, 1.0);
}
