// dL/ds = -sigma_t L + sigma_a L_e + sigma_s * integral p(theta) L_in
// Each effect supplies its own coefficients; this module holds only the equation.
module radiative_transfer;

import "include/math.slang";

public float rteTransmittance(float sigmaT, float distance) {
    return exp(-sigmaT * distance);
}

public float3 rteTransmittance(float3 sigmaT, float distance) {
    return exp(-sigmaT * distance);
}

public float rteTransmittanceFromOpticalDepth(float opticalDepth) {
    return exp(-opticalDepth);
}

public float rteOpacity(float sigmaT, float opticalDepth) {
    return 1.0 - exp(-sigmaT * opticalDepth);
}

// S * (1 - exp(-sigma*dt)) / sigma with Taylor fallback so sigma -> 0 stays continuous.
public float rteIntegrateEmissionSegment(float source, float sigmaT, float dt) {
    float x = sigmaT * dt;
    if (x < 1e-3) {
        return source * dt * (1.0 - 0.5 * x + x * x * (1.0 / 6.0));
    }
    return source * (1.0 - exp(-x)) / sigmaT;
}

public float rteHenyeyGreenstein(float cosTheta, float g) {
    float denom = 1.0 + g * g - 2.0 * g * cosTheta;
    return (1.0 - g * g) / (4.0 * PI * denom * sqrt(max(denom, 1e-6)));
}

public float rteMidpointDistance(int index, int sampleCount, float pathLength) {
    return (float(index) + 0.5) * pathLength / float(sampleCount);
}

public float3 rteSingleScatterSample(float3 sigmaS, float3 sigmaT, float phase, float viewDistance, float ds, float3 lightRadiance) {
    return sigmaS * phase * rteTransmittance(sigmaT, viewDistance) * lightRadiance * ds;
}
