#version 450

layout(binding = 1) uniform sampler2D texSampler;
layout(binding = 2) uniform sampler2D positionSampler;

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

layout(location = 0) in vec2 fragTexCoord;
layout(location = 1) in vec3 fragBillboardWorldPos;
layout(location = 2) in float fragBillboardViewDepth;
layout(location = 3) in vec4 fragClipPos;

layout(location = 0) out vec4 outColor;

void main() {
    vec4 texColor = texture(texSampler, fragTexCoord);
    if (texColor.a < 0.1) {
        discard;
    }

    vec3 ndc = fragClipPos.xyz / fragClipPos.w;
    vec2 screenUV = (ndc.xy + 1.0) * 0.5;

    vec4 gbufferPosition = texture(positionSampler, screenUV);

    vec4 gbufferViewPos = ubo.view * vec4(gbufferPosition.xyz, 1.0);
    float gbufferDepth = -gbufferViewPos.z;

    float maxRelevantDepth = fragBillboardViewDepth * 3.0;
    bool hasRelevantGeometry = gbufferPosition.w > 0.5 && gbufferDepth < maxRelevantDepth && gbufferDepth > 0.0;

    float alpha = texColor.a;
    if (hasRelevantGeometry) {
        bool billboardBehind = fragBillboardViewDepth > gbufferDepth;
        if (billboardBehind) {
            alpha *= 0.3;
        }
    }

    outColor = vec4(texColor.rgb, alpha);
}
