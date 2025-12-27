#version 450

// Fullscreen triangle vertex shader (no vertex buffer needed)
// Generates a fullscreen triangle that covers the entire viewport

layout(location = 0) out vec2 fragTexCoord;

void main() {
    // Generate fullscreen triangle using vertex ID
    // This technique draws a single triangle that covers the entire screen
    // without needing a vertex buffer
    //
    // Vertex 0: position (-1, -1), UV (0, 0)
    // Vertex 1: position ( 3, -1), UV (2, 0)
    // Vertex 2: position (-1,  3), UV (0, 2)
    //
    // The triangle extends beyond the viewport, but only the visible portion
    // [(-1,-1) to (1,1)] will be rasterized with UV coordinates [0,1]

    // Generate UV coordinates [0, 2] using bit tricks
    // gl_VertexIndex 0: (0, 0)
    // gl_VertexIndex 1: (2, 0)
    // gl_VertexIndex 2: (0, 2)
    vec2 uv = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);

    // Pass texture coordinates to fragment shader
    // The visible screen area will have UV in [0, 1]
    fragTexCoord = uv * 0.5;

    // Convert UV [0, 2] to NDC [-1, 3]
    gl_Position = vec4(uv * 2.0 - 1.0, 0.0, 1.0);
}
