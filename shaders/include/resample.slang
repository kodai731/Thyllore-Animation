// Separable tent kernels and the Jimenez 2014 ("Next Generation Post Processing in Call of Duty")
// 13-tap downsample, over any sampler so every pass and mip level shares one implementation.
module resample;

// Triangular weight of an integer tap at `offset` inside a kernel of `radius`: 1, 2, .., radius + 1, .., 2, 1.
public uint tentWeight(int offset, int radius) {
    return uint(radius + 1 - abs(offset));
}

// Sum of the 2D tent weights over the (2 radius + 1)^2 taps.
public uint tentWeightSum(int radius) {
    uint axisSum = uint((radius + 1) * (radius + 1));
    return axisSum * axisSum;
}

// (2 radius + 1)^2 tap tent filter; radius 1 is the 9-tap bloom upsample.
public float3 tentFilter(Sampler2D source, float2 uv, float2 texelSize, int radius) {
    float3 sum = float3(0.0);
    for (int dy = -radius; dy <= radius; ++dy) {
        for (int dx = -radius; dx <= radius; ++dx) {
            float weight = float(tentWeight(dx, radius) * tentWeight(dy, radius));
            sum += weight * source.Sample(uv + texelSize * float2(float(dx), float(dy))).rgb;
        }
    }
    return sum / float(tentWeightSum(radius));
}

// Karis average: weight by 1 / (1 + luma) so a single hot pixel cannot flicker the chain.
public float3 karisAverage(float3 c) {
    float luma = dot(c, float3(0.2126, 0.7152, 0.0722));
    return c / (1.0 + luma);
}

struct Taps13 {
    float3 outerCorner[4];
    float3 outerEdge[4];
    float3 innerCorner[4];
    float3 center;
};

Taps13 sample13(Sampler2D source, float2 uv, float2 texelSize) {
    Taps13 taps;
    taps.outerCorner[0] = source.Sample(uv + texelSize * float2(-1.0, -1.0)).rgb;
    taps.outerCorner[1] = source.Sample(uv + texelSize * float2( 1.0, -1.0)).rgb;
    taps.outerCorner[2] = source.Sample(uv + texelSize * float2(-1.0,  1.0)).rgb;
    taps.outerCorner[3] = source.Sample(uv + texelSize * float2( 1.0,  1.0)).rgb;
    taps.outerEdge[0] = source.Sample(uv + texelSize * float2( 0.0, -1.0)).rgb;
    taps.outerEdge[1] = source.Sample(uv + texelSize * float2(-1.0,  0.0)).rgb;
    taps.outerEdge[2] = source.Sample(uv + texelSize * float2( 1.0,  0.0)).rgb;
    taps.outerEdge[3] = source.Sample(uv + texelSize * float2( 0.0,  1.0)).rgb;
    taps.innerCorner[0] = source.Sample(uv + texelSize * float2(-0.5, -0.5)).rgb;
    taps.innerCorner[1] = source.Sample(uv + texelSize * float2( 0.5, -0.5)).rgb;
    taps.innerCorner[2] = source.Sample(uv + texelSize * float2(-0.5,  0.5)).rgb;
    taps.innerCorner[3] = source.Sample(uv + texelSize * float2( 0.5,  0.5)).rgb;
    taps.center = source.Sample(uv).rgb;
    return taps;
}

// 13-tap downsample: centre 1/8, inner corners 1/8 each, outer edges 1/16 each, outer corners 1/32 each.
public float3 downsample13Tap(Sampler2D source, float2 uv, float2 texelSize) {
    Taps13 taps = sample13(source, uv, texelSize);
    float3 result = taps.center * 0.125;
    result += (taps.outerEdge[0] + taps.outerEdge[1] + taps.outerEdge[2] + taps.outerEdge[3]) * 0.0625;
    result += (taps.innerCorner[0] + taps.innerCorner[1] + taps.innerCorner[2] + taps.innerCorner[3]) * 0.125;
    result += (taps.outerCorner[0] + taps.outerCorner[1] + taps.outerCorner[2] + taps.outerCorner[3]) * 0.03125;
    return result;
}

// First downsample of a chain: the 13 taps grouped into five 2x2 boxes, each Karis averaged before blending.
public float3 downsample13TapKaris(Sampler2D source, float2 uv, float2 texelSize) {
    Taps13 taps = sample13(source, uv, texelSize);
    float3 topLeft = karisAverage((taps.outerCorner[0] + taps.outerEdge[0] + taps.outerEdge[1] + taps.innerCorner[0]) * 0.25);
    float3 topRight = karisAverage((taps.outerEdge[0] + taps.outerCorner[1] + taps.innerCorner[0] + taps.innerCorner[1]) * 0.25);
    float3 bottomLeft = karisAverage((taps.outerEdge[1] + taps.innerCorner[0] + taps.outerCorner[2] + taps.innerCorner[2]) * 0.25);
    float3 bottomRight = karisAverage((taps.innerCorner[0] + taps.innerCorner[1] + taps.innerCorner[2] + taps.innerCorner[3]) * 0.25);
    float3 center = karisAverage(taps.center);
    return (topLeft + topRight + bottomLeft + bottomRight) * 0.125 + center * 0.5;
}
