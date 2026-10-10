[[vk::binding(0, 0)]] RWStructuredBuffer<uint> histogram;
[[vk::binding(1, 0)]] RWStructuredBuffer<float> result;

struct PushConstants {
    float lowPercent;
    float highPercent;
    float timeDelta;
    float adaptationSpeedUp;
    float adaptationSpeedDown;
    float minEv;
    float maxEv;
    float minLogLuminance;
    float logLuminanceRange;
    uint pixelCount;
};

[[vk::push_constant]] ConstantBuffer<PushConstants> pc;

groupshared uint sharedHistogram[256];

[shader("compute")]
[numthreads(256, 1, 1)]
void main(uint3 globalInvocationId : SV_DispatchThreadID, uint localIndex : SV_GroupIndex) {
    sharedHistogram[localIndex] = histogram[localIndex];
    histogram[localIndex] = 0;
    GroupMemoryBarrierWithGroupSync();

    if (localIndex != 0) {
        return;
    }

    uint totalPixels = pc.pixelCount;
    uint lowCount = uint(float(totalPixels) * pc.lowPercent);
    uint highCount = uint(float(totalPixels) * pc.highPercent);

    uint accumulated = 0;
    uint trimmedCount = 0;
    float weightedLogSum = 0.0;

    for (uint i = 0; i < 256; i++) {
        uint binCount = sharedHistogram[i];
        uint newAccumulated = accumulated + binCount;

        uint effectiveLow = max(accumulated, lowCount);
        uint effectiveHigh = min(newAccumulated, highCount);

        if (effectiveHigh > effectiveLow) {
            uint contributing = effectiveHigh - effectiveLow;
            float binLogLum =
                pc.minLogLuminance
                + (float(i) - 0.5) / 254.0
                * pc.logLuminanceRange;

            if (i == 0) {
                binLogLum = pc.minLogLuminance;
            }

            weightedLogSum += binLogLum * float(contributing);
            trimmedCount += contributing;
        }

        accumulated = newAccumulated;
    }

    float avgLogLum = (trimmedCount > 0)
        ? weightedLogSum / float(trimmedCount)
        : pc.minLogLuminance;

    float avgLum = exp2(avgLogLum);

    float targetEv = log2(max(avgLum, 0.00001) * 100.0 / 12.5);
    targetEv = clamp(targetEv, pc.minEv, pc.maxEv);

    float targetExposure = 1.0 / (1.2 * exp2(targetEv));

    float prevExposure = result[1];
    if (prevExposure <= 0.0) {
        prevExposure = targetExposure;
    }

    float speed = (targetExposure > prevExposure)
        ? pc.adaptationSpeedUp
        : pc.adaptationSpeedDown;

    float adapted = prevExposure
        + (targetExposure - prevExposure)
        * (1.0 - exp(-pc.timeDelta * speed));

    result[0] = avgLum;
    result[1] = adapted;
}
