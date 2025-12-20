#version 450

// Gizmo用のUniform Buffer（カメラの回転行列のみ）
layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;

layout(location = 0) out vec3 fragColor;

void main() {
    // カメラの回転行列を抽出（view行列から平行移動成分を除去）
    mat3 rotation = mat3(ubo.view);

    // Gizmoの頂点にカメラの回転のみを適用
    vec3 rotatedPos = rotation * inPosition;

    // 画面右上に配置するためのオフセット
    // NDC座標系: x: [-1, 1], y: [-1, 1]
    // 右上の位置: x = 0.75, y = -0.75 (Y-down coordinate system)
    vec2 gizmoOffset = vec2(0.75, -0.75);

    // Gizmoのスケール（小さく表示）
    float gizmoScale = 0.15;

    // 最終的な位置を計算（透視投影なしで直接NDC座標に配置）
    vec4 position = vec4(rotatedPos * gizmoScale, 1.0);
    position.xy += gizmoOffset;
    position.z = 0.0; // 深度は固定（常に手前に表示）

    gl_Position = position;
    fragColor = inColor;
}
