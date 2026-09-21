// C ABI surface of the volume_shell module for the CPU (C++) target; scalar parameters keep the
// generated C++ free of Slang vector and struct types at the boundary.
import volume_shell;

VolumeShell makeShell(
    float height, float radiusBase, float radiusSlope, float radiusOffsetQ, float widthQ,
    float strength, float hTop, float topFade, float sigmaT) {
    VolumeShell shell;
    shell.height = height;
    shell.radiusBase = radiusBase;
    shell.radiusSlope = radiusSlope;
    shell.radiusOffsetQ = radiusOffsetQ;
    shell.widthQ = widthQ;
    shell.strength = strength;
    shell.hTop = hTop;
    shell.topFade = topFade;
    shell.sigmaT = sigmaT;
    return shell;
}

export __extern_cpp float thylloreShellOpticalDepth(
    float height, float radiusBase, float radiusSlope, float radiusOffsetQ, float widthQ,
    float strength, float hTop, float topFade, float sigmaT,
    float ox, float oy, float oz, float dx, float dy, float dz, float tNear, float tFar) {
    VolumeShell shell = makeShell(height, radiusBase, radiusSlope, radiusOffsetQ, widthQ, strength, hTop, topFade, sigmaT);
    return shellOpticalDepth(shell, float3(ox, oy, oz), float3(dx, dy, dz), tNear, tFar);
}

export __extern_cpp float thylloreShellOpticalDepthToward(
    float height, float radiusBase, float radiusSlope, float radiusOffsetQ, float widthQ,
    float strength, float hTop, float topFade, float sigmaT,
    float ox, float oy, float oz, float dx, float dy, float dz, float tMax) {
    VolumeShell shell = makeShell(height, radiusBase, radiusSlope, radiusOffsetQ, widthQ, strength, hTop, topFade, sigmaT);
    return shellOpticalDepthToward(shell, float3(ox, oy, oz), float3(dx, dy, dz), tMax);
}

export __extern_cpp float thylloreShellDensityAt(
    float height, float radiusBase, float radiusSlope, float radiusOffsetQ, float widthQ,
    float strength, float hTop, float topFade, float sigmaT,
    float px, float py, float pz) {
    VolumeShell shell = makeShell(height, radiusBase, radiusSlope, radiusOffsetQ, widthQ, strength, hTop, topFade, sigmaT);
    return shellDensityAt(shell, float3(px, py, pz));
}
