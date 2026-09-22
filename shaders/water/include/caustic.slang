// Fixed-point caustic accumulation shared by the splat and apply passes.
module water_caustic;

// Splat weights are accumulated into an r32ui image as weight * this scale.
public static const float CAUSTIC_ACCUM_FIXED_POINT_SCALE = 256.0;
public static const int CAUSTIC_FILTER_RADIUS = 2;
public static const float CAUSTIC_INTENSITY = 0.25;
public static const float CAUSTIC_MAX_IRRADIANCE = 1.5;
public static const float CAUSTIC_TORUS_RADIUS_GAIN_MAX = 2.0;
public static const float CAUSTIC_LIGHT_GAIN = 3.0;
