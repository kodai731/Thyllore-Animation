module hit_shading_record;

import "include/vertex_streams.slang";

// One record per TLAS instance, indexed by InstanceID(); a procedural instance uses params and effectData.
public struct HitShadingRecord {
    public uint64_t geometryAddress;
    public uint64_t indexAddress;
    public uint64_t effectData;
    public uint64_t attributeAddress;
    public float4x4 model;
    public float4x4 normalMatrix;
    public float4 baseColor;
    public float4 params;
};

public uint3 hitTriangleVertices(HitShadingRecord rec, uint primitiveIndex) {
    uint firstCorner = primitiveIndex * 3;
    if (rec.indexAddress == 0) {
        return uint3(firstCorner, firstCorner + 1, firstCorner + 2);
    }
    uint* indices = (uint*)rec.indexAddress;
    return uint3(indices[firstCorner], indices[firstCorner + 1], indices[firstCorner + 2]);
}

public float3 hitVertexPosition(HitShadingRecord rec, uint vertex) {
    return readStreamFloat3((float*)rec.geometryAddress, vertex, 0);
}

public float3 hitVertexNormal(HitShadingRecord rec, uint vertex) {
    return readStreamFloat3((float*)rec.geometryAddress, vertex, GEOMETRY_NORMAL_OFFSET);
}

public float3 hitVertexColor(HitShadingRecord rec, uint vertex) {
    return readStreamFloat3((float*)rec.attributeAddress, vertex, 0);
}
