#version 450

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec4 inColor;
layout(location = 2) in vec2 inTexCoord;
layout(location = 3) in vec3 inNormal;

layout(location = 0) out vec3 fragWorldPos;
layout(location = 1) out vec3 fragWorldNormal;
layout(location = 2) out vec2 fragTexCoord;

void main() {
    // Transform position to world space
    vec4 worldPos = ubo.model * vec4(inPosition, 1.0);
    fragWorldPos = worldPos.xyz;

    // Transform normal to world space (use inverse transpose for non-uniform scaling)
    // For now, assuming uniform scaling, so just use model matrix
    fragWorldNormal = mat3(ubo.model) * inNormal;

    fragTexCoord = inTexCoord;

    // Final position in clip space
    gl_Position = ubo.proj * ubo.view * worldPos;
}
