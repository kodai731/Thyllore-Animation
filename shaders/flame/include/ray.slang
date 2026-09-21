// Ray reconstruction and emission segment integration for flame passes.
module flame_ray;

import "include/ray.slang";

public float evaluateHeightAlongRay(float t, float hOrigin, float hDir) {
    return hOrigin + t * hDir;
}
