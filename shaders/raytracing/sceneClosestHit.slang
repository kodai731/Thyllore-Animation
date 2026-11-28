import "include/trace_payload.slang";
import "include/trace_push.slang";
import "include/hit_shading_record.slang";

[[vk::binding(3, 0)]] StructuredBuffer<HitShadingRecord> hitTable;

[shader("closesthit")]
void main(inout TracePayload payload, in BuiltInTriangleIntersectionAttributes attribs) {
    HitShadingRecord rec = hitTable[InstanceID()];
    if (rec.geometryAddress == 0) {
        payload.color = float4(0.0);
        payload.hit = float4(0.0, 0.0, 0.0, TRACE_HIT_NONE);
        return;
    }
    uint3 vi = hitTriangleVertices(rec, PrimitiveIndex());
    float3 p0 = hitVertexPosition(rec, vi.x); float3 c0 = hitVertexColor(rec, vi.x); float3 n0 = hitVertexNormal(rec, vi.x);
    float3 p1 = hitVertexPosition(rec, vi.y); float3 c1 = hitVertexColor(rec, vi.y); float3 n1 = hitVertexNormal(rec, vi.y);
    float3 p2 = hitVertexPosition(rec, vi.z); float3 c2 = hitVertexColor(rec, vi.z); float3 n2 = hitVertexNormal(rec, vi.z);
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
