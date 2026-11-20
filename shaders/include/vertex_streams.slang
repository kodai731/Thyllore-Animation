module vertex_streams;

// A vertex stream is 6 floats per vertex (VertexGeometry / VertexAttributes in thyllore-model-core).
public static const uint VERTEX_STREAM_FLOAT_COUNT = 6;
public static const uint GEOMETRY_NORMAL_OFFSET = 3;

public float3 readFloat3(float* words, uint first) {
    return float3(words[first], words[first + 1], words[first + 2]);
}

public void writeFloat3(float* words, uint first, float3 value) {
    words[first] = value.x;
    words[first + 1] = value.y;
    words[first + 2] = value.z;
}

public float3 readStreamFloat3(float* stream, uint vertex, uint offset) {
    return readFloat3(stream, vertex * VERTEX_STREAM_FLOAT_COUNT + offset);
}

public void writeStreamFloat3(float* stream, uint vertex, uint offset, float3 value) {
    writeFloat3(stream, vertex * VERTEX_STREAM_FLOAT_COUNT + offset, value);
}
