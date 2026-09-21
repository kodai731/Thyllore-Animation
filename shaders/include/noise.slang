// Hash and gradient noise on the unit lattice (ALU only, no textures).
module noise;

public uint3 pcg3d(uint3 v) {
    v = v * 1664525u + 1013904223u;
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    v ^= v >> 16u;
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    return v;
}

public float hash13(float3 p) {
    uint3 h = pcg3d(asuint(p));
    return float(h.x) * (1.0 / 4294967296.0);
}

// Perlin's improved fade 6t^5 - 15t^4 + 10t^3.
public float quinticFade(float t) {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

public float3 latticeGradient3(float3 cell) {
    float3 g = float3(
        2.0 * hash13(cell) - 1.0,
        2.0 * hash13(cell + float3(17.1, 9.3, 4.7)) - 1.0,
        2.0 * hash13(cell + float3(31.7, 2.9, 12.3)) - 1.0);
    return g / max(length(g), 1e-4);
}

public float latticeCornerDot3(float3 cell, float3 f, float3 corner) {
    return dot(latticeGradient3(cell + corner), f - corner);
}

// Perlin gradient noise on the unit lattice, zero at every lattice point.
public float gradientNoise3(float3 p) {
    float3 cell = floor(p);
    float3 f = p - cell;
    float3 w = float3(quinticFade(f.x), quinticFade(f.y), quinticFade(f.z));

    float nx00 = lerp(latticeCornerDot3(cell, f, float3(0.0, 0.0, 0.0)), latticeCornerDot3(cell, f, float3(1.0, 0.0, 0.0)), w.x);
    float nx10 = lerp(latticeCornerDot3(cell, f, float3(0.0, 1.0, 0.0)), latticeCornerDot3(cell, f, float3(1.0, 1.0, 0.0)), w.x);
    float nx01 = lerp(latticeCornerDot3(cell, f, float3(0.0, 0.0, 1.0)), latticeCornerDot3(cell, f, float3(1.0, 0.0, 1.0)), w.x);
    float nx11 = lerp(latticeCornerDot3(cell, f, float3(0.0, 1.0, 1.0)), latticeCornerDot3(cell, f, float3(1.0, 1.0, 1.0)), w.x);
    float nxy0 = lerp(nx00, nx10, w.y);
    float nxy1 = lerp(nx01, nx11, w.y);
    return lerp(nxy0, nxy1, w.z);
}
