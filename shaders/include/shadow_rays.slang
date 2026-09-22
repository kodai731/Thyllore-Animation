// Sun and sky transmittance of a participating medium from the optical depth its shadow rays
// accumulate. The effect implements IShadowMedium for its own medium.
module shadow_rays;

import "include/math.slang";
import "include/radiative_transfer.slang";

public static const int SKY_TILTED_DIRECTIONS = 6;
public static const float SKY_TILT_COSINE = 0.70710678;

public interface IShadowMedium {
    float shadowOpticalDepthToward(float3 origin, float3 direction);
}

public float sunTransmittance<M : IShadowMedium>(M medium, float3 position, float3 lightDir) {
    return rteTransmittanceFromOpticalDepth(medium.shadowOpticalDepthToward(position, lightDir));
}

// Cosine-weighted average over the zenith and six directions tilted 45 degrees, so a uniform
// sky lights the medium from every side instead of through one vertical shadow ray.
public float skyTransmittance<M : IShadowMedium>(M medium, float3 position) {
    float weightedSum = rteTransmittanceFromOpticalDepth(
        medium.shadowOpticalDepthToward(position, float3(0.0, 1.0, 0.0)));
    float weightSum = 1.0;
    for (int i = 0; i < SKY_TILTED_DIRECTIONS; ++i) {
        float azimuth = TWO_PI * float(i) / float(SKY_TILTED_DIRECTIONS);
        float3 direction = float3(
            cos(azimuth) * SKY_TILT_COSINE, SKY_TILT_COSINE, sin(azimuth) * SKY_TILT_COSINE);
        weightedSum += SKY_TILT_COSINE
            * rteTransmittanceFromOpticalDepth(medium.shadowOpticalDepthToward(position, direction));
        weightSum += SKY_TILT_COSINE;
    }
    return weightedSum / weightSum;
}
