struct VSOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] float2 fragTexCoord : TEXCOORD0;
};

[shader("vertex")]
VSOutput main(uint vertexIndex : SV_VertexID) {
    VSOutput output;

    float2 uv = float2((vertexIndex << 1) & 2, vertexIndex & 2);
    output.fragTexCoord = uv;
    output.position = float4(uv * 2.0 - 1.0, 0.0, 1.0);
    return output;
}
