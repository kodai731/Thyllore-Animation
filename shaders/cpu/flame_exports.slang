import "flame/include/branch_transport.slang";
import "flame/include/self_shadow.slang";

export __extern_cpp float3 thylloreFlameBranchPullBack(float3 p) {
    return flameBranchPullBack(p);
}

export __extern_cpp void thylloreFlameBranchPullBackJvp(float3 p, float3 dirIn, out float3 pulled, out float3 dirOut) {
    float3 dir = dirIn;
    float3 result = flameBranchPullBackJvp(p, dir);
    pulled = result;
    dirOut = dir;
}

export __extern_cpp float thylloreFlameBranchBurnoutMask(float3 p) {
    return flameBranchBurnoutMask(p);
}

export __extern_cpp float thylloreFlameSelfShadowTau(float3 p, float3 l) {
    return computeSelfShadowTau(p, l);
}
