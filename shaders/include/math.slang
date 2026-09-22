// Scalar constants and helpers shared by every module; effect math never re-declares them.
module math;

public static const float PI = 3.14159265358979;
public static const float TWO_PI = 6.28318530717959;
public static const float HALF_PI = 1.57079632679490;

// GLSL mod: x - y * floor(x / y), non-negative for y > 0 (fmod keeps the sign of x).
public float floorMod(float x, float y) {
    return x - y * floor(x / y);
}
