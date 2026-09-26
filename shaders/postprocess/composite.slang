import "include/outline.slang";
import "include/ray.slang";
import "include/lighting.slang";
import "include/scene.slang";

struct Varyings {
    float4 position : SV_Position;
    [[vk::location(0)]] float2 fragTexCoord : TEXCOORD0;
};

[shader("vertex")]
Varyings vertexMain(uint vertexIndex : SV_VertexID) {
    Varyings output;

    float2 uv = float2((vertexIndex << 1) & 2, vertexIndex & 2);
    output.fragTexCoord = uv;
    output.position = float4(uv * 2.0 - 1.0, 0.0, 1.0);
    return output;
}

[[vk::binding(0, 0)]] Sampler2D positionSampler;
[[vk::binding(1, 0)]] Sampler2D normalSampler;
[[vk::binding(2, 0)]] Sampler2D shadowMaskSampler;
[[vk::binding(3, 0)]] Sampler2D albedoSampler;

[[vk::binding(4, 0)]] ConstantBuffer<SceneData> sceneData;

[[vk::binding(5, 0)]] Sampler2D<uint4> objectIDSampler;

struct SelectionData {
    uint4 selectedIDs[32];
    uint selectedCount;
    uint _pad0;
    uint _pad1;
    uint _pad2;
};

[[vk::binding(6, 0)]] ConstantBuffer<SelectionData> selection;

struct PushConstants {
    int debugMode;
};

[[vk::push_constant]] ConstantBuffer<PushConstants> pc;

static const float3 OUTLINE_COLOR = float3(1.0, 0.5, 0.0);
static const float OUTLINE_WIDTH = 1.5;

struct SelectedEntities : IPixelClass {
    bool contains(uint id) {
        if (id == 0u) return false;
        for (uint i = 0u; i < selection.selectedCount; i++) {
            if (selection.selectedIDs[i].x == id) return true;
        }
        return false;
    }
};

bool onSelectionOutline(float2 fragTexCoord) {
    uint width;
    uint height;
    objectIDSampler.GetDimensions(width, height);
    float2 texelSize = 1.0 / float2(width, height);
    SelectedEntities selected;
    return detectClassEdge(selected, objectIDSampler, fragTexCoord, texelSize, OUTLINE_WIDTH);
}

[shader("fragment")]
float4 fragmentMain(Varyings input, out float fragDepth : SV_Depth) : SV_Target0 {
    float2 fragTexCoord = input.fragTexCoord;
    // Sample G-Buffer
    float3 worldPosition = positionSampler.Sample(fragTexCoord).xyz;
    float3 worldNormal = normalSampler.Sample(fragTexCoord).xyz;
    float shadowMask = shadowMaskSampler.Sample(fragTexCoord).r;
    float4 albedo = albedoSampler.Sample(fragTexCoord);

    // Check if this is a valid fragment (normal length check)
    bool isBackground = length(worldNormal) < 0.01;

    // Write scene depth for overlay depth testing
    if (isBackground) {
        fragDepth = DEPTH_FAR;
    } else {
        fragDepth = worldToClipDepth(worldPosition, sceneData.view, sceneData.proj);
    }

    // No opaque surface here: leave the target untouched so the pass clear color,
    // which carries the background radiance, stays as the scene behind volumetrics.
    if (isBackground) {
        discard;
    }

    // Debug view modes
    if (pc.debugMode == 1) {
        // Position view (world space) - visualize as color
        // Map world position to visible range
        float3 posColor = worldPosition * 0.1 + 0.5;
        return float4(posColor, 1.0);
    }
    else if (pc.debugMode == 2) {
        // Normal view (world space) - map [-1,1] to [0,1]
        float3 normalColor = normalize(worldNormal) * 0.5 + 0.5;
        return float4(normalColor, 1.0);
    }
    else if (pc.debugMode == 3) {
        // Shadow mask view - show shadow as grayscale
        return float4(float3(shadowMask), 1.0);
    }
    else if (pc.debugMode == 4) {
        // N dot L view - visualize light direction relative to surface
        float3 n = normalize(worldNormal);
        float3 lightVector = sceneData.lightPosition.xyz - worldPosition;
        float3 l = normalize(lightVector);
        float ndotl = dot(n, l);
        // Red = facing away from light, Green = facing toward light
        float3 ndotlColor = ndotl > 0.0 ? float3(0.0, ndotl, 0.0) : float3(-ndotl, 0.0, 0.0);
        return float4(ndotlColor, 1.0);
    }
    else if (pc.debugMode == 5) {
        // Light direction view - show raw light direction as color
        float3 lightVector = sceneData.lightPosition.xyz - worldPosition;
        float3 l = normalize(lightVector);
        return float4(l * 0.5 + 0.5, 1.0);
    }
    else if (pc.debugMode == 6) {
        // View depth mode - visualize GBuffer depth in view space
        float4 worldPos4 = positionSampler.Sample(fragTexCoord);
        bool hasGeometry = worldPos4.w > 0.5;

        if (!hasGeometry) {
            return float4(0.0, 0.0, 0.2, 1.0);
        }

        float4 viewPos = mul(sceneData.view, float4(worldPos4.xyz, 1.0));
        float viewDepth = -viewPos.z;

        float normalizedDepth = viewDepth * 0.005;
        normalizedDepth = clamp(normalizedDepth, 0.0, 1.0);

        return float4(0.0, normalizedDepth, 0.0, 1.0);
    }
    else if (pc.debugMode == 7) {
        uint objectID = objectIDSampler.Sample(fragTexCoord).r;
        if (objectID == 0u) {
            return float4(0.0, 0.0, 0.0, 1.0);
        } else {
            float r = float((objectID * 37u) % 256u) / 255.0;
            float g = float((objectID * 97u) % 256u) / 255.0;
            float b = float((objectID * 151u) % 256u) / 255.0;
            return float4(r, g, b, 1.0);
        }
    }
    else if (pc.debugMode == 8) {
        uint objectID = objectIDSampler.Sample(fragTexCoord).r;

        if (selection.selectedCount > 0u && objectID == selection.selectedIDs[0].x) {
            return float4(1.0, 0.5, 0.0, 1.0);
        } else if (objectID > 0u) {
            return float4(0.3, 0.3, 0.3, 1.0);
        } else {
            return float4(0.0, 0.0, 0.0, 1.0);
        }
    }
    else if (pc.debugMode == 9) {
        float countVis = float(selection.selectedCount) / 5.0;
        return float4(countVis, countVis, countVis, 1.0);
    }
    else if (pc.debugMode == 10) {
        return float4(albedo.rgb, 1.0);
    }

    worldNormal = normalize(worldNormal);

    float3 lightVector = sceneData.lightPosition.xyz - worldPosition;
    float lightDistance = length(lightVector);
    float3 lightDir = normalize(lightVector);
    float3 viewDir = normalize(sceneData.cameraPosition.xyz - worldPosition);

    float shadowFactor = lerp(1.0, shadowMask, sceneData.shadowStrength);

    float3 baseColor = albedo.rgb;
    float metallic = albedo.a;
    float roughness = normalSampler.Sample(fragTexCoord).w;

    SurfaceSample surface;
    surface.baseColor = baseColor;
    surface.metallic = metallic;
    surface.roughness = roughness;
    surface.normal = worldNormal;

    float attenuation = sceneData.enableDistanceAttenuation != 0 ? distanceAttenuation(lightDistance, sceneData.lighting.y, sceneData.lighting.z) : 1.0;
    float3 radiance = sceneData.lightColor.rgb * attenuation * shadowFactor;

    float3 finalColor = shadeSurface(surface, lightDir, viewDir, radiance, sceneData.lighting.x);

    if (selection.selectedCount > 0u && onSelectionOutline(fragTexCoord)) {
        return float4(OUTLINE_COLOR, 1.0);
    } else {
        return float4(finalColor, 1.0);
    }
}
