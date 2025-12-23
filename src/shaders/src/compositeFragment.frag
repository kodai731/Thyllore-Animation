#version 450

layout(location = 0) in vec2 fragTexCoord;

layout(location = 0) out vec4 outColor;

// G-Buffer inputs (as sampled images)
layout(binding = 0) uniform sampler2D positionSampler;
layout(binding = 1) uniform sampler2D normalSampler;
layout(binding = 2) uniform sampler2D shadowMaskSampler;

// Scene data (light information and debug mode)
layout(binding = 3) uniform SceneData {
    vec4 lightPosition;    // Light position (w component unused)
    vec4 lightColor;       // Light color and intensity
    mat4 view;             // View matrix (for debugging)
    mat4 proj;             // Projection matrix (for debugging)
    int debugMode;         // Debug view mode (0=Final, 1=Position, 2=Normal, 3=Shadow)
    int _padding[3];       // Padding for alignment
} sceneData;

void main() {
    // Sample G-Buffer
    vec3 worldPosition = texture(positionSampler, fragTexCoord).xyz;
    vec3 worldNormal = texture(normalSampler, fragTexCoord).xyz;
    float shadowMask = texture(shadowMaskSampler, fragTexCoord).r;

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

    // Final rendering mode (debugMode == 0)
    // Normalize normal
    worldNormal = normalize(worldNormal);

    // Calculate lighting direction
    vec3 lightDir = normalize(sceneData.lightPosition.xyz - worldPosition);

    // Simple diffuse lighting
    float diffuse = max(dot(worldNormal, lightDir), 0.0);

    // Ambient lighting
    float ambient = 0.2;

    // Combine lighting with shadow
    vec3 baseColor = vec3(0.8, 0.8, 0.8); // Simple gray material
    vec3 lighting = (ambient + diffuse * shadowMask) * sceneData.lightColor.rgb;
    vec3 finalColor = baseColor * lighting;

    outColor = vec4(finalColor, 1.0);
}
