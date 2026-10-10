import "include/torus_intersect.slang";
import "include/hit_shading_record.slang";

[[vk::binding(3, 0)]] StructuredBuffer<HitShadingRecord> hitTable;

[shader("intersection")]
void main() {
    HitShadingRecord rec = hitTable[InstanceID()];
    float majorRadius = rec.params.y;
    float minorRadius = rec.params.z;

    float3 dObj = ObjectRayDirection();
    float len = length(dObj);
    float3 o = ObjectRayOrigin() / majorRadius;
    float3 d = dObj / len;
    float roots[4];
    bool fb;
    int n = intersectTorus(o, d, minorRadius / majorRadius, roots, fb);
    if (n > 0) {
        float t = roots[0] * majorRadius / len;
        if (t >= RayTMin() && t <= RayTCurrent()) {
            BuiltInTriangleIntersectionAttributes attribs;
            attribs.barycentrics = float2(0.0);
            ReportHit(t, 0u, attribs);
        }
    }
}
