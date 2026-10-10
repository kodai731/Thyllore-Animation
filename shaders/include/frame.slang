// Per-frame camera and light block shared by every pass (set 0, binding 0).
module frame;

public struct FrameUBO {
    public float4x4 view;
    public float4x4 proj;
    public float4 camera_pos;
    public float4 light_pos;
    public float4 light_color;
    public float4 lighting;
};

[[vk::binding(0, 0)]] public ConstantBuffer<FrameUBO> frame;
