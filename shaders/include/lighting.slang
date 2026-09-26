module lighting;

import "include/math.slang";

public static const float DIELECTRIC_F0 = 0.04;
public static const float GEOMETRY_EPSILON = 0.0001;

public struct SurfaceSample {
    public float3 baseColor;
    public float metallic;
    public float roughness;
    public float3 normal;
}

float ggx_distribution(float roughness, float NoH) {
    float a = roughness * roughness;
    float a2 = a * a;
    float d = NoH * NoH;
    float num = a2;
    float denom = (d * (a2 - 1.0) + 1.0);
    return num / (PI * denom * denom);
}

float smith_schlick_ggx_geometry(float roughness, float NoV) {
    float r = roughness + 1.0;
    float k = r * r * 0.125;
    return NoV / (NoV * (1.0 - k) + k);
}

float3 schlick_fresnel(float3 f0, float VoH) {
    float exponent = pow(1.0 - VoH, 5.0);
    return f0 + (1.0 - f0) * exponent;
}

public float distanceAttenuation(float distance, float linear, float quadratic) {
    return 1.0 / (1.0 + linear * distance + quadratic * distance * distance);
}

public float3 shadeSurface(SurfaceSample surface, float3 lightDir, float3 viewDir, float3 radiance, float ambient) {
    float3 f0 = lerp(float3(DIELECTRIC_F0), surface.baseColor, surface.metallic);
    float3 diffuseColor = surface.baseColor * (1.0 - surface.metallic);

    float3 halfDir = normalize(lightDir + viewDir);
    float NoL = max(dot(surface.normal, lightDir), 0.0);
    float NoV = max(dot(surface.normal, viewDir), 0.0);
    float NoH = max(dot(surface.normal, halfDir), 0.0);
    float VoH = max(dot(viewDir, halfDir), 0.0);

    float specDistribution = ggx_distribution(surface.roughness, NoH);
    float specGeometry = smith_schlick_ggx_geometry(surface.roughness, NoV) * smith_schlick_ggx_geometry(surface.roughness, NoL);
    float3 specFresnel = schlick_fresnel(f0, VoH);

    float3 specular = (specDistribution * specGeometry * specFresnel) / (4.0 * NoV * NoL + GEOMETRY_EPSILON);

    return ambient * surface.baseColor + (diffuseColor + PI * specular) * NoL * radiance;
}
