// Ray segment types shared by the flame resolve entry and the reference march.
module flame_segment;

import "include/ray.slang";

public struct FlameDepthClamp {
    public float sceneDepth;
    public float tDepth;
    public int state;
};

public struct FlameRaySegment {
    public float tNear;
    public float tFar;
    public float3 localOrigin;
    public float3 localDir;
    public float boundaryHeightIntegral;
    public FlameDepthClamp depthClamp;
    public bool cylinderDomain;
};

public float evaluateHeightAlongRay(float t, float hOrigin, float hDir) {
    return hOrigin + t * hDir;
}
