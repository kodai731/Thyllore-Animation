#ifndef TRACE_PUSH_GLSL
#define TRACE_PUSH_GLSL

layout(push_constant) uniform TracePush {
    mat4 invViewProj;
    vec4 cameraPos;
    vec4 lightPos;
    vec4 lightColor;
} trace;

#endif
