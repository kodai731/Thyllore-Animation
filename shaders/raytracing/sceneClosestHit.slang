import "include/trace_payload.slang";
import "include/trace_push.slang";
import "include/hit_shading_record.slang";

[[vk::binding(3, 0)]] StructuredBuffer<HitShadingRecord> hitTable;

int3 vertexBaseIndices(HitShadingRecord rec, uint primitiveIndex) {
    if (rec.indexAddress != 0) {
        uint* indices = (uint*)rec.indexAddress;
        uint i0 = indices[primitiveIndex * 3];
        uint i1 = indices[primitiveIndex * 3 + 1];
        uint i2 = indices[primitiveIndex * 3 + 2];
        return int3(int(i0) * 3, int(i1) * 3, int(i2) * 3);
    }
    int vi0 = int(primitiveIndex) * 9;
    return int3(vi0, vi0 + 3, vi0 + 6);
}

[shader("closesthit")]
void main(inout TracePayload payload, in BuiltInTriangleIntersectionAttributes attribs) {
    HitShadingRecord rec = hitTable[InstanceID()];
    if (rec.vertexAddress == 0) {
        payload.color = float4(0.0);
        payload.hit = float4(0.0, 0.0, 0.0, TRACE_HIT_NONE);
        return;
    }
    float4* vb = (float4*)rec.vertexAddress;
    int3 vi = vertexBaseIndices(rec, PrimitiveIndex());
    float3 p0 = vb[vi.x].xyz;    float3 c0 = vb[vi.x + 1].rgb; float3 n0 = vb[vi.x + 2].xyz;
    float3 p1 = vb[vi.y].xyz;    float3 c1 = vb[vi.y + 1].rgb; float3 n1 = vb[vi.y + 2].xyz;
    float3 p2 = vb[vi.z].xyz;    float3 c2 = vb[vi.z + 1].rgb; float3 n2 = vb[vi.z + 2].xyz;
    float u = attribs.barycentrics.x;
    float v = attribs.barycentrics.y;
    float w = 1.0 - u - v;
    float3 P = mul(ObjectToWorld3x4(), float4(p0 * w + p1 * u + p2 * v, 1.0));
    float3 vertexColor = c0 * w + c1 * u + c2 * v;
    float3 N = n0 * w + n1 * u + n2 * v;
    float3 norm = normalize(mul(N, WorldToObject3x4()).xyz);
    float3 L = trace.lightPos.xyz - P;
    float dist = length(L);
    L /= dist;
    float ndotl = max(dot(norm, L), 0.0);
    float atten = 1.0 / (1.0 + 0.05 * dist * dist);
    float3 shaded = rec.baseColor.rgb * vertexColor * (0.15 + 0.85 * ndotl) * trace.lightColor.rgb * atten;
    payload.color = float4(shaded, 1.0);
    payload.hit = float4(P, TRACE_HIT_MESH);
}
