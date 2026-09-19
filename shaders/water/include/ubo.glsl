#ifndef WATER_UBO_GLSL
#define WATER_UBO_GLSL

struct WaterUBO {
    mat4 model;
    mat4 inverseModel;
    vec4 radii;
    vec4 absorption;
    vec4 flow;
    vec4 composite;
    vec4 tint;
    vec4 lighting;
    vec4 scattering;
    vec4 temporal;
    vec4 waveModes[16];
    mat4 invViewProj;
    vec4 lbModes[20];
};

#endif
