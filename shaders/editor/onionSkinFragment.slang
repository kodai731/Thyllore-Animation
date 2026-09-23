struct PushConstants {
    float ghostTintR;
    float ghostTintG;
    float ghostTintB;
    float ghostOpacity;
    int debugMode;
};

[[vk::push_constant]] ConstantBuffer<PushConstants> pc;

struct VSOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] float3 fragWorldPos : TEXCOORD0;
    [[vk::location(1)]] float3 fragWorldNormal : TEXCOORD1;
    [[vk::location(2)]] float2 fragTexCoord : TEXCOORD2;
    [[vk::location(3)]] float4 fragColor : TEXCOORD3;
};

[shader("fragment")]
float4 main(VSOutput input) : SV_Target0 {
    if (pc.debugMode == 1) {
        float3 normalizedPos = frac(input.fragWorldPos * 0.5 + 0.5);
        return float4(normalizedPos, 1.0);
    }

    if (pc.debugMode == 2) {
        float depth = input.position.z;
        return float4(depth, depth, depth, 1.0);
    }

    if (pc.debugMode == 3) {
        float3 n = normalize(input.fragWorldNormal) * 0.5 + 0.5;
        return float4(n, 1.0);
    }

    float3 ghostColor = float3(pc.ghostTintR, pc.ghostTintG, pc.ghostTintB);
    return float4(ghostColor, pc.ghostOpacity);
}
