module trace_push;

public struct TracePush {
    public float4x4 invViewProj;
    public float4 cameraPos;
    public float4 lightPos;
    public float4 lightColor;
};

[[vk::push_constant]] public ConstantBuffer<TracePush> trace;
