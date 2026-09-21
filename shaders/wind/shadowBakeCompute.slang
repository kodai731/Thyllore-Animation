import "wind/include/component.slang";
import "wind/include/field.slang";
import "wind/include/integral.slang";
import "wind/include/shadow_volume.slang";
import "include/frame.slang";
import "include/shadow_rays.slang";
import "include/shadow_volume.slang";

[[vk::binding(1, 1)]] [format("rg16f")] RWTexture3D<float2> shadowVolumeImage;

[shader("compute")]
[numthreads(8, 8, 1)]
void main(uint3 globalInvocationId : SV_DispatchThreadID) {
    int3 texel = int3(globalInvocationId);
    if (texel.x >= WindShadowGrid.RADIAL || texel.y >= WindShadowGrid.HEIGHT || texel.z >= WindShadowGrid.THETA) {
        return;
    }

    WindShadowGrid grid;
    float3 position = shadowVolumeTexelPosition(grid, texel);
    float3 lightPositionLocal = mul(wind.inverseModel, float4(frame.light_pos.xyz, 1.0)).xyz;
    float3 lightDir = normalize(lightPositionLocal - position);
    WindShadowMedium medium;
    float sun = sunTransmittance(medium, position, lightDir);
    float sky = skyTransmittance(medium, position);

    int3 slotTexel = texel + int3(windShadowSlot() * WindShadowGrid.RADIAL, 0, 0);
    shadowVolumeImage[slotTexel] = float2(sun, sky);
}
