import "wind/include/component.slang";
import "wind/include/field.slang";
import "wind/include/integral.slang";
import "wind/include/lighting.slang";
import "wind/include/reference_quadrature.slang";
import "include/frame.slang";
import "include/radiative_transfer.slang";
import "include/ray.slang";
import "include/volume_shell.slang";

[[vk::binding(1, 1)]] Sampler2D sceneDepthSampler;

struct WindPush {
    int mode;
    int stepCount;
    int debugView;
};

[[vk::push_constant]] ConstantBuffer<WindPush> push;

static const int WIND_MODE_CLOSED_FORM = 0;
static const int WIND_MODE_REFERENCE_QUADRATURE = 1;
static const int WIND_DEBUG_OFF = 0;
static const int WIND_DEBUG_OPTICAL_DEPTH = 1;
static const int WIND_DEBUG_KNOT_COUNT = 2;
static const int WIND_DEBUG_COVERAGE = 3;
static const float SEGMENT_T_MAX = 1e4;

// Scene depth projected onto the view ray cuts the interval where an opaque surface occludes.
bool clampToSceneDepth(float2 fragTexCoord, float3 rayDir, inout float tFar, float tNear) {
    float sceneDepth = sceneDepthSampler.Sample(fragTexCoord).r;
    if (sceneDepth == DEPTH_FAR) {
        return true;
    }
    float4 surfaceClip = mul(wind.invViewProj, float4(fragTexCoord * 2.0 - 1.0, sceneDepth, 1.0));
    float3 surfaceWorld = surfaceClip.xyz / surfaceClip.w;
    float tDepth = dot(surfaceWorld - frame.camera_pos.xyz, rayDir);
    if (tNear >= tDepth) {
        return false;
    }
    tFar = min(tFar, tDepth);
    return true;
}

[shader("fragment")]
float4 main([[vk::location(0)]] float2 fragTexCoord : TEXCOORD0) : SV_Target0 {
    float3 rayDir = reconstructRayDirection(fragTexCoord, wind.invViewProj, frame.camera_pos.xyz);
    float3 localOrigin = mul(wind.inverseModel, float4(frame.camera_pos.xyz, 1.0)).xyz;
    float3 localDir = mul(wind.inverseModel, float4(rayDir, 0.0)).xyz;

    float tNear = 0.0;
    float tFar = SEGMENT_T_MAX;
    if (!clampToWindCone(localOrigin, localDir, tNear, tFar)) {
        if (push.debugView != WIND_DEBUG_COVERAGE) {
            discard;
        }
        return float4(0.0, 0.0, 0.0, 1.0);
    }
    tNear = max(tNear, 0.0);
    if (!clampToSceneDepth(fragTexCoord, rayDir, tFar, tNear) || tFar <= tNear) {
        if (push.debugView != WIND_DEBUG_COVERAGE) {
            discard;
        }
        return float4(0.0, 0.0, 0.0, 1.0);
    }

    float3 lightPositionLocal = mul(wind.inverseModel, float4(frame.light_pos.xyz, 1.0)).xyz;

    int knotCount = 0;
    float opticalDepth = 0.0;
    float3 scattered = float3(0.0);
    if (push.mode == WIND_MODE_REFERENCE_QUADRATURE) {
        opticalDepth = windReferenceOpticalDepth(localOrigin, localDir, tNear, tFar, push.stepCount);
    } else {
        scattered = windSingleScatterRadiance(
            localOrigin, localDir, tNear, tFar, lightPositionLocal, opticalDepth, knotCount);
    }
    float coverage = 1.0 - rteTransmittanceFromOpticalDepth(opticalDepth);

    if (push.debugView == WIND_DEBUG_OPTICAL_DEPTH) {
        return float4(float3(coverage), 1.0);
    }
    if (push.debugView == WIND_DEBUG_KNOT_COUNT) {
        return float4(float(knotCount) / float(RAY_MAX_KNOTS), 0.0, 0.0, 1.0);
    }
    if (push.debugView == WIND_DEBUG_COVERAGE) {
        return float4(float3(coverage), 1.0);
    }

    if (push.mode == WIND_MODE_REFERENCE_QUADRATURE) {
        scattered = wind.albedo.rgb * windSkyBrightness() * coverage;
    }
    return float4(scattered, coverage);
}
