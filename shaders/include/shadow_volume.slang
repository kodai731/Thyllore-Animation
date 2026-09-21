// Cylindrical grid (radius, height, angle) that caches the transmittance toward the sun (x) and
// averaged over the sky (y) for every instance slot side by side along the radius axis. The
// radius axis is measured from the wall radius at the texel's height, so a shell sits on the
// same texels at every height and its steep transmittance gradient never drifts over texels.
// The effect implements IShadowVolumeGrid with its extents and wall radius.
module shadow_volume;

import "include/radiative_transfer.slang";

public interface IShadowVolumeGrid {
    static const int RADIAL;
    static const int HEIGHT;
    static const int THETA;
    static const int SLOTS;
    float topY();
    float wallRadius(float y);
    float radialExtent();
}

public float3 shadowVolumeTexelPosition<G : IShadowVolumeGrid>(G grid, int3 texel) {
    float y = (float(texel.y) + 0.5) / float(G.HEIGHT) * grid.topY();
    float wallOffset = ((float(texel.x) + 0.5) / float(G.RADIAL) * 2.0 - 1.0)
        * grid.radialExtent();
    float radius = max(grid.wallRadius(y) + wallOffset, 0.0);
    float theta = (float(texel.z) + 0.5) / float(G.THETA) * TWO_PI - PI;
    return float3(radius * cos(theta), y, radius * sin(theta));
}

// The radius coordinate is clamped to the slot's own texel centres so linear filtering never
// bleeds into the neighbouring instance; height clamps and angle wraps in the sampler.
public float3 shadowVolumeUvw<G : IShadowVolumeGrid>(G grid, float3 local, int slot) {
    float halfTexel = 0.5 / float(G.RADIAL);
    float wallOffset = length(local.xz) - grid.wallRadius(local.y);
    float radial = clamp(
        wallOffset / grid.radialExtent() * 0.5 + 0.5, halfTexel, 1.0 - halfTexel);
    float u = (float(slot) + radial) / float(G.SLOTS);
    float v = local.y / grid.topY();
    float w = (atan2(local.z, local.x) + PI) / TWO_PI;
    return float3(u, v, w);
}

// The height axis clamps to texel centres and the angle axis wraps by blending the two seam
// texels explicitly, so the result does not depend on the sampler's address modes.
public float2 shadowVolumeTransmittances<G : IShadowVolumeGrid>(G grid, Sampler3D volume, float3 local, int slot) {
    float3 uvw = shadowVolumeUvw(grid, local, slot);
    float halfTexelV = 0.5 / float(G.HEIGHT);
    uvw.y = clamp(uvw.y, halfTexelV, 1.0 - halfTexelV);

    float halfTexelW = 0.5 / float(G.THETA);
    float seamSpan = 2.0 * halfTexelW;
    if (uvw.z >= halfTexelW && uvw.z <= 1.0 - halfTexelW) {
        return volume.Sample(uvw).xy;
    }
    float seamOffset = uvw.z < halfTexelW ? uvw.z + halfTexelW : uvw.z - (1.0 - halfTexelW);
    float2 lastSlice = volume.Sample(float3(uvw.xy, 1.0 - halfTexelW)).xy;
    float2 firstSlice = volume.Sample(float3(uvw.xy, halfTexelW)).xy;
    return lerp(lastSlice, firstSlice, seamOffset / seamSpan);
}
