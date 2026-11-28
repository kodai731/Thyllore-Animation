// Ray query secondary rays for the water resolve pass: scene hits shade through the hit
// table, and hits that project on screen reuse the scene color instead.
module water_secondary;

import "include/noise.slang";
import "include/frame.slang";
import "include/hit_shading_record.slang";
import "water/include/component.slang";

[[vk::binding(1, 1)]] public Sampler2D sceneColorSampler;
[[vk::binding(2, 1)]] RaytracingAccelerationStructure sceneTlas;
[[vk::binding(3, 1)]] StructuredBuffer<HitShadingRecord> hitTable;

public static const int WATER_TRACE_MISS = 0;
public static const int WATER_TRACE_HIT_ON_SCREEN = 1;
public static const int WATER_TRACE_HIT_OFF_SCREEN = 2;

public static int waterTraceLastHitKind = WATER_TRACE_MISS;

// Interleaved gradient noise for depth-2 probabilistic sampling.
public float waterJitter(float2 fragCoord, float frameIndex) {
    return interleavedGradientNoise(fragCoord + float2(frameIndex * 5.588238));
}

public bool waterLightOccluded(float3 origin, float3 lightPos) {
    float3 toLight = lightPos - origin;
    float distance = length(toLight);
    RayDesc ray;
    ray.Origin = origin;
    ray.TMin = 1e-3;
    ray.Direction = toLight / distance;
    ray.TMax = distance;
    RayQuery<RAY_FLAG_NONE> rq;
    rq.TraceRayInline(sceneTlas, RAY_FLAG_FORCE_OPAQUE | RAY_FLAG_ACCEPT_FIRST_HIT_AND_END_SEARCH, 0xFF, ray);
    while (rq.Proceed()) {
    }
    return rq.CommittedStatus() == COMMITTED_TRIANGLE_HIT;
}

// Casts a ray against the scene TLAS (AABB candidates are ignored: water self re-entry is
// handled by the caller's tTorusNext). Returns true on a triangle hit.
public bool traceScene(float3 o, float3 d, float tMax, out float3 color, out float tHit) {
    color = float3(0.0);
    RayDesc ray;
    ray.Origin = o;
    ray.TMin = 1e-3;
    ray.Direction = d;
    ray.TMax = tMax;
    RayQuery<RAY_FLAG_NONE> rq;
    rq.TraceRayInline(sceneTlas, RAY_FLAG_FORCE_OPAQUE, 0xFF, ray);
    while (rq.Proceed()) {
    }

    if (rq.CommittedStatus() != COMMITTED_TRIANGLE_HIT) {
        tHit = tMax;
        waterTraceLastHitKind = WATER_TRACE_MISS;
        return false;
    }

    HitShadingRecord rec = hitTable[rq.CommittedInstanceID()];
    if (rec.geometryAddress == 0) {
        tHit = tMax;
        waterTraceLastHitKind = WATER_TRACE_MISS;
        return false;
    }
    tHit = rq.CommittedRayT();

    uint3 vi = hitTriangleVertices(rec, rq.CommittedPrimitiveIndex());
    float3 p0 = hitVertexPosition(rec, vi.x); float3 c0 = hitVertexColor(rec, vi.x); float3 n0 = hitVertexNormal(rec, vi.x);
    float3 p1 = hitVertexPosition(rec, vi.y); float3 c1 = hitVertexColor(rec, vi.y); float3 n1 = hitVertexNormal(rec, vi.y);
    float3 p2 = hitVertexPosition(rec, vi.z); float3 c2 = hitVertexColor(rec, vi.z); float3 n2 = hitVertexNormal(rec, vi.z);

    float2 bary = rq.CommittedTriangleBarycentrics();
    float u = bary.x;
    float v = bary.y;
    float w = 1.0 - u - v;

    float3 P = mul(rq.CommittedObjectToWorld3x4(), float4(p0 * w + p1 * u + p2 * v, 1.0));
    float3 vertexColor = c0 * w + c1 * u + c2 * v;
    float3 nLocal = n0 * w + n1 * u + n2 * v;
    float3 N = normalize(mul(nLocal, rq.CommittedWorldToObject3x4()).xyz);

    float3 L = normalize(frame.light_pos.xyz - P);
    float ndotl = max(dot(N, L), 0.0);
    float3 hitLighting = rec.baseColor.rgb * vertexColor * (0.15 + 0.85 * ndotl) * frame.light_color.rgb * water.lighting.x;

    float4 clip = mul(mul(frame.proj, frame.view), float4(P, 1.0));
    if (clip.w > 0.0) {
        float2 uv = (clip.xy / clip.w) * 0.5 + 0.5;
        if (uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0) {
            color = sceneColorSampler.Sample(uv).rgb;
            waterTraceLastHitKind = WATER_TRACE_HIT_ON_SCREEN;
            return true;
        }
    }

    color = hitLighting;
    waterTraceLastHitKind = WATER_TRACE_HIT_OFF_SCREEN;
    return true;
}
