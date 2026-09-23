// Screen-space outline of a pixel class over an object id image: a pixel is on the outline
// when a neighbour within `width` texels belongs to the other class.
module outline;

public interface IPixelClass {
    bool contains(uint id);
}

public bool detectClassEdge<Class : IPixelClass>(
    Class pixelClass, Sampler2D<uint4> idImage, float2 uv, float2 texelSize, float width) {
    bool centerInside = pixelClass.contains(idImage.Sample(uv).r);
    for (int dy = -1; dy <= 1; dy++) {
        for (int dx = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) continue;
            float2 offset = float2(float(dx), float(dy)) * texelSize * width;
            if (pixelClass.contains(idImage.Sample(uv + offset).r) != centerInside) {
                return true;
            }
        }
    }
    return false;
}
