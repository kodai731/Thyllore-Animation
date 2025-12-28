#version 450

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec2 inTexCoord;

layout(location = 0) out vec2 fragTexCoord;

void main() {
    vec3 billboardCenter = (ubo.model * vec4(0.0, 0.0, 0.0, 1.0)).xyz;

    vec3 cameraRight = vec3(ubo.view[0][0], ubo.view[1][0], ubo.view[2][0]);
    vec3 cameraUp = vec3(ubo.view[0][1], ubo.view[1][1], ubo.view[2][1]);

    vec3 worldPos = billboardCenter + cameraRight * inPosition.x + cameraUp * inPosition.y;

    gl_Position = ubo.proj * ubo.view * vec4(worldPos, 1.0);
    fragTexCoord = inTexCoord;
}
