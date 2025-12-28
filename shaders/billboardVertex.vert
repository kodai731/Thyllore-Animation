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
    vec4 billboardCenter = ubo.proj * ubo.view * ubo.model * vec4(0.0, 0.0, 0.0, 1.0);

    vec2 screenOffset = inPosition.xy * 0.1;

    gl_Position = billboardCenter;
    gl_Position.xy += screenOffset * billboardCenter.w;

    fragTexCoord = inTexCoord;
}
