module hit_shading_record;

// One record per TLAS instance, indexed by InstanceID(). Mesh instances fill the vertex / index
// addresses; procedural instances fill params (x = kind, yz = torus radii) and effectData, the
// device address of the effect's own instance block.
public struct HitShadingRecord {
    public uint64_t vertexAddress;
    public uint64_t indexAddress;
    public uint64_t effectData;
    public uint64_t reserved;
    public float4x4 model;
    public float4x4 normalMatrix;
    public float4 baseColor;
    public float4 params;
};
