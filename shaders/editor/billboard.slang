import "include/math.slang";

struct UniformBufferObject {
    float4x4 model;
    float4x4 view;
    float4x4 proj;
};

[[vk::binding(0, 0)]] ConstantBuffer<UniformBufferObject> ubo;
[[vk::binding(2, 0)]] Sampler2D positionSampler;

struct Varyings {
    float4 position : SV_Position;
    [[vk::location(0)]] float2 fragTexCoord : TEXCOORD0;
    [[vk::location(1)]] float3 fragBillboardWorldPos : TEXCOORD1;
    [[vk::location(2)]] float fragBillboardViewDepth : TEXCOORD2;
    [[vk::location(3)]] float4 fragClipPos : TEXCOORD3;
};

[shader("vertex")]
Varyings vertexMain(
    [[vk::location(0)]] float3 inPosition : POSITION,
    [[vk::location(1)]] float2 inTexCoord : TEXCOORD) {
    Varyings output;

    float4 worldPos = mul(ubo.model, float4(inPosition, 1.0));

    float4 viewPos = mul(ubo.view, worldPos);

    float4 clipPos = mul(ubo.proj, viewPos);

    output.position = clipPos;
    output.fragTexCoord = inTexCoord;
    output.fragBillboardWorldPos = worldPos.xyz;
    output.fragBillboardViewDepth = -viewPos.z;
    output.fragClipPos = clipPos;
    return output;
}

static const float LINE_WIDTH = 0.045;
static const float BULB_RADIUS = 0.30;
static const float2 BULB_CENTER = float2(0.0, 0.12);
static const float NECK_HALF_WIDTH = 0.11;
static const float BASE_Y = -0.48;
static const float RAY_START_OFFSET = 0.10;
static const float RAY_LENGTH = 0.16;
static const float3 ICON_COLOR = float3(0.0);

float sdSegment(float2 p, float2 a, float2 b) {
    float2 pa = p - a;
    float2 ba = b - a;
    float h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h);
}

float sdArc(float2 p, float2 center, float radius, float gapAngle) {
    float2 rel = p - center;
    float ang = atan2(rel.y, rel.x);

    float gapCenter = -PI / 2.0;
    float halfGap = gapAngle * 0.5;
    float diff = ang - gapCenter;
    if (diff > PI) diff -= TWO_PI;
    if (diff < -PI) diff += TWO_PI;

    if (abs(diff) < halfGap) {
        float edgeAngle1 = gapCenter + halfGap;
        float edgeAngle2 = gapCenter - halfGap;
        float2 tip1 = center + radius * float2(cos(edgeAngle1), sin(edgeAngle1));
        float2 tip2 = center + radius * float2(cos(edgeAngle2), sin(edgeAngle2));
        return min(length(p - tip1), length(p - tip2));
    }

    return abs(length(rel) - radius);
}

float generateBulbShape(float2 p) {
    float d = 1e10;

    float gapAngle = 2.0 * asin(NECK_HALF_WIDTH / BULB_RADIUS);
    d = min(d, sdArc(p, BULB_CENTER, BULB_RADIUS, gapAngle));

    float junctionDy = sqrt(BULB_RADIUS * BULB_RADIUS - NECK_HALF_WIDTH * NECK_HALF_WIDTH);
    float junctionY = BULB_CENTER.y - junctionDy;

    float2 leftTop  = float2(-NECK_HALF_WIDTH, junctionY);
    float2 leftBot  = float2(-NECK_HALF_WIDTH * 0.82, BASE_Y);
    float2 rightTop = float2( NECK_HALF_WIDTH, junctionY);
    float2 rightBot = float2( NECK_HALF_WIDTH * 0.82, BASE_Y);
    d = min(d, sdSegment(p, leftTop, leftBot));
    d = min(d, sdSegment(p, rightTop, rightBot));

    float bw = NECK_HALF_WIDTH * 0.82;
    d = min(d, sdSegment(p, float2(-bw, BASE_Y), float2(bw, BASE_Y)));

    float lineStep = 0.07;
    for (int i = 1; i <= 2; i++) {
        float y = BASE_Y - lineStep * float(i);
        float w = bw * (1.0 - 0.2 * float(i));
        d = min(d, sdSegment(p, float2(-w, y), float2(w, y)));
    }

    return d;
}

float generateRays(float2 p) {
    float d = 1e10;
    float rayStart = BULB_RADIUS + RAY_START_OFFSET;
    float rayEnd = rayStart + RAY_LENGTH;

    float angles[5] = {
        1.57079632,
        2.35619449,
        0.78539816,
        PI,
        0.0
    };

    for (int i = 0; i < 5; i++) {
        float a = angles[i];
        float2 dir = float2(cos(a), sin(a));
        d = min(d, sdSegment(p, BULB_CENTER + dir * rayStart, BULB_CENTER + dir * rayEnd));
    }

    return d;
}

[shader("fragment")]
float4 fragmentMain(Varyings input) : SV_Target0 {
    float2 uv = input.fragTexCoord - 0.5;
    uv.y = -uv.y;
    float2 p = uv * 2.0;

    float dBulb = generateBulbShape(p);
    float dRays = generateRays(p);
    float d = min(dBulb, dRays);

    float fw = fwidth(d) * 1.2;
    float halfW = LINE_WIDTH * 0.5;
    float shapeAlpha = 1.0 - smoothstep(halfW - fw, halfW + fw, d);

    if (shapeAlpha < 0.01) {
        discard;
    }

    float3 ndc = input.fragClipPos.xyz / input.fragClipPos.w;
    float2 screenUV = (ndc.xy + 1.0) * 0.5;

    float4 gbufferPosition = positionSampler.Sample(screenUV);

    float4 gbufferViewPos = mul(ubo.view, float4(gbufferPosition.xyz, 1.0));
    float gbufferDepth = -gbufferViewPos.z;

    float maxRelevantDepth = input.fragBillboardViewDepth * 3.0;
    bool hasRelevantGeometry = gbufferPosition.w > 0.5 && gbufferDepth < maxRelevantDepth && gbufferDepth > 0.0;

    float alpha = shapeAlpha;
    if (hasRelevantGeometry) {
        bool billboardBehind = input.fragBillboardViewDepth > gbufferDepth;
        if (billboardBehind) {
            alpha *= 0.3;
        }
    }

    return float4(ICON_COLOR, alpha);
}
