// WaterUBO is mirrored by thyllore-effect-core's WaterUBO (checked by block_definition tests).
// Each entry copies its own source (descriptor block or hit record pointer) into `water` once.
module water_component;

public struct WaterUBO {
    public float4x4 model;
    public float4x4 inverseModel;
    public float4 radii;
    public float4 absorption;
    public float4 flow;
    public float4 composite;
    public float4 tint;
    public float4 lighting;
    public float4 scattering;
    public float4 temporal;
    public float4 waveModes[16];
    public float4x4 invViewProj;
    public float4 lbModes[20];
};

public static WaterUBO water = {};
