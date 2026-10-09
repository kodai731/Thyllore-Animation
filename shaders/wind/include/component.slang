// WindUBO is generated into thyllore-effect-core/src/wind/gpu/components/generated.rs
// (cargo run -p thyllore-shader-manifest --bin generate_gpu_blocks).
module wind_component;

public struct WindUBO {
    public float4x4 model;
    public float4x4 inverseModel;
    public float4 shape;
    public float4 wall;
    public float4 optics;
    public float4 albedo;
    public float4 lighting;
    public float4 streak;
    public float4 streak2;
    public float4 eddy;
    public float4 eddy2;
    public float4 puffParams;
    public float4 puffs[96];
    public float4x4 invViewProj;
};

[[vk::binding(0, 1)]] public ConstantBuffer<WindUBO> wind;

public int windPuffCount() {
    return int(wind.puffParams.x);
}

public float windPuffStrength() {
    return wind.puffParams.y;
}

public int windShadowSlot() {
    return int(wind.puffParams.z);
}

public float windShadowRadialExtent() {
    return wind.puffParams.w;
}
