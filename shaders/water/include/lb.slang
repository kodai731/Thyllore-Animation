module water_lb;

import "include/chebyshev.slang";
import "include/radiative_transfer.slang";
import "water/include/component.slang";

static const int LB_MODE_COUNT = 4;
static const int LB_SLOTS_PER_MODE = 5;

float waterLbCheb(float4 lo, float4 hi, float t) {
    return evaluateChebyshev8(lo, hi, 0.5 * t + 0.5);
}

float glslMod(float x, float y) {
    return x - y * floor(x / y);
}

public void waterLbHeightAndGradient(float2 uv, float time, float2 flowRate, inout float h, inout float hu, inout float hv) {
    for (int k = 0; k < LB_MODE_COUNT; ++k) {
        int slot = LB_SLOTS_PER_MODE * k;
        float4 head = water.lbModes[slot];
        float m = head.x;
        float omega = head.y;
        float amplitude = head.z;
        float phase = head.w;

        if (amplitude <= 0.0) {
            continue;
        }

        float phasePrime = m * (uv.x + flowRate.x * time) - omega * time + phase;
        float vAdvected = glslMod(uv.y + flowRate.y * time, TWO_PI);
        float t = (vAdvected - PI) / PI;

        float phi = waterLbCheb(water.lbModes[slot + 1], water.lbModes[slot + 2], t);
        float dphi = waterLbCheb(water.lbModes[slot + 3], water.lbModes[slot + 4], t);

        h += amplitude * cos(phasePrime) * phi;
        hu += -amplitude * m * sin(phasePrime) * phi;
        hv += amplitude * cos(phasePrime) * dphi;
    }
}
