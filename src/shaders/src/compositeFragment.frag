#version 450

layout(location = 0) in vec2 fragTexCoord;

layout(location = 0) out vec4 outColor;

// G-Buffer inputs (as sampled images)
layout(binding = 0) uniform sampler2D positionSampler;
layout(binding = 1) uniform sampler2D normalSampler;
layout(binding = 2) uniform sampler2D shadowMaskSampler;

// Scene data (light information)
layout(binding = 3) uniform SceneData {
    vec4 lightPosition;    // Light position (w component unused)
    vec4 lightColor;       // Light color and intensity
    mat4 view;             // View matrix (for debugging)
    mat4 proj;             // Projection matrix (for debugging)
} sceneData;

void main() {
    // Sample G-Buffer
    vec3 worldPosition = texture(positionSampler, fragTexCoord).xyz;
    vec3 worldNormal = texture(normalSampler, fragTexCoord).xyz;
    float shadowMask = texture(shadowMaskSampler, fragTexCoord).r;

    // Check if this is a valid fragment (normal length check)
    if (length(worldNormal) < 0.01) {
        // Background pixel - simple sky color
        outColor = vec4(0.5, 0.7, 1.0, 1.0);
        return;
    }

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
