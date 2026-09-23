import "include/trace_payload.slang";
import "include/trace_push.slang";
import "include/hit_shading_record.slang";
import "include/torus_intersect.slang";
import "water/include/component.slang";
import "water/include/trace.slang";

[[vk::binding(0, 0)]] RaytracingAccelerationStructure tlas;
[[vk::binding(3, 0)]] StructuredBuffer<HitShadingRecord> hitTable;

static TracePayload secondary;

bool traceSecondary(int depth, float3 origin, float3 dir) {
    secondary.depth = depth + 1;
    RayDesc ray;
    ray.Origin = origin + dir * 1e-3;
    ray.TMin = 1e-3;
    ray.Direction = dir;
    ray.TMax = 1e30;
    TraceRay(tlas, RAY_FLAG_FORCE_OPAQUE, 0xff, 0, 0, 0, ray, secondary);
    return secondary.color.a > 0.5 && int(secondary.hit.w) != TRACE_HIT_EFFECT;
}

[shader("closesthit")]
void main(inout TracePayload payload, in BuiltInTriangleIntersectionAttributes attribs) {
    HitShadingRecord rec = hitTable[InstanceID()];
    float3 hitPoint = WorldRayOrigin() + WorldRayDirection() * RayTCurrent();
    if (payload.depth > 0 || rec.effectData == 0) {
        payload.color = float4(0.0);
        payload.hit = float4(hitPoint, TRACE_HIT_EFFECT);
        return;
    }
    WaterUBO* ubo = (WaterUBO*)rec.effectData;
    water = *ubo;

    float3 oLocal = ObjectRayOrigin() / water.radii.x;
    float3 dLocal = normalize(ObjectRayDirection());
    float roots[4];
    bool fallbackUsed;
    int hitCount = intersectTorus(oLocal, dLocal, water.radii.y / water.radii.x, roots, fallbackUsed);
    if (hitCount <= 0) {
        payload.color = float4(0.0);
        payload.hit = float4(0.0, 0.0, 0.0, TRACE_HIT_NONE);
        return;
    }

    WaterSurfaceHit surface = waterSurfaceHit(oLocal, dLocal, roots, hitCount);
    bool reflectionHit = traceSecondary(payload.depth, surface.entry, surface.reflectDir);
    float3 reflectionColor = secondary.color.rgb;
    bool backgroundHit = traceSecondary(payload.depth, surface.exitOrigin, surface.exitDir);
    float3 backgroundColor = secondary.color.rgb;

    float3 composite = waterTraceComposite(surface, reflectionHit, reflectionColor, backgroundHit, backgroundColor, trace.lightPos.xyz, trace.lightColor.rgb);
    payload.color = float4(composite, 1.0);
    payload.hit = float4(surface.entry, TRACE_HIT_EFFECT);
}
