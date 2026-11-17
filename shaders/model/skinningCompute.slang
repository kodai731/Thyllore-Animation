struct SkinningPush {
    uint64_t restAddress;
    uint64_t influenceAddress;
    uint64_t paletteAddress;
    uint64_t skinnedAddress;
    uint vertexCount;
};

[[vk::push_constant]] ConstantBuffer<SkinningPush> skinning;

// Streams are plain 32 bit words (thyllore-model-core): VertexGeometry = position.xyz, normal.xyz;
// SkinInfluence = 4 palette indices then 4 weights; a palette entry = a column-major 4x4 matrix.
static const uint GEOMETRY_WORD_COUNT = 6;
static const uint GEOMETRY_NORMAL_OFFSET = 3;
static const uint INFLUENCE_WORD_COUNT = 8;
static const uint INFLUENCE_WEIGHT_OFFSET = 4;
static const uint INFLUENCES_PER_VERTEX = 4;
static const uint MATRIX_WORD_COUNT = 16;
static const float3 FALLBACK_NORMAL = float3(0.0, 1.0, 0.0);

float3 readFloat3(float* words, uint first) {
    return float3(words[first], words[first + 1], words[first + 2]);
}

void writeFloat3(float* words, uint first, float3 value) {
    words[first] = value.x;
    words[first + 1] = value.y;
    words[first + 2] = value.z;
}

float3 transformVector(float* palette, uint entry, float3 v) {
    uint first = entry * MATRIX_WORD_COUNT;
    return readFloat3(palette, first) * v.x
        + readFloat3(palette, first + 4) * v.y
        + readFloat3(palette, first + 8) * v.z;
}

float3 transformPoint(float* palette, uint entry, float3 p) {
    return transformVector(palette, entry, p) + readFloat3(palette, entry * MATRIX_WORD_COUNT + 12);
}

// Linear blend skinning, the same sum as skin_geometry in thyllore-model-core.
[shader("compute")]
[numthreads(64, 1, 1)]
void main(uint3 dispatchThreadId : SV_DispatchThreadID) {
    uint vertex = dispatchThreadId.x;
    if (vertex >= skinning.vertexCount) {
        return;
    }

    float* rest = (float*)skinning.restAddress;
    uint* paletteIndices = (uint*)skinning.influenceAddress;
    float* weights = (float*)skinning.influenceAddress;
    float* palette = (float*)skinning.paletteAddress;
    float* skinned = (float*)skinning.skinnedAddress;

    uint geometryFirst = vertex * GEOMETRY_WORD_COUNT;
    float3 restPosition = readFloat3(rest, geometryFirst);
    float3 restNormal = readFloat3(rest, geometryFirst + GEOMETRY_NORMAL_OFFSET);

    float3 position = float3(0.0);
    float3 normal = float3(0.0);
    uint influenceFirst = vertex * INFLUENCE_WORD_COUNT;
    for (uint slot = 0; slot < INFLUENCES_PER_VERTEX; ++slot) {
        uint entry = paletteIndices[influenceFirst + slot];
        float weight = weights[influenceFirst + INFLUENCE_WEIGHT_OFFSET + slot];
        position += transformPoint(palette, entry, restPosition) * weight;
        normal += transformVector(palette, entry, restNormal) * weight;
    }

    float normalLength = length(normal);
    float3 unitNormal = normalLength > 0.0 ? normal / normalLength : FALLBACK_NORMAL;

    writeFloat3(skinned, geometryFirst, position);
    writeFloat3(skinned, geometryFirst + GEOMETRY_NORMAL_OFFSET, unitNormal);
}
