#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_GOOGLE_include_directive : require
#extension GL_EXT_buffer_reference : require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#define WATER_UBO_BY_REFERENCE
#include "water/include/component.glsl"
#include "water/include/trace.glsl"
#include "include/trace_payload.glsl"
#include "include/trace_push.glsl"
#include "include/hit_shading_record.glsl"

layout(set = 0, binding = 0) uniform accelerationStructureEXT tlas;
layout(set = 0, binding = 3, std430) readonly buffer HitShadingTable { HitShadingRecord records[]; } hitTable;
layout(location = 0) rayPayloadInEXT TracePayload payload;
layout(location = 1) rayPayloadEXT TracePayload secondary;

bool traceSecondary(vec3 origin, vec3 dir) {
    secondary.depth = payload.depth + 1;
    traceRayEXT(tlas, gl_RayFlagsOpaqueEXT, 0xff, 0, 0, 0, origin + dir * 1e-3, 1e-3, dir, 1e30, 1);
    return secondary.color.a > 0.5 && int(secondary.hit.w) != TRACE_HIT_EFFECT;
}

void main() {
    HitShadingRecord rec = hitTable.records[gl_InstanceCustomIndexEXT];
    vec3 hitPoint = gl_WorldRayOriginEXT + gl_WorldRayDirectionEXT * gl_HitTEXT;
    if (payload.depth > 0 || rec.effectData == 0ul) {
        payload.color = vec4(0.0);
        payload.hit = vec4(hitPoint, TRACE_HIT_EFFECT);
        return;
    }
    water = WaterUBORef(rec.effectData).water;

    vec3 oLocal = gl_ObjectRayOriginEXT / water.radii.x;
    vec3 dLocal = normalize(gl_ObjectRayDirectionEXT);
    float roots[4];
    bool fallbackUsed;
    int hitCount = intersectTorus(oLocal, dLocal, water.radii.y / water.radii.x, roots, fallbackUsed);
    if (hitCount <= 0) {
        payload.color = vec4(0.0);
        payload.hit = vec4(0.0, 0.0, 0.0, TRACE_HIT_NONE);
        return;
    }

    WaterSurfaceHit surface = waterSurfaceHit(oLocal, dLocal, roots, hitCount);
    bool reflectionHit = traceSecondary(surface.entry, surface.reflectDir);
    vec3 reflectionColor = secondary.color.rgb;
    bool backgroundHit = traceSecondary(surface.exitOrigin, surface.exitDir);
    vec3 backgroundColor = secondary.color.rgb;

    vec3 composite = waterTraceComposite(surface, reflectionHit, reflectionColor, backgroundHit, backgroundColor, trace.lightPos.xyz, trace.lightColor.rgb);
    payload.color = vec4(composite, 1.0);
    payload.hit = vec4(surface.entry, TRACE_HIT_EFFECT);
}
