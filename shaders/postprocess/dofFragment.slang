import "include/math.slang";

[[vk::binding(0, 0)]] Sampler2D hdrSampler;
[[vk::binding(1, 0)]] Sampler2D depthSampler;

struct PushConstants {
    float focalLengthMm;
    float apertureFStops;
    float sensorHeightMm;
    float focusDistance;
    float nearPlane;
    float maxBlurRadius;
    float viewportHeight;
    int enabled;
};

[[vk::push_constant]] ConstantBuffer<PushConstants> pc;

float linearizeDepth(float depth) {
    return pc.nearPlane / max(depth, 0.0001);
}

float computeCoC(float depth) {
    float focalLength = pc.focalLengthMm * 0.001;
    float apertureDiameter = focalLength / max(pc.apertureFStops, 0.01);

    float numerator = apertureDiameter * focalLength * (pc.focusDistance - depth);
    float denominator = depth * (pc.focusDistance - focalLength);

    float cocWorld = abs(numerator / max(abs(denominator), 0.0001));
    float cocPixels = cocWorld * (pc.viewportHeight / (pc.sensorHeightMm * 0.001));

    return clamp(cocPixels, 0.0, pc.maxBlurRadius);
}

[shader("fragment")]
float4 main([[vk::location(0)]] float2 fragTexCoord : TEXCOORD0) : SV_Target0 {
    float4 hdrColor = hdrSampler.Sample(fragTexCoord);

    if (pc.enabled == 0) {
        return hdrColor;
    }

    float depthValue = depthSampler.Sample(fragTexCoord).r;
    float linearDepth = linearizeDepth(depthValue);
    float coc = computeCoC(linearDepth);

    if (coc < 0.5) {
        return hdrColor;
    }

    uint width;
    uint height;
    hdrSampler.GetDimensions(width, height);
    float2 texelSize = 1.0 / float2(width, height);
    float radius = min(coc, pc.maxBlurRadius);
    int sampleCount = int(clamp(radius * 2.0, 4.0, 32.0));

    float3 accumColor = float3(0.0);
    float totalWeight = 0.0;
    float angleStep = TWO_PI / float(sampleCount);

    for (int ring = 1; ring <= 3; ring++) {
        float ringRadius = radius * float(ring) / 3.0;
        for (int i = 0; i < sampleCount; i++) {
            float angle = float(i) * angleStep + float(ring) * 0.5;
            float2 offset = float2(cos(angle), sin(angle)) * ringRadius * texelSize;
            float2 sampleUV = fragTexCoord + offset;

            float sampleDepth = depthSampler.Sample(sampleUV).r;
            float sampleLinearDepth = linearizeDepth(sampleDepth);
            float sampleCoC = computeCoC(sampleLinearDepth);

            float weight = smoothstep(0.0, 1.0, sampleCoC / max(radius, 0.001));
            accumColor += hdrSampler.Sample(sampleUV).rgb * weight;
            totalWeight += weight;
        }
    }

    float centerWeight = 1.0 - smoothstep(0.0, 2.0, coc);
    accumColor += hdrColor.rgb * centerWeight;
    totalWeight += centerWeight;

    float3 blurredColor = accumColor / max(totalWeight, 0.001);
    float blendFactor = smoothstep(0.0, 2.0, coc);

    return float4(lerp(hdrColor.rgb, blurredColor, blendFactor), hdrColor.a);
}
