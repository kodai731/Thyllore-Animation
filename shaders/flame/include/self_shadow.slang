module flame_self_shadow;

import "include/chebyshev.slang";
import "flame/include/component.slang";

public float evaluateHeightPrimitive(float height01) {
    return evaluateChebyshev12(
        flame.heightPrimitiveCoefficients[0],
        flame.heightPrimitiveCoefficients[1],
        flame.heightPrimitiveCoefficients[2],
        height01);
}

public float computeSelfShadowTau(float3 p, float3 l) {
    // Layer radii S = [1/3, 2/3, 1], midpoints m = [1/6, 0.5, 5/6]
    float s[3] = { 1.0/3.0, 2.0/3.0, 1.0 };
    float m[3] = { 1.0/6.0, 0.5, 5.0/6.0 };

    // Evaluate density at each layer midpoint using Chebyshev coefficients
    float dens[4];
    for (int k = 0; k < 3; ++k) {
        dens[k] = evaluateChebyshev8(flame.radialCoefficients[0], flame.radialCoefficients[1], m[k]);
    }
    dens[3] = 0.0;

    // Compute weights w_k = dens_k - dens_{k+1}
    float w[3];
    for (int k = 0; k < 3; ++k) {
        w[k] = dens[k] - dens[k + 1];
    }

    float px = p.x, py = p.y, pz = p.z;
    float lx = l.x, ly = l.y, lz = l.z;

    float total = 0.0;

    for (int k = 0; k < 3; ++k) {
        float sk = s[k];
        float a = lx * lx + lz * lz;

        // Find intersection of cylinder (x^2 + z^2 = S_k^2) and ray p + s*L
        float s0, s1;
        if (a < 1e-6) {
            // Ray is parallel to cylinder axis
            if (px * px + pz * pz <= sk * sk) {
                s0 = 0.0;
                s1 = 1e4;
            } else {
                continue;
            }
        } else {
            // Solve quadratic: a*s^2 + 2*(px*lx + pz*lz)*s + (px^2 + pz^2 - sk^2) = 0
            float b = 2.0 * (px * lx + pz * lz);
            float c = px * px + pz * pz - sk * sk;
            float disc = b * b - 4.0 * a * c;

            if (disc <= 0.0) {
                continue;
            }

            float sqrt_disc = sqrt(disc);
            s0 = (-b - sqrt_disc) / (2.0 * a);
            s1 = (-b + sqrt_disc) / (2.0 * a);

            // Clip to s >= 0
            if (s1 < 0.0) {
                continue;
            }
            if (s0 < 0.0) {
                s0 = 0.0;
            }
        }

        // Clip interval by height h(s) = p.y + s*L.y in [0, 1]
        float lo = s0;
        float hi = s1;

        if (abs(ly) < 1e-4) {
            // h is approximately constant
            if (py < 0.0 || py > 1.0) {
                continue;
            }
            // F is coefficients.height evaluated at p.y
            float f_val = evaluateChebyshev8(flame.heightCoefficients[0], flame.heightCoefficients[1], py);
            total += w[k] * f_val * (hi - lo);
        } else {
            // h(s) = py + s*ly, find where h in [0, 1]
            float s_lo = (0.0 - py) / ly;
            float s_hi = (1.0 - py) / ly;

            if (s_lo > s_hi) {
                float tmp = s_lo;
                s_lo = s_hi;
                s_hi = tmp;
            }

            // Clip [lo, hi] by [s_lo, s_hi]
            lo = max(lo, s_lo);
            hi = min(hi, s_hi);

            if (lo >= hi) {
                continue;
            }

            // I_k = (H1(h(s1)) - H1(h(s0))) / L.y
            float h_s0 = py + lo * ly;
            float h_s1 = py + hi * ly;

            float h1_s0 = evaluateHeightPrimitive(h_s0);
            float h1_s1 = evaluateHeightPrimitive(h_s1);

            total += w[k] * (h1_s1 - h1_s0) / ly;
        }
    }

    return max(0.0, flame.sigmaT * total);
}
