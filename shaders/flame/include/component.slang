// Every struct shared with the Rust side lives here. FlameUBO and its members are
// generated into thyllore-effect-core/src/flame/gpu/components/generated.rs
// (cargo run -p thyllore-shader-manifest --bin generate_gpu_blocks); VortexElement
// is mirrored by hand in flame/branch.rs.
module flame_component;

import "include/chebyshev.slang";

public struct FlameColorBase {
    public float3 rgb;
    public float occlusionLumRef;
};

public struct FlameColorMid {
    public float3 rgb;
    public float pad0;
};

public struct FlameColorTip {
    public float3 rgb;
    public float pad0;
};

public struct FlameTemporalParams {
    public float accumWeight;
    public float frameIndex;
    public float noiseAnisoY;
    public float warpYScale;
};

public struct FlameLightParams {
    public float3 direction;
    public float selfShadowStrength;
};

public struct FlameWarpStyle {
    public float warpAmp;
    public float warpFreq;
    public float riseSpeed;
    public float taperPower;
};

public struct FlameEdgeStyle {
    public float radiusTipRatio;
    public float edgeLow;
    public float edgeHigh;
    public float whiteBoost;
};

public struct FlameWindBend {
    public float2 windDirection;
    public float bendAmount;
    public float bendPower;
};

public struct FlameTrailMeta {
    public float sampleCount;
    public float maxAge;
    public float pad0;
    public float pad1;
};

public struct FlameEmitterParams {
    public float kind;
    public float ringMajorRatio;
    public float ringAngularSpeed;
    // Gaussian half-depth of the SDF billboard slab (emitter kind 2).
    public float sdfSlabDepth;
};

public struct FlameContourParams {
    public float wiggleAmp;
    public float anisoAxisAdvect;
    public float rteBands;
    public float sigmaDispersion;
};

// Bridge model of smoothstep(edgeLow, edgeHigh, x): two gaussians around a center (ErfResponseModel).
public struct FlameErosionResponse {
    public float center;
    public float kappa;
    public float weight1;
    public float weight2;
};

public struct FlameWaveCfParams {
    public float enabled;
    public float shearLayerCount;
    public float skippedPowerPlain;
    public float skippedPowerEnv;
};

public struct FlameBoundaryParams {
    public float amp;
    public float freq;
    public float speed;
    public float radiusRatio;
};

// Near fade plus the amplitude-scaled effective erosion edge window.
public struct FlameNearFadeParams {
    public float radius;
    public float carveResidual;
    public float edgeLow;
    public float edgeHigh;
};

public struct FlameProfileParams {
    public float radiusActive;
    public float radiusMax;
    public float colorActive;
    public float pad0;
};

public struct FlameWaveShaping {
    public float trackedCount;
    public float envCoeff;
    public float inverseScale;
    public float amplitude;
};

public struct FlameTipCarveParams {
    public float depth;
    public float invReach;
    public float primitiveTop;
    public float invPrimitiveRange;
};

public struct FlameWarpStrainParams {
    public float strainBase;
    public float strainTip;
    public float invReach;
    public float invStrainNorm;
};

public struct FlameWarpFormParams {
    public float displacementForm;
    public float burnoutGain;
    public float pad0;
    public float pad1;
};

public struct FlameUnifiedParams {
    public float enabled;
    public float sigmaFloor;
    public float pad0;
    public float pad1;
};

public struct FlameMixParams {
    public float lo;
    public float hi;
    public float invCarrierStd;
    public float heightGain;
    // Wavenumber scale of the mixing eddies relative to the low erosion octave.
    public float scale;
    public float radialGain;
    public float pad0;
    public float pad1;
};

public struct FlameThermalParams {
    public float densityExp;
    public float tempExp;
    public float tempHotK;
    public float tempColdK;
    public float wienCK;
    public float pad0;
    public float pad1;
    public float pad2;
};

public struct FlameSegmentParams {
    public float count;
    public float invCount;
    public float pad0;
    public float pad1;
};

public struct FlameSpreadParams {
    public float gain;
    public float edgeOuterSharpen;
    public float twistGain;
    public float erosionNoiseGain;
};

public struct FlameSupportMotion {
    public float supportMargin;
    public float meanderAmp;
    public float swirlSpeed;
    // 0 = delegate the twist rate to swirlSpeed.
    public float twistSpeed;
};

public struct FlameTwistMode {
    public float kappa;
    public float omega;
    public float phase;
    public float amp;
};

public struct FlameTwistField {
    public FlameTwistMode modes[2];
    public float coreRadiusSq;
    public float pad0;
    public float pad1;
    public float pad2;
};

public struct FlameMeanderMode {
    public float2 direction;
    public float kappa;
    public float omega;
    public float phase;
    public float pad0;
    public float pad1;
    public float pad2;
};

public static const int FLAME_BRANCH_MAX_ELEMENTS = 32;

public struct FlameBranchElement {
    public float spawnTime;
    public float side;
    public float azimuth;
    public float spawnHeight;
    // Scatter lane: size multiplier of reach and core, line tilt out of the horizontal [rad],
    // and window center shift along the line in reach units.
    public float size;
    public float tilt;
    public float alongOffset;
    public float hash01;
    // Trunk support radius at the spawn height (flame-local); reach and core radii are ratios of it.
    public float trunkRadius;
    public float pad0;
    public float pad1;
    public float pad2;
};

// Age-profile fractions of the vortex transport (winding, burnout), evaluated identically on both sides.
public struct FlameBranchAgeProfile {
    public float windFraction;
    public float burnoutStartFraction;
    public float burnoutReleaseFraction;
    public float burnoutMargin;
    public float burnoutTrunkInner;
    public float pad0;
    public float pad1;
    public float pad2;
};

// Branch element table (newest first) with the per-effect age-profile constants; count = 0 is a no-op.
public struct FlameBranchField {
    public float count;
    public float period;
    public float life;
    public float gain;
    public float riseRate;
    public float driftRate;
    public float aspect;
    public float coreRadius;
    public float reachStart;
    public float reachEnd;
    public float envelopeTime;
    public float coreOffset;
    public float boundingPad;
    public float boundingPadY;
    public float pad1;
    public float pad2;
    public FlameBranchAgeProfile ageProfile;
    public FlameBranchElement elements[FLAME_BRANCH_MAX_ELEMENTS];
};

public struct FlameUBO {
    public float4x4 model;
    public float4x4 inverseModel;
    public float4 heightPrimitiveCoefficients[3];
    public float4 radialCoefficients[2];
    public float4 heightCoefficients[2];
    public float time;
    public float sigmaT;
    public float intensity;
    public float heightAxisScale;
    public float noiseAmplitude;
    public float noiseFrequency;
    public float noiseScrollSpeed;
    public float radialSharpness;
    public FlameColorBase colorBase;
    public FlameColorMid colorMid;
    public FlameColorTip colorTip;
    public FlameTemporalParams temporalData;
    public FlameLightParams lightData;
    public FlameWarpStyle warpStyle;
    public FlameEdgeStyle edgeStyle;
    public FlameWindBend windBend;
    public float4x4 trailUnitInverse;
    public FlameTrailMeta trailMeta;
    public float4 trail_coefficients[4];
    public FlameEmitterParams emitterParams;
    public FlameContourParams contourParams;
    public FlameErosionResponse erosionResponse;
    public FlameWaveCfParams waveCfParams;
    public FlameBoundaryParams boundaryParams;
    public FlameNearFadeParams nearFadeParams;
    public float4 radiusCoefficients[2];
    public float4 colorRamp[8];
    // Planckian chromaticity from temperatureTipK (index 0) to temperatureBaseK (index 7).
    public float4 tempRamp[8];
    public FlameProfileParams profileParams;
    public FlameWaveShaping waveParams;
    public FlameTipCarveParams tipCarveParams;
    public FlameWarpStrainParams warpStrainParams;
    public FlameWarpFormParams warpFormParams;
    public FlameUnifiedParams unifiedParams;
    public FlameMixParams mixParams;
    public FlameSegmentParams segmentParams;
    public FlameThermalParams thermalParams;
    public FlameSpreadParams spreadParams;
    public FlameSupportMotion supportMotion;
    public FlameTwistField twistField;
    public FlameMeanderMode meanderModes[2];
    public FlameBranchField branchField;
    public float4 waveModes[428];
    public float4 waveJitter[96];
};

[[vk::binding(0, 1)]] public ConstantBuffer<FlameUBO> flame;

[[vk::binding(5, 1)]] public Sampler2D flameSdfSampler;

public float evaluateHeightFalloff(float height01) {
    return evaluateChebyshev8(flame.heightCoefficients[0], flame.heightCoefficients[1], height01);
}

public struct FlameVortexElement {
    public float3 center;
    public float3 outward;
    public float3 line;
    public float3 up;
    public float reach;
    public float coreRadius;
    public float circulation;
    public float alongOffset;
};
