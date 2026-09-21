// Ray reconstruction and emission segment integration for flame passes.
module flame_ray;

import "include/ray.slang";

// resolveFragment declares these between two of its #includes, so the GLSL
// reference march reads types its includer supplies. Slang needs them in a
// module the march can import.
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
