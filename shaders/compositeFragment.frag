#version 450

layout(location = 0) in vec2 fragTexCoord;

layout(location = 0) out vec4 outColor;

// G-Buffer inputs (as sampled images)
layout(binding = 0) uniform sampler2D positionSampler;
layout(binding = 1) uniform sampler2D normalSampler;
layout(binding = 2) uniform sampler2D shadowMaskSampler;
layout(binding = 3) uniform sampler2D albedoSampler;

layout(binding = 4) uniform SceneData {
    vec4 lightPosition;
    vec4 lightColor;
    mat4 view;
    mat4 proj;
    int debugMode;
    float shadowStrength;
    int _padding[2];
} sceneData;

void main() {
    // Sample G-Buffer
    vec3 worldPosition = texture(positionSampler, fragTexCoord).xyz;
    vec3 worldNormal = texture(normalSampler, fragTexCoord).xyz;
    float shadowMask = texture(shadowMaskSampler, fragTexCoord).r;
    vec4 albedo = texture(albedoSampler, fragTexCoord);

    // Check if this is a valid fragment (normal length check)
    bool isBackground = length(worldNormal) < 0.01;

    if (isBackground) {
        // Background pixel - simple sky color
        outColor = vec4(0.5, 0.7, 1.0, 1.0);
        return;
    }

    // Debug view modes
    if (sceneData.debugMode == 1) {
        // Position view (world space) - visualize as color
        // Map world position to visible range
        vec3 posColor = worldPosition * 0.1 + 0.5;
        outColor = vec4(posColor, 1.0);
        return;
    }
    else if (sceneData.debugMode == 2) {
        // Normal view (world space) - map [-1,1] to [0,1]
        vec3 normalColor = normalize(worldNormal) * 0.5 + 0.5;
        outColor = vec4(normalColor, 1.0);
        return;
    }
    else if (sceneData.debugMode == 3) {
        // Shadow mask view - show shadow as grayscale
        outColor = vec4(vec3(shadowMask), 1.0);
        return;
    }

    worldNormal = normalize(worldNormal);

    vec3 lightDir = normalize(sceneData.lightPosition.xyz - worldPosition);

    float diffuse = max(dot(worldNormal, lightDir), 0.0);

    float ambient = 0.2;

    float shadowFactor = mix(1.0, shadowMask, sceneData.shadowStrength);

    vec3 baseColor = albedo.rgb;
    vec3 lighting = (ambient + diffuse * shadowFactor) * sceneData.lightColor.rgb;
    vec3 finalColor = baseColor * lighting;

    outColor = vec4(finalColor, 1.0);
}
