// LightningUBO is generated into thyllore-effect-core/src/lightning/gpu/components/generated.rs
// (cargo run -p thyllore-shader-manifest --bin generate_gpu_blocks).
module lightning_component;

public struct LightningUBO {
    public float4x4 model;
    public float4x4 inverseModel;
    public float4 core;
    public float4 glow;
    public float4 shape;
    public float4 debug_;
    public float4x4 invViewProj;
    public float4 flash;
};

[[vk::binding(0, 1)]] public ConstantBuffer<LightningUBO> lightning;

public struct LightningSegmentsUBO {
    public float4 seg_a_r0[256];
    public float4 seg_b_r1[256];
    public float4 seg_misc[256];
};

[[vk::binding(1, 1)]] public ConstantBuffer<LightningSegmentsUBO> segments;
