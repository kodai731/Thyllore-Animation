import "water/include/surface.slang";
import "water/include/lb.slang";

export __extern_cpp void thylloreWaterHeightAndGradient(float2 uv, float time, float2 flowRate, int modeCount, float2 footprint, out float h, out float hu, out float hv, out float slopeVariance) {
    waterHeightAndGradient(uv, time, flowRate, modeCount, footprint, h, hu, hv, slopeVariance);
}

export __extern_cpp void thylloreWaterLbHeightAndGradient(float2 uv, float time, float2 flowRate, out float h, out float hu, out float hv) {
    float _h = 0.0;
    float _hu = 0.0;
    float _hv = 0.0;
    waterLbHeightAndGradient(uv, time, flowRate, _h, _hu, _hv);
    h = _h;
    hu = _hu;
    hv = _hv;
}
