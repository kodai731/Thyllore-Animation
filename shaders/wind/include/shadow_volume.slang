// Wall + envelope transmittance cache of the wind cone on the grid of include/shadow_volume.slang.
// Extents are mirrored in thyllore-effect-core/src/wind/gpu/components/shadow_volume.rs.
module wind_shadow_volume;

import "wind/include/component.slang";
import "wind/include/field.slang";
import "include/shadow_volume.slang";

public struct WindShadowGrid : IShadowVolumeGrid {
    public static const int RADIAL = 48;
    public static const int HEIGHT = 48;
    public static const int THETA = 64;
    public static const int SLOTS = 4;

    public float topY() {
        return windHTop() * windHeight();
    }

    public float wallRadius(float y) {
        return sqrt(max(windWallRadiusSq(y / windHeight()), 0.0));
    }

    public float radialExtent() {
        return windShadowRadialExtent();
    }
};
