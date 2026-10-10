module water_flow;

public float2 torusUV(float3 pLocalNormalized) {
    return float2(atan2(pLocalNormalized.z, pLocalNormalized.x),
                  atan2(pLocalNormalized.y, length(pLocalNormalized.xz) - 1.0));
}

public float2 advectUV(float2 uv, float2 flowRate, float time) {
    return uv + flowRate * time;
}
