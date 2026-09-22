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

// Cubic Hermite fade 3t^2 - 2t^3 (the smoothstep curve on [0, 1]).
public float hermiteFade(float t) {
    return t * t * (3.0 - 2.0 * t);
}

// IGN (Jimenez 2014, "Next Generation Post Processing in Call of Duty"): a per-pixel
// ray start jitter so a short march dithers instead of banding.
public float interleavedGradientNoise(float2 pixelCoord) {
    return fract(52.9829189 * fract(dot(pixelCoord, float2(0.06711056, 0.00583715))));
}

public interface IHash3 {
    float hash(float3 cell);
}

static const float CELL_HASH_FREQUENCY = 0.3183099;
static const float3 CELL_HASH_OFFSET = float3(0.1, 0.2, 0.3);
static const float CELL_HASH_SPREAD = 17.0;

// Fractional-product lattice hash (ALU only, no trigonometry).
public struct CellHash : IHash3 {
    public float hash(float3 cell) {
        float3 p = fract(cell * CELL_HASH_FREQUENCY + CELL_HASH_OFFSET);
        p *= CELL_HASH_SPREAD;
        return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
    }
};

static const float3 SIN_HASH_DIRECTION = float3(127.1, 311.7, 74.7);
static const float SIN_HASH_SCALE = 43758.5453123;

// Classic sin-dot lattice hash.
public struct SinHash : IHash3 {
    public float hash(float3 cell) {
        return fract(sin(dot(cell, SIN_HASH_DIRECTION)) * SIN_HASH_SCALE);
    }
};

// Value noise on the unit lattice with a Hermite fade, in [0, 1].
public float valueNoise3<Hash : IHash3>(Hash hash, float3 p) {
    float3 cell = floor(p);
    float3 f = fract(p);
    float3 u = float3(hermiteFade(f.x), hermiteFade(f.y), hermiteFade(f.z));

    float n000 = hash.hash(cell + float3(0.0, 0.0, 0.0));
    float n100 = hash.hash(cell + float3(1.0, 0.0, 0.0));
    float n010 = hash.hash(cell + float3(0.0, 1.0, 0.0));
    float n110 = hash.hash(cell + float3(1.0, 1.0, 0.0));
    float n001 = hash.hash(cell + float3(0.0, 0.0, 1.0));
    float n101 = hash.hash(cell + float3(1.0, 0.0, 1.0));
    float n011 = hash.hash(cell + float3(0.0, 1.0, 1.0));
    float n111 = hash.hash(cell + float3(1.0, 1.0, 1.0));

    float nx00 = lerp(n000, n100, u.x);
    float nx10 = lerp(n010, n110, u.x);
    float nx01 = lerp(n001, n101, u.x);
    float nx11 = lerp(n011, n111, u.x);
    float nxy0 = lerp(nx00, nx10, u.y);
    float nxy1 = lerp(nx01, nx11, u.y);
    return lerp(nxy0, nxy1, u.z);
}

public struct FbmParams {
    public int octaves;
    public float lacunarity;
    public float gain;
    public float3 octaveShift;
};

// Octave sum of value noise; each octave scales p by lacunarity and offsets it by octaveShift.
public float fbm3<Hash : IHash3>(Hash hash, FbmParams params, float3 p) {
    float sum = 0.0;
    float amplitude = params.gain;
    for (int octave = 0; octave < params.octaves; ++octave) {
        sum += amplitude * valueNoise3(hash, p);
        p = p * params.lacunarity + params.octaveShift;
        amplitude *= params.gain;
    }
    return sum;
}
