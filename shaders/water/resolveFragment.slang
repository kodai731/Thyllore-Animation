import "include/frame.slang";
import "include/radiative_transfer.slang";
import "include/ray.slang";
import "include/torus_intersect.slang";
import "water/include/component.slang";
import "water/include/flow.slang";
import "water/include/surface.slang";
import "water/include/lb.slang";
import "water/include/lighting.slang";
#ifdef WATER_RAY_QUERY
import "water/include/secondary.slang";
#endif

[[vk::binding(0, 1)]] ConstantBuffer<WaterUBO> waterBlock;
#ifndef WATER_RAY_QUERY
[[vk::binding(1, 1)]] Sampler2D sceneColorSampler;
#endif
#ifdef WATER_RAY_QUERY
[[vk::binding(4, 1)]] Sampler2D waterHistorySampler;
[[vk::binding(5, 1)]] Sampler2D waterTraceSampler;
#endif

struct WaterPush {
    int secondaryRays;
    int debugView;
};

[[vk::push_constant]] ConstantBuffer<WaterPush> push;

static const int SECONDARY_RAY_QUERY = 0;
static const int SECONDARY_SCREEN_SPACE = 1;
static const int SECONDARY_TRACE_PIPELINE = 2;
static const int DEBUG_OFF = 0;
static const int DEBUG_ROOT_COUNT = 1;
static const int DEBUG_NORMAL = 2;
static const int DEBUG_PROBE_NEAR = 3;
static const int DEBUG_PROBE_SECOND = 4;
static const int DEBUG_TRACE_IMAGE = 5;
static const int DEBUG_HISTORY_NAN = 6;
static const int DEBUG_HIT_KIND = 7;

struct Output {
    float4 color : SV_Target0;
#ifdef WATER_RAY_QUERY
    float4 history : SV_Target1;
#endif
    float depth : SV_Depth;
};

Output debugOutput(float4 color, float depth) {
    Output output;
    output.color = color;
#ifdef WATER_RAY_QUERY
    output.history = color;
#endif
    output.depth = depth;
    return output;
}

float fresnelAverage(float cosThetaI, float eta) {
    float sinThetaT2 = (1.0 - cosThetaI * cosThetaI) / (eta * eta);
    float cosThetaT = sqrt(max(1.0 - sinThetaT2, 0.0));
    float rPar = (eta * cosThetaI - cosThetaT) / (eta * cosThetaI + cosThetaT);
    float rPerp = (cosThetaI - eta * cosThetaT) / (cosThetaI + eta * cosThetaT);
    return (rPar * rPar + rPerp * rPerp) * 0.5;
}

float3 surfaceNormalAt(float3 pLocal, float2 footprint, float rHat, out float slopeVariance) {
    float2 uv = torusUV(pLocal);
    float h, hu, hv;
    waterHeightAndGradient(uv, water.flow.z, water.flow.xy, int(water.composite.z), footprint, h, hu, hv, slopeVariance);
    waterLbHeightAndGradient(uv, water.flow.z, water.flow.xy, h, hu, hv);
    return waterPerturbedNormal(uv.x, uv.y, h, hu, hv, rHat);
}

float2 projectToScreen(float4 worldPos, float2 fallback) {
    float4 clip = mul(mul(frame.proj, frame.view), worldPos);
    if (clip.w > 0) {
        return clamp((clip.xy / clip.w) * 0.5 + 0.5, 0.0, 1.0);
    }
    return fallback;
}

#ifdef WATER_RAY_QUERY
// Second-bounce misses read the scene color through a wide box filter so thin screen
// features (grid lines) cannot alias into combs after two refractions.
float3 sampleSceneColorBlurred(float2 uv) {
    uint width, height;
    sceneColorSampler.GetDimensions(width, height);
    float2 texel = 2.0 / float2(width, height);
    float3 sum = float3(0.0);
    for (int y = -3; y <= 3; ++y) {
        for (int x = -3; x <= 3; ++x) {
            sum += sceneColorSampler.Sample(clamp(uv + float2(x, y) * texel, 0.0, 1.0)).rgb;
        }
    }
    return sum / 49.0;
}

// Fresnel-weighted probabilistic bounce at a torus re-entry point; returns the traced color,
// and the bounce direction so a miss can be shaded by the caller.
float3 reentryBounce(float3 pWorld, float3 dir, float2 footprint, float rHat, float eta, float2 fragCoord, out float3 bounced, out float reentryWeight, out float slopeVariance, out bool traced) {
    float3 pLocal = mul(water.inverseModel, float4(pWorld, 1.0)).xyz / water.radii.x;
    float3 nLocal = surfaceNormalAt(pLocal, footprint, rHat, slopeVariance);
    float3 n = normalize(mul(float3x3(water.model), nLocal));
    float cosThetaI = max(-dot(dir, n), 0.0);
    reentryWeight = torusReentryWeight(cosThetaI);
    float F = fresnelAverage(cosThetaI, eta);
    float u = waterJitter(fragCoord, water.temporal.y);
    if (u < F) {
        bounced = reflect(dir, n);
    } else {
        bounced = refract(dir, n, 1.0 / eta);
        if (length(bounced) < 1e-4) bounced = reflect(dir, n);
    }
    float3 color;
    float tScene;
    traced = traceScene(pWorld, bounced, 1e30, color, tScene);
    return color;
}
#endif

[shader("fragment")]
Output main([[vk::location(0)]] float2 fragTexCoord : TEXCOORD0, float4 fragCoord : SV_Position) {
    water = waterBlock;
    float3 rayDir = reconstructRayDirection(fragTexCoord, water.invViewProj, frame.camera_pos.xyz);

    float3 pLocalOrigin = mul(water.inverseModel, float4(frame.camera_pos.xyz, 1.0)).xyz;
    pLocalOrigin /= water.radii.x;
    float3 dLocal = mul(water.inverseModel, float4(rayDir, 0.0)).xyz;
    dLocal = normalize(dLocal);

    float roots[4];
    bool fallbackUsed;
    int hitCount = intersectTorus(pLocalOrigin, dLocal, water.radii.y / water.radii.x, roots, fallbackUsed);
    if (hitCount == 0) {
        discard;
    }

    float t1 = roots[0] * water.radii.x;
    float3 p1 = frame.camera_pos.xyz + t1 * rayDir;
    float waterDepth = worldToClipDepth(p1, frame.view, frame.proj);

    if (push.debugView == DEBUG_ROOT_COUNT) {
        if (hitCount == 2) {
            return debugOutput(float4(0.0, 1.0, 0.0, 1.0), waterDepth);
        } else if (hitCount == 4) {
            return debugOutput(float4(0.0, 0.0, 1.0, 1.0), waterDepth);
        }
        return debugOutput(float4(1.0, 0.0, 0.0, 1.0), waterDepth);
    }

#ifdef WATER_RAY_QUERY
    if (push.debugView == DEBUG_TRACE_IMAGE) {
        return debugOutput(float4(waterTraceSampler.Sample(fragTexCoord).rgb, 1.0), waterDepth);
    }
#endif

    if (push.debugView == DEBUG_PROBE_NEAR || push.debugView == DEBUG_PROBE_SECOND) {
        float t = (push.debugView == DEBUG_PROBE_NEAR) ? roots[0] * water.radii.x : roots[1] * water.radii.x;
        float hi = floor(t);
        float mid = floor(frac(t) * 1024.0);
        float lo = frac(t * 1024.0);
        float marker = -(float(hitCount) + (fallbackUsed ? 10.0 : 0.0));
        return debugOutput(float4(hi, mid, lo, marker), waterDepth);
    }

    float chord = 0.0;
    if (hitCount >= 2) {
        chord = (roots[1] - roots[0]) * water.radii.x;
    }
    if (hitCount >= 4) {
        chord += (roots[3] - roots[2]) * water.radii.x;
    }

    float3 pLocal1 = pLocalOrigin + roots[0] * dLocal;
    float rHat = water.radii.y / water.radii.x;
    float2 uv = torusUV(pLocal1);

    float du_dx = ddx(uv.x);
    float du_dy = ddy(uv.x);
    float dv_dx = ddx(uv.y);
    float dv_dy = ddy(uv.y);
    float2 footprint = float2(length(float2(du_dx, du_dy)), length(float2(dv_dx, dv_dy)));
    if (abs(du_dx) > 3.0) {
        footprint.x = 0.0;
    }
    if (any(isnan(footprint)) || any(isinf(footprint))) { footprint = float2(0.0); }

    float h, hu, hv, var;
    waterHeightAndGradient(uv, water.flow.z, water.flow.xy, int(water.composite.z), footprint, h, hu, hv, var);
    waterLbHeightAndGradient(uv, water.flow.z, water.flow.xy, h, hu, hv);
    float3 nLocal = waterPerturbedNormal(uv.x, uv.y, h, hu, hv, rHat);
    float3 n = normalize(mul(float3x3(water.model), nLocal));

    if (push.debugView == DEBUG_NORMAL) {
        return debugOutput(float4(n * 0.5 + 0.5, 1.0), waterDepth);
    }

    float eta = water.absorption.w;
    float cosThetaI = clamp(-dot(rayDir, n), 0.0, 1.0);
    float F = fresnelAverage(cosThetaI, eta);

#ifdef WATER_RAY_QUERY
    bool reentryTouched = false;
#endif
    float3 reflDir = reflect(rayDir, n);
    float3 reflection;
#ifdef WATER_RAY_QUERY
    if (push.secondaryRays == SECONDARY_RAY_QUERY) {
        float3 reflDirLocal = normalize(mul(water.inverseModel, float4(reflDir, 0.0)).xyz);
        float3 reflOriginLocal = pLocal1 + reflDirLocal * 1e-3;

        float tReflEntry = torusEntryFromOutside(reflOriginLocal, reflDirLocal, rHat);
        float tTorusNext = (tReflEntry > 0.0) ? (1e-3 + tReflEntry) * water.radii.x : 1e30;

        float3 rayColor;
        float tScene;
        if (traceScene(p1, reflDir, tTorusNext, rayColor, tScene)) {
            reflection = rayColor;
        } else if (tTorusNext < 1e30) {
            float3 p2 = p1 + reflDir * tTorusNext;
            float3 d2;
            float reentryWeight2, var2;
            bool traced;
            float3 depth2 = reentryBounce(p2, reflDir, footprint, rHat, eta, fragCoord.xy, d2, reentryWeight2, var2, traced);
            if (reentryWeight2 > 0.0) {
                reentryTouched = true;
            }
            if (!traced) {
                float3 lightDir2 = normalize(frame.light_pos.xyz - p2);
                depth2 = waterEnvironmentReflection(d2, lightDir2, frame.light_color.rgb, var2);
            }
            float3 lightDir = normalize(frame.light_pos.xyz - p1);
            float3 direct = waterEnvironmentReflection(reflDir, lightDir, frame.light_color.rgb, var);
            reflection = lerp(direct, depth2, reentryWeight2);
        } else {
            float3 lightDir = normalize(frame.light_pos.xyz - p1);
            reflection = waterEnvironmentReflection(reflDir, lightDir, frame.light_color.rgb, var);
        }
    } else
#endif
    {
        float3 lightDir = normalize(frame.light_pos.xyz - p1);
        reflection = waterEnvironmentReflection(reflDir, lightDir, frame.light_color.rgb, var);
    }

#ifdef WATER_RAY_QUERY
    int reflectionHitKind = waterTraceLastHitKind;
#endif

    float3 dRefr = refract(dLocal, nLocal, 1.0 / eta);
    if (length(dRefr) < 1e-4) {
        dRefr = reflect(dLocal, nLocal);
    }

    float tExit = torusExitFromInside(pLocal1 + dRefr * 1e-3, dRefr, rHat);
    float3 pExitLocal = (tExit > 0.0) ? pLocal1 + dRefr * (1e-3 + tExit) : pLocal1;

    float3 nExit = normalize(torusGradient(pExitLocal, rHat));
    float3 dExitLocal = refract(dRefr, -nExit, eta);
    if (length(dExitLocal) < 1e-4) {
        dRefr = reflect(dRefr, nExit);
        float tReExit = torusExitFromInside(pExitLocal + dRefr * 1e-3, dRefr, rHat);
        if (tReExit > 0.0) {
            pExitLocal = pExitLocal + dRefr * (1e-3 + tReExit);
        }
        nExit = normalize(torusGradient(pExitLocal, rHat));
        dExitLocal = refract(dRefr, -nExit, eta);
        if (length(dExitLocal) < 1e-4) {
            dExitLocal = dRefr;
        }
    }
    float3 dExit = normalize(mul(float3x3(water.model), dExitLocal));

    float4 pExitWorld = mul(water.model, float4(pExitLocal * water.radii.x, 1.0));
    float3 background;
    float tBackground = 1e30;
#ifdef WATER_RAY_QUERY
    if (push.secondaryRays == SECONDARY_RAY_QUERY) {
        float3 rayColor;
        if (traceScene(pExitWorld.xyz, dExit, 1e30, rayColor, tBackground)) {
            background = rayColor;
        } else {
            float tReEntry = torusEntryFromOutside(pExitLocal + dExitLocal * 1e-3, dExitLocal, rHat);
            background = sceneColorSampler.Sample(projectToScreen(pExitWorld, fragTexCoord)).rgb;
            if (tReEntry > 0.0) {
                float reT = (1e-3 + tReEntry) * water.radii.x;
                float3 pReEntryWorld = pExitWorld.xyz + dExit * reT;
                float3 dRe;
                float reentryWeight, varRe;
                bool traced;
                float3 reentryColor = reentryBounce(pReEntryWorld, dExit, footprint, rHat, eta, fragCoord.xy, dRe, reentryWeight, varRe, traced);
                if (reentryWeight > 0.0) {
                    reentryTouched = true;
                }
                if (!traced) {
                    reentryColor = sampleSceneColorBlurred(projectToScreen(float4(pReEntryWorld, 1.0), fragTexCoord));
                }
                background = lerp(background, reentryColor, reentryWeight);
            }
        }
    } else if (push.secondaryRays == SECONDARY_TRACE_PIPELINE) {
        background = waterTraceSampler.Sample(fragTexCoord).rgb;
    } else
#endif
    {
        background = sceneColorSampler.Sample(projectToScreen(pExitWorld, fragTexCoord)).rgb;
    }

#ifdef WATER_RAY_QUERY
    if (push.debugView == DEBUG_HIT_KIND) {
        if (waterTraceLastHitKind == WATER_TRACE_HIT_ON_SCREEN || reflectionHitKind == WATER_TRACE_HIT_ON_SCREEN) {
            return debugOutput(float4(0.0, 1.0, 0.0, 1.0), waterDepth);
        } else if (waterTraceLastHitKind == WATER_TRACE_HIT_OFF_SCREEN || reflectionHitKind == WATER_TRACE_HIT_OFF_SCREEN) {
            return debugOutput(float4(1.0, 0.0, 0.0, 1.0), waterDepth);
        } else if (reentryTouched) {
            return debugOutput(float4(1.0, 1.0, 0.0, 1.0), waterDepth);
        }
        return debugOutput(float4(0.0, 0.0, 1.0, 1.0), waterDepth);
    }

    if (push.secondaryRays == SECONDARY_RAY_QUERY && water.radii.z > 0.0 && tBackground < 1e29) {
        float R = water.radii.x;
        float cosV = clamp((length(pExitLocal.xz) - 1.0) / rHat, -1.0, 1.0);
        float kappa1 = 1.0 / (rHat * R);
        float kappa2 = cosV / ((1.0 + rHat * cosV) * R);
        float eta2 = water.absorption.w;
        float k1t = eta2 * kappa1;
        float k2t = eta2 * kappa2;
        float d = tBackground;
        float focus = 1.0 / (abs((1.0 - d * k1t) * (1.0 - d * k2t)) + 0.05);
        float caustic = lerp(1.0, clamp(focus, 0.0, 4.0), clamp(water.radii.z, 0.0, 2.0) * 0.5);
        background *= caustic;
    }
#endif

    float3 transmission = lerp(background, water.tint.rgb, clamp(water.tint.a, 0.0, 1.0)) * rteTransmittance(waterExtinctionCoefficient(), chord);
    float3 lightDirExit = normalize(frame.light_pos.xyz - pExitWorld.xyz);
    transmission += waterTransmittedHighlight(dExit, lightDirExit, frame.light_color.rgb, chord);

    float scatterPath = length(pExitWorld.xyz - p1);
    if (scatterPath > 1e-6) {
        float3 viewDirWater = (pExitWorld.xyz - p1) / scatterPath;
        float ds = scatterPath / float(WATER_SCATTER_SAMPLES);
        for (int i = 0; i < WATER_SCATTER_SAMPLES; ++i) {
            WaterScatterSample smp = waterScatterSampleAt(p1, pExitWorld.xyz, i, frame.light_pos.xyz);
#ifdef WATER_RAY_QUERY
            if (push.secondaryRays == SECONDARY_RAY_QUERY && waterLightOccluded(smp.lightExitPoint, frame.light_pos.xyz)) {
                continue;
            }
#endif
            transmission += waterScatterSampleRadiance(smp, viewDirWater, frame.light_color.rgb, ds);
        }
    }

    float4 outColor = float4(F * reflection * water.composite.x + (1.0 - F) * transmission * water.composite.y, 1.0);

#ifdef WATER_RAY_QUERY
    if (push.secondaryRays == SECONDARY_TRACE_PIPELINE) {
        outColor = float4(waterTraceSampler.Sample(fragTexCoord).rgb, 1.0);
    }

    float4 current = outColor;
    bool currentNonFinite = any(isnan(outColor)) || any(isinf(outColor));
    if (currentNonFinite) {
        current = float4(0.0, 0.0, 0.0, 1.0);
    }
    float4 blended = current;
    if (water.temporal.x > 0.0) {
        float4 history = waterHistorySampler.Sample(fragTexCoord);
        if (!any(isnan(history)) && !any(isinf(history))) {
            blended = lerp(current, history, water.temporal.x);
        }
    }
    Output output;
    output.depth = waterDepth;
    output.history = blended;
    if (push.debugView == DEBUG_HISTORY_NAN) {
        if (currentNonFinite) {
            output.color = float4(0.0, 1.0, 0.0, 1.0);
        } else if (any(isnan(blended)) || any(isinf(blended))) {
            output.color = float4(1.0, 0.0, 1.0, 1.0);
        } else {
            output.color = float4(0.0);
        }
    } else {
        output.color = blended;
    }
    return output;
#else
    return debugOutput(outColor, waterDepth);
#endif
}
