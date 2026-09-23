import volume_shell;

export __extern_cpp float thylloreShellOpticalDepth(VolumeShell shell, float3 o, float3 d, float tNear, float tFar) {
    return shellOpticalDepth(shell, o, d, tNear, tFar);
}

export __extern_cpp float thylloreShellOpticalDepthToward(VolumeShell shell, float3 o, float3 d, float tMax) {
    return shellOpticalDepthToward(shell, o, d, tMax);
}

export __extern_cpp float thylloreShellDensityAt(VolumeShell shell, float3 p) {
    return shellDensityAt(shell, p);
}
