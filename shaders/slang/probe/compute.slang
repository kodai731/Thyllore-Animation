// Compute probe that exercises the volume_shell module on the GPU targets (SPIR-V, GLSL).
import volume_shell;

struct ProbeRay {
    float3 origin;
    float tNear;
    float3 direction;
    float tFar;
};

ConstantBuffer<VolumeShell> shell;
StructuredBuffer<ProbeRay> rays;
RWStructuredBuffer<float> opticalDepth;

[shader("compute")]
[numthreads(64, 1, 1)]
void probeMain(uint3 id : SV_DispatchThreadID) {
    ProbeRay ray = rays[id.x];
    opticalDepth[id.x] = shellOpticalDepth(shell, ray.origin, ray.direction, ray.tNear, ray.tFar);
}
