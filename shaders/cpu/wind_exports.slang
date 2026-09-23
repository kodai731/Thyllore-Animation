import "include/volume_shell.slang";
import "wind/include/integral.slang";
import "wind/include/field.slang";

export __extern_cpp float thylloreWindOpticalDepth(float3 o, float3 d, float tNear, float tFar) {
    return windOpticalDepth(o, d, tNear, tFar);
}

export __extern_cpp float thylloreWindDensityAt(float3 p) {
    return windDensityAt(p);
}

export __extern_cpp float thylloreWindOpticalDepthToward(float3 o, float3 d) {
    WindShadowMedium medium;
    return medium.shadowOpticalDepthToward(o, d);
}

export __extern_cpp bool thylloreWindClampRayToCone(float3 o, float3 d, float tNearIn, float tFarIn, out float tNearOut, out float tFarOut) {
    float tNear = tNearIn;
    float tFar = tFarIn;
    bool hit = clampToWindCone(o, d, tNear, tFar);
    tNearOut = tNear;
    tFarOut = tFar;
    return hit;
}

export __extern_cpp float thylloreWindEnvelopeRadius(float h) {
    return shellEnvelopeRadius(windShell(), h);
}
