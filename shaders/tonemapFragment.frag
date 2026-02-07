#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D hdrSampler;

layout(push_constant) uniform PushConstants {
    int toneMapOperator;
    float gamma;
    float exposureValue;
    float vignetteIntensity;
    float chromaticAberrationIntensity;
} pc;

vec3 acesFilmic(vec3 x) {
    float a = 2.51;
    float b = 0.03;
    float c = 2.43;
    float d = 0.59;
    float e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);
}

vec3 reinhard(vec3 x) {
    return x / (x + vec3(1.0));
}

vec3 sampleWithChromaticAberration(vec2 uv, float intensity) {
    vec2 center = vec2(0.5);
    vec2 offset = (uv - center) * intensity;

    float r = texture(hdrSampler, uv + offset).r;
    float g = texture(hdrSampler, uv).g;
    float b = texture(hdrSampler, uv - offset).b;

    return vec3(r, g, b);
}

float computeVignette(vec2 uv, float intensity) {
    vec2 d = uv - vec2(0.5);
    return 1.0 - intensity * dot(d, d) * 4.0;
}

void main() {
    vec3 hdrColor;
    if (pc.chromaticAberrationIntensity > 0.0) {
        hdrColor = sampleWithChromaticAberration(fragTexCoord, pc.chromaticAberrationIntensity);
    } else {
        hdrColor = texture(hdrSampler, fragTexCoord).rgb;
    }

    hdrColor *= pc.exposureValue;

    vec3 mapped;
    if (pc.toneMapOperator == 1) {
        mapped = acesFilmic(hdrColor);
    } else if (pc.toneMapOperator == 2) {
        mapped = reinhard(hdrColor);
    } else {
        mapped = clamp(hdrColor, 0.0, 1.0);
    }

    mapped = pow(mapped, vec3(1.0 / pc.gamma));

    if (pc.vignetteIntensity > 0.0) {
        float vignette = computeVignette(fragTexCoord, pc.vignetteIntensity);
        mapped *= vignette;
    }

    outColor = vec4(mapped, 1.0);
}
