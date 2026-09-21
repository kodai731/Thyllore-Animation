// Procedural noise for the styled flame raymarch (mode 3).
// IGN (Jimenez 2014, "Next Generation Post Processing in Call of Duty") jitters
// the per-pixel ray start so 8-step marching dithers instead of banding.
module flame_noise;

public float interleavedGradientNoise(float2 pixelCoord) {
    return fract(52.9829189 * fract(dot(pixelCoord, float2(0.06711056, 0.00583715))));
}

float hashCell(float3 cell) {
    float3 p = fract(cell * 0.3183099 + float3(0.1, 0.2, 0.3));
    p *= 17.0;
    return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}

public float valueNoise3(float3 p) {
    float3 cell = floor(p);
    float3 f = fract(p);
    float3 u = f * f * (3.0 - 2.0 * f);

    float n000 = hashCell(cell + float3(0.0, 0.0, 0.0));
    float n100 = hashCell(cell + float3(1.0, 0.0, 0.0));
    float n010 = hashCell(cell + float3(0.0, 1.0, 0.0));
    float n110 = hashCell(cell + float3(1.0, 1.0, 0.0));
    float n001 = hashCell(cell + float3(0.0, 0.0, 1.0));
    float n101 = hashCell(cell + float3(1.0, 0.0, 1.0));
    float n011 = hashCell(cell + float3(0.0, 1.0, 1.0));
    float n111 = hashCell(cell + float3(1.0, 1.0, 1.0));

    float nx00 = lerp(n000, n100, u.x);
    float nx10 = lerp(n010, n110, u.x);
    float nx01 = lerp(n001, n101, u.x);
    float nx11 = lerp(n011, n111, u.x);
    float nxy0 = lerp(nx00, nx10, u.y);
    float nxy1 = lerp(nx01, nx11, u.y);
    return lerp(nxy0, nxy1, u.z);
}

public float fbm3(float3 p) {
    float sum = 0.0;
    float amplitude = 0.5;
    for (int octave = 0; octave < 3; ++octave) {
        sum += amplitude * valueNoise3(p);
        p = p * 2.02 + float3(13.7);
        amplitude *= 0.5;
    }
    return sum;
}
