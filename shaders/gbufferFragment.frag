#version 450

layout(location = 0) in vec3 fragWorldPos;
layout(location = 1) in vec3 fragWorldNormal;
layout(location = 2) in vec2 fragTexCoord;
layout(location = 3) in vec4 fragColor;

layout(location = 0) out vec4 outPosition;
layout(location = 1) out vec4 outNormal;
layout(location = 2) out vec4 outAlbedo;
layout(location = 3) out uint outObjectID;

layout(set = 1, binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform PushConstants {
    uint objectID;
    float ghostTintR;
    float ghostTintG;
    float ghostTintB;
    float ghostOpacity;
} pc;

void main() {
    vec4 texColor = texture(texSampler, fragTexCoord);
    vec4 albedo = texColor * fragColor;
    if (albedo.a < 0.5) discard;

    outPosition = vec4(fragWorldPos, 1.0);
    outNormal = vec4(normalize(fragWorldNormal), 1.0);

    if (pc.ghostOpacity > 0.0) {
        vec3 ghostColor = vec3(pc.ghostTintR, pc.ghostTintG, pc.ghostTintB);
        outAlbedo = vec4(mix(albedo.rgb, ghostColor, pc.ghostOpacity), 1.0);
    } else {
        outAlbedo = albedo;
    }

    outObjectID = pc.objectID;
}
