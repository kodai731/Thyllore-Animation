// depth.glsl - Reverse-Z depth utilities
//
// This engine uses Reverse-Z depth mapping:
//   Near plane = 1.0, Far plane = 0.0
//   Depth comparison for "closer wins" = GREATER_OR_EQUAL
//
// Shader authors should use these constants and helpers instead of
// hardcoding depth values. This keeps reverse-Z knowledge in one place.

#ifndef DEPTH_GLSL
#define DEPTH_GLSL

// Canonical depth constants (reverse-Z)
const float DEPTH_FAR  = 0.0;
const float DEPTH_NEAR = 1.0;

// Convert world position to clip-space depth value suitable for gl_FragDepth.
//
// Usage:
//   gl_FragDepth = worldToClipDepth(worldPos, view, proj);
float worldToClipDepth(vec3 worldPos, mat4 view, mat4 proj) {
    vec4 clipPos = proj * view * vec4(worldPos, 1.0);
    return clipPos.z / clipPos.w;
}

// Convert raw depth buffer value to linear eye-space distance.
//
// With reverse-Z the projection maps:
//   z_near -> 1.0,  z_far -> 0.0
// The relationship is:  rawDepth = (near / z) for infinite far plane,
// or more generally:     rawDepth = near * far / (far - z * (far - near))
//
// This function inverts that mapping.
//
// Usage:
//   float linearDist = linearizeDepth(gl_FragCoord.z, nearPlane, farPlane);
float linearizeDepth(float rawDepth, float nearPlane, float farPlane) {
    return nearPlane * farPlane / (farPlane - rawDepth * (farPlane - nearPlane));
}

#endif
