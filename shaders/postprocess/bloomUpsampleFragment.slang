[[vk::binding(0, 0)]] Sampler2D inputSampler;

float3 upsample9Tap(float2 uv) {
    uint width;
    uint height;
    inputSampler.GetDimensions(width, height);
    float2 texelSize = 1.0 / float2(width, height);

    float3 a = inputSampler.Sample(uv + texelSize * float2(-1.0, -1.0)).rgb;
    float3 b = inputSampler.Sample(uv + texelSize * float2( 0.0, -1.0)).rgb;
    float3 c = inputSampler.Sample(uv + texelSize * float2( 1.0, -1.0)).rgb;
    float3 d = inputSampler.Sample(uv + texelSize * float2(-1.0,  0.0)).rgb;
    float3 e = inputSampler.Sample(uv).rgb;
    float3 f = inputSampler.Sample(uv + texelSize * float2( 1.0,  0.0)).rgb;
    float3 g = inputSampler.Sample(uv + texelSize * float2(-1.0,  1.0)).rgb;
    float3 h = inputSampler.Sample(uv + texelSize * float2( 0.0,  1.0)).rgb;
    float3 i = inputSampler.Sample(uv + texelSize * float2( 1.0,  1.0)).rgb;

    float3 result = e * 4.0;
    result += (b + d + f + h) * 2.0;
    result += (a + c + g + i);
    result *= (1.0 / 16.0);

    return result;
}

[shader("fragment")]
float4 main([[vk::location(0)]] float2 fragTexCoord : TEXCOORD0) : SV_Target0 {
    return float4(upsample9Tap(fragTexCoord), 1.0);
}
