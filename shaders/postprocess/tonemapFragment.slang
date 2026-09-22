import "include/math.slang";
import "include/noise.slang";
import "include/ray.slang";

[[vk::binding(0, 0)]] Sampler2D hdrSampler;
[[vk::binding(1, 0)]] Sampler2D bloomSampler;
[[vk::binding(2, 0)]] Sampler2D positionSampler;

struct SceneData {
    float4 lightPosition;
    float4 lightColor;
    float4x4 view;
    float4x4 proj;
    int debugMode;
    float shadowStrength;
    int enableDistanceAttenuation;
    float exposureValue;
};

[[vk::binding(3, 0)]] ConstantBuffer<SceneData> sceneData;

struct PushConstants {
    int toneMapOperator;
    float gamma;
    float exposureValue;
    float vignetteIntensity;
    float chromaticAberrationIntensity;
    float bloomIntensity;
    float4 plumePosition;
    float4 plumeParams0;
    float4 plumeParams1;
    float4 plumeParams2;
};

[[vk::push_constant]] ConstantBuffer<PushConstants> pc;

float3 acesFilmic(float3 x) {
    float a = 2.51;
    float b = 0.03;
    float c = 2.43;
    float d = 0.59;
    float e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);
}

float3 reinhard(float3 x) {
    return x / (x + float3(1.0));
}

float3 applyToneMapOperator(float3 x) {
    if (pc.toneMapOperator == 1) {
        return acesFilmic(x);
    }
    if (pc.toneMapOperator == 2) {
        return reinhard(x);
    }
    return clamp(x, 0.0, 1.0);
}

float3 sampleWithChromaticAberration(float2 uv, float intensity) {
    float2 center = float2(0.5);
    float2 offset = (uv - center) * intensity;

    float r = hdrSampler.Sample(uv + offset).r;
    float g = hdrSampler.Sample(uv).g;
    float b = hdrSampler.Sample(uv - offset).b;

    return float3(r, g, b);
}

float computeVignette(float2 uv, float intensity) {
    float2 d = uv - float2(0.5);
    return 1.0 - intensity * dot(d, d) * 4.0;
}

// Heat plume refraction: closed-form deflection matching flame_plume.rs exactly.
static const float PLUME_REFRACTIVITY_AIR = 2.77e-4;
static const float PLUME_AMBIENT_TEMPERATURE_K = 293.0;
static const int PLUME_BAND_COUNT = 6;
static const float PLUME_FLAT_EXPONENT = 2e-2;
static const FbmParams PLUME_FBM = { 3, 2.03, 0.5, float3(0.0) };

// Abramowitz-Stegun 7.1.26 (same as flameErf in radial_integral.glsl).
float plumeErf(float x) {
    float magnitude = abs(x);
    float t = 1.0 / (1.0 + 0.3275911 * magnitude);
    float series = ((((1.061405429 * t - 1.453152027) * t + 1.421413741) * t - 0.284496736) * t
        + 0.254829592) * t;
    return sign(x) * (1.0 - series * exp(-magnitude * magnitude));
}

// int_{-halfWidth}^{halfWidth} s^m exp(-(a s^2 + b s + c)) ds for m = 0, 1, 2.
// Same formula as flameGaussianMoments in radial_integral.glsl.
float3 plumeGaussianMoments(float a, float b, float c, float halfWidth) {
    if (a * halfWidth * halfWidth < PLUME_FLAT_EXPONENT
        && abs(b) * halfWidth < PLUME_FLAT_EXPONENT) {
        float constantWeight = exp(-c);
        return constantWeight
            * float3(2.0 * halfWidth, 0.0, (2.0 / 3.0) * halfWidth * halfWidth * halfWidth);
    }

    float rootA = sqrt(a);
    float center = b / (2.0 * a);
    float peak = exp(-(c - b * b / (4.0 * a)));
    float moment0 = peak * 0.5 * sqrt(PI / a)
        * (plumeErf(rootA * (halfWidth + center)) - plumeErf(rootA * (center - halfWidth)));

    float gaugeHi = exp(-(a * halfWidth * halfWidth + b * halfWidth + c));
    float gaugeLo = exp(-(a * halfWidth * halfWidth - b * halfWidth + c));
    float moment1 = (gaugeLo - gaugeHi - b * moment0) / (2.0 * a);
    float moment2 = (moment0 - b * moment1 - halfWidth * (gaugeHi + gaugeLo)) / (2.0 * a);
    return float3(moment0, moment1, moment2);
}

// Temperature difference at height h: (T - 293) * min((h + 0.2)^(-5/3), 1.0).
float plumeDeltaT(float h) {
    float base = pc.plumeParams0.x - PLUME_AMBIENT_TEMPERATURE_K;
    float factor = pow(h + 0.2, -5.0 / 3.0);
    return base * min(factor, 1.0);
}

// Plume width at height h: width_base + width_slope * max(h, 0.0).
float plumeWidth(float h) {
    return pc.plumeParams0.y + pc.plumeParams0.z * max(h, 0.0);
}

// Closed-form deflection using 6 height bands with frozen parameters.
// Returns [dx, dz] deflection matching flame_plume.rs exactly.
float2 plumeDeflection(float3 o, float3 d, float tNear, float tFar) {
    float bandDt = (tFar - tNear) / float(PLUME_BAND_COUNT);
    float totalDx = 0.0;
    float totalDz = 0.0;

    for (int band = 0; band < PLUME_BAND_COUNT; band++) {
        float t0 = tNear + float(band) * bandDt;
        float tc = t0 + bandDt * 0.5;

        // Midpoint position
        float3 pc_pos = o + tc * d;
        float hc = pc_pos.y;
        float b_hc = plumeWidth(hc);
        float k = 1.0 / (b_hc * b_hc);

        // Gaussian coefficients
        float a = k * (d.x * d.x + d.z * d.z);
        float b_lin = 2.0 * k * (pc_pos.x * d.x + pc_pos.z * d.z);
        float c = k * (pc_pos.x * pc_pos.x + pc_pos.z * pc_pos.z);

        float3 moments = plumeGaussianMoments(a, b_lin, c, bandDt * 0.5);

        // Amplitude: -REFRACTIVITY_AIR * delta_t(hc) / AMBIENT_TEMPERATURE_K
        float amp = -PLUME_REFRACTIVITY_AIR * (plumeDeltaT(hc) / PLUME_AMBIENT_TEMPERATURE_K);

        // x contribution: -(2k) * amp * (pc.x*M0 + d.x*M1)
        totalDx += -(2.0 * k) * amp * (pc_pos.x * moments.x + d.x * moments.y);
        // z contribution: -(2k) * amp * (pc.z*M0 + d.z*M1)
        totalDz += -(2.0 * k) * amp * (pc_pos.z * moments.x + d.z * moments.y);
    }

   return float2(totalDx, totalDz);
}

[shader("fragment")]
float4 main([[vk::location(0)]] float2 fragTexCoord : TEXCOORD0,
    out float fragDepth : SV_Depth) : SV_Target0 {
    float4 positionData = positionSampler.Sample(fragTexCoord);
    if (positionData.w < 0.5) {
        fragDepth = DEPTH_FAR;
    } else {
        fragDepth = worldToClipDepth(positionData.xyz, sceneData.view, sceneData.proj);
    }
    // Heat plume refraction: if active, distort UV before HDR sampling.
    float2 uv = fragTexCoord;
    if (pc.plumePosition.w > 0.5) {
        // Camera position C from view matrix: C = -transpose(R) * t
        float4x4 viewColumns = transpose(sceneData.view);
        float3 C = -(viewColumns[0].xyz * viewColumns[3].x
                 + viewColumns[1].xyz * viewColumns[3].y
                 + viewColumns[2].xyz * viewColumns[3].z);

        // Sample background position
        float3 Pbg = positionSampler.Sample(fragTexCoord).xyz;
        if (any(Pbg != float3(0.0))) {
            float3 rayDir = normalize(Pbg - C);
            float dBg = length(Pbg - C);

            // Plume origin in local coords (C relative to plume base)
            float3 originLocal = C - pc.plumePosition.xyz;
            float plumeHeight = pc.plumeParams1.x;

            // Clip t interval to y-slab [0, plume_height]
            float tNear = 0.0;
            float tFar = dBg;

            if (rayDir.y > 1e-6) {
                float ty0 = -originLocal.y / rayDir.y;
                float ty1 = (plumeHeight - originLocal.y) / rayDir.y;
                tNear = max(tNear, ty0);
                tFar = min(tFar, ty1);
            } else if (rayDir.y < -1e-6) {
                float ty0 = -originLocal.y / rayDir.y;
                float ty1 = (plumeHeight - originLocal.y) / rayDir.y;
                tNear = max(tNear, ty1);
                tFar = min(tFar, ty0);
            } else {
                // Ray parallel to y-axis: check if origin is within slab
                if (originLocal.y < 0.0 || originLocal.y > plumeHeight) {
                    tFar = -1.0; // no intersection
                }
            }

            // Clip by radius |xz| < 4 * b(plume_height)
            float maxRadius = 4.0 * plumeWidth(plumeHeight);
            if (maxRadius > 0.0) {
                // Solve |originLocal.xz + t * rayDir.xz|^2 < maxRadius^2
                float ax = rayDir.x * rayDir.x + rayDir.z * rayDir.z;
                float bx = 2.0 * (originLocal.x * rayDir.x + originLocal.z * rayDir.z);
                float cx = originLocal.x * originLocal.x + originLocal.z * originLocal.z - maxRadius * maxRadius;

                if (ax > 1e-8) {
                    float disc = bx * bx - 4.0 * ax * cx;
                    if (disc > 0.0) {
                        float sqrtDisc = sqrt(disc);
                        float rt0 = (-bx - sqrtDisc) / (2.0 * ax);
                        float rt1 = (-bx + sqrtDisc) / (2.0 * ax);
                        tNear = max(tNear, rt0);
                        tFar = min(tFar, rt1);
                    } else {
                        // Ray never enters radius: no intersection
                        tFar = -1.0;
                    }
                } else {
                    // Near-zero xz direction: check if origin is within radius
                    if (cx > 0.0) {
                        tFar = -1.0; // no intersection
                    }
                }
           }


            // If interval is valid and non-empty, compute deflection
            if (tNear < tFar && tFar >= 0.0) {
               float2 theta = plumeDeflection(originLocal, rayDir, tNear, tFar) * pc.plumeParams0.w;

                float3 pMid = C + rayDir * (tNear + tFar) * 0.5;
                float turb = 1.0 + pc.plumeParams1.z * (fbm3(SinHash(), PLUME_FBM, pMid * 6.0 - float3(pc.plumeParams2.x, max(pc.plumeParams2.y, 1.0), pc.plumeParams2.z) * (2.0 * pc.plumeParams1.y)) * 2.0 - 1.0);
                theta *= turb;
              float deltaD = dBg - tFar;
                if (deltaD > 0.0) {
                    float3 Pdisp = Pbg + float3(theta.x, 0.0, theta.y) * deltaD;

                    float4 clipD = mul(mul(sceneData.proj, sceneData.view), float4(Pdisp, 1.0));
                    float4 clipO = mul(mul(sceneData.proj, sceneData.view), float4(Pbg, 1.0));
                    float2 uvD = (clipD.xy / clipD.w) * 0.5 + 0.5;
                    float2 uvO = (clipO.xy / clipO.w) * 0.5 + 0.5;
                    uv = clamp(uv + (uvD - uvO), 0.0, 1.0);
                }
            }
        }
    }

    // Every pixel takes one path: the HDR buffer already holds scene radiance for
    // background, opaque surfaces, grid and volumetrics alike, so a single exposure
    // and tone map curve applies to all of them.
   float3 hdrColor;
    if (pc.chromaticAberrationIntensity > 0.0) {
        hdrColor = sampleWithChromaticAberration(uv, pc.chromaticAberrationIntensity);
    } else {
        hdrColor = hdrSampler.Sample(uv).rgb;
    }

    if (pc.bloomIntensity > 0.0) {
        float3 bloomColor = bloomSampler.Sample(fragTexCoord).rgb;
        hdrColor += bloomColor * pc.bloomIntensity;
    }

    hdrColor *= pc.exposureValue;

    float3 mapped = applyToneMapOperator(hdrColor);
    mapped = pow(mapped, float3(1.0 / pc.gamma));

    if (pc.vignetteIntensity > 0.0) {
        float vignette = computeVignette(fragTexCoord, pc.vignetteIntensity);
        mapped *= vignette;
    }

    return float4(mapped, 1.0);
}
