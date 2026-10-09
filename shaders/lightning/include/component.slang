// LightningUBO is generated into thyllore-effect-core/src/lightning/gpu/components/generated.rs
// (cargo run -p thyllore-shader-manifest --bin generate_gpu_blocks).
module lightning_component;

public struct LightningUBO {
    public float4x4 model;
    public float4x4 inverseModel;
    public float4 core;
    public float4 rim;
    public float4 shape;
    public float4 debug_;
    public float4x4 invViewProj;
    public float4 flash;
};

[[vk::binding(0, 1)]] public ConstantBuffer<LightningUBO> lightning;

public static const int LIGHTNING_MAX_SEGMENTS = 256;

public struct LightningSegmentsUBO {
    public float4 seg_a_r0[LIGHTNING_MAX_SEGMENTS];
    public float4 seg_b_r1[LIGHTNING_MAX_SEGMENTS];
    public float4 seg_misc[LIGHTNING_MAX_SEGMENTS];
};

[[vk::binding(1, 1)]] public ConstantBuffer<LightningSegmentsUBO> segments;
