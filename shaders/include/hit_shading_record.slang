module hit_shading_record;

// One record per TLAS instance, indexed by InstanceID(). Mesh instances fill the geometry /
// attribute / index addresses; procedural instances fill params (x = kind, yz = torus radii) and
// effectData, the device address of the effect's own instance block.
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

// Both vertex streams are 6 floats per vertex: geometry = position.xyz, normal.xyz and
// attributes = color.rgba, texCoord.uv (VertexGeometry / VertexAttributes in thyllore-model-core).
static const uint VERTEX_STREAM_FLOAT_COUNT = 6;
static const uint GEOMETRY_NORMAL_OFFSET = 3;

public uint3 hitTriangleVertices(HitShadingRecord rec, uint primitiveIndex) {
    uint firstCorner = primitiveIndex * 3;
    if (rec.indexAddress == 0) {
        return uint3(firstCorner, firstCorner + 1, firstCorner + 2);
    }
    uint* indices = (uint*)rec.indexAddress;
    return uint3(indices[firstCorner], indices[firstCorner + 1], indices[firstCorner + 2]);
}

float3 readStreamFloat3(uint64_t streamAddress, uint vertex, uint offset) {
    float* stream = (float*)streamAddress;
    uint first = vertex * VERTEX_STREAM_FLOAT_COUNT + offset;
    return float3(stream[first], stream[first + 1], stream[first + 2]);
}

public float3 hitVertexPosition(HitShadingRecord rec, uint vertex) {
    return readStreamFloat3(rec.geometryAddress, vertex, 0);
}

public float3 hitVertexNormal(HitShadingRecord rec, uint vertex) {
    return readStreamFloat3(rec.geometryAddress, vertex, GEOMETRY_NORMAL_OFFSET);
}

public float3 hitVertexColor(HitShadingRecord rec, uint vertex) {
    return readStreamFloat3(rec.attributeAddress, vertex, 0);
}
