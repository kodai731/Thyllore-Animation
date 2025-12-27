#version 450

layout(location = 0) in vec3 fragWorldPos;
layout(location = 1) in vec3 fragWorldNormal;
layout(location = 2) in vec2 fragTexCoord;

// Multiple render targets (MRT) for G-Buffer
layout(location = 0) out vec4 outPosition;  // World position
layout(location = 1) out vec4 outNormal;    // World normal
layout(location = 2) out vec4 outAlbedo;    // Albedo/Base color

layout(binding = 1) uniform sampler2D texSampler;

void main() {
    // Sample albedo texture
    vec4 albedo = texture(texSampler, fragTexCoord);
    if (albedo.a < 0.5) discard;

    // Output world position
    outPosition = vec4(fragWorldPos, 1.0);

    // Output normalized world normal
    outNormal = vec4(normalize(fragWorldNormal), 1.0);

    // Output albedo/base color
    outAlbedo = albedo;
}
