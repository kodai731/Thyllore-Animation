import "lightning/include/component.slang";
import "lightning/include/integral.slang";
import "lightning/include/reference_quadrature.slang";
import "include/frame.slang";
import "include/ray.slang";

[[vk::binding(2, 1)]] Sampler2D sceneDepthSampler;

struct LightningPush {
    int shadingMode;
    int stepCount;
    int debugView;
};

[[vk::push_constant]] ConstantBuffer<LightningPush> push;

static const int LIGHTNING_MODE_CLOSED_FORM = 0;
static const int LIGHTNING_MODE_REFERENCE_QUADRATURE = 1;
static const int LIGHTNING_DEBUG_OFF = 0;
static const int LIGHTNING_DEBUG_COVERAGE = 1;
static const int LIGHTNING_DEBUG_SEGMENT_HITS = 2;
static const int LIGHTNING_DEBUG_CORE_COVERAGE = 3;
static const float SEGMENT_T_MAX = 1e4;
static const float FLASH_RADIUS_MIN = 1e-6;

float3 lightningFlash(float2 uv) {
    float2 offset = (uv - lightning.flash.yz) / max(lightning.flash.w, FLASH_RADIUS_MIN);
    return lightning.flash.x * lightning.core.rgb * exp(-dot(offset, offset));
}

float4 outsideSegments(float2 fragTexCoord) {
    if (push.debugView == LIGHTNING_DEBUG_COVERAGE) {
        return float4(0.0, 0.0, 0.0, 1.0);
    }
    if (push.debugView != LIGHTNING_DEBUG_OFF || lightning.flash.x <= 0.0) {
        discard;
    }
    return float4(lightningFlash(fragTexCoord), 1.0);
}

[shader("fragment")]
float4 main([[vk::location(0)]] float2 fragTexCoord : TEXCOORD0) : SV_Target0 {
    float3 rayDir = reconstructRayDirection(fragTexCoord, lightning.invViewProj, frame.camera_pos.xyz);
    float3 localOrigin = mul(lightning.inverseModel, float4(frame.camera_pos.xyz, 1.0)).xyz;
    float3 localDir = mul(lightning.inverseModel, float4(rayDir, 0.0)).xyz;

    float tNear = 0.0;
    float tFar = SEGMENT_T_MAX;
    LightningHitMask hitMask;
    if (!clampToLightningSegments(localOrigin, localDir, tNear, tFar, hitMask)) {
        return outsideSegments(fragTexCoord);
    }
    tNear = max(tNear, 0.0);
    if (!clampToSceneDepth(sceneDepthSampler, fragTexCoord, lightning.invViewProj, frame.camera_pos.xyz, rayDir, tFar, tNear)
        || tFar <= tNear) {
        return outsideSegments(fragTexCoord);
    }

    float coverage;
    int hitCount;
    float coreCoverage;
    float3 scattered;
    if (push.shadingMode == LIGHTNING_MODE_REFERENCE_QUADRATURE) {
        scattered = lightningReferenceEmission(localOrigin, localDir, tNear, tFar, push.stepCount,
                                               coverage, hitCount, coreCoverage);
    } else {
        scattered = lightningEmission(localOrigin, localDir, tNear, tFar, hitMask, coverage, hitCount, coreCoverage);
    }

    if (push.debugView == LIGHTNING_DEBUG_COVERAGE) {
        return float4(float3(coverage), 1.0);
    }
    if (push.debugView == LIGHTNING_DEBUG_SEGMENT_HITS) {
        return float4(float(hitCount) / float(LIGHTNING_MAX_SEGMENTS), 0.0, 0.0, 1.0);
    }
    if (push.debugView == LIGHTNING_DEBUG_CORE_COVERAGE) {
        return float4(float3(coreCoverage), 1.0);
    }

    return float4(scattered + lightningFlash(fragTexCoord), 1.0);
}
