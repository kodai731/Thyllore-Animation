module scene;

public struct SceneData {
    public float4 lightPosition;
    public float4 lightColor;
    public float4x4 view;
    public float4x4 proj;
    public int debugMode;
    public float shadowStrength;
    public int enableDistanceAttenuation;
    public float exposureValue;
    public float4 lighting;
    public float4 cameraPosition;
};
