// Sample-based reference for the closed-form integral (debug mode only): midpoint
// quadrature of the pointwise density. Never part of the product path.
module reference_quadrature;

import field;

public float windReferenceOpticalDepth(float3 o, float3 d, float tNear, float tFar, int stepCount) {
    int steps = max(stepCount, 1);
    float step = (tFar - tNear) / float(steps);
    float total = 0.0;
    for (int i = 0; i < steps; ++i) {
        float t = tNear + (float(i) + 0.5) * step;
        total += windDensityAt(o + d * t);
    }
    return total * step;
}
