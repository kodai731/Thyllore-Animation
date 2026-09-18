#ifndef LIGHTNING_COMPONENT_GLSL
#define LIGHTNING_COMPONENT_GLSL

layout(set = 1, binding = 0) uniform LightningUBO {
    mat4 model;
    mat4 inverseModel;
    vec4 core;
    vec4 glow;
    vec4 shape;
    vec4 debug_;
    mat4 invViewProj;
    vec4 flash;
} lightning;

layout(set = 1, binding = 1) uniform LightningSegmentsUBO {
    vec4 seg_a_r0[256];
    vec4 seg_b_r1[256];
    vec4 seg_misc[256];
} segments;

#endif
