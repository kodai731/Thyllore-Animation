#if UNITY_EDITOR

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Newtonsoft.Json;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using VRC.SDK3.Avatars.Components;
using VRC.SDK3.Dynamics.PhysBone.Components;

namespace Thyllore.Avatar
{
    [Serializable]
    public class MorphTrackDump
    {
        [JsonProperty("clip")]
        public string clip;

        [JsonProperty("anim_file")]
        public string anim_file;

        [JsonProperty("duration")]
        public float duration;

        [JsonProperty("samples")]
        public List<MorphTrackDumpSample> samples = new List<MorphTrackDumpSample>();
    }

    [Serializable]
    public class MorphTrackDumpSample
    {
        [JsonProperty("time")]
        public float time;

        [JsonProperty("weights")]
        public List<MorphTrackDumpWeight> weights = new List<MorphTrackDumpWeight>();
    }

    [Serializable]
    public class MorphTrackDumpWeight
    {
        [JsonProperty("source_mesh")]
        public string source_mesh;

        [JsonProperty("channel")]
        public string channel;

        [JsonProperty("weight")]
        public float weight;
    }

    public static class BatchCheck
    {
        private const string AssetDir = "Assets/Avatar";
        private const float MorphWeightTolerance = 0.002f;
        private static int failures;

        private static void Check(bool condition, string label)
        {
            Debug.Log($"BATCHCHECK {(condition ? "PASS" : "FAIL")} {label}");
            if (!condition) failures++;
        }

        public static void Run()
        {
            try
            {
                RunChecks();
            }
            catch (Exception ex)
            {
                Debug.LogError($"BATCHCHECK EXCEPTION {ex}");
                failures++;
            }

            Debug.Log($"BATCHCHECK DONE failures={failures}");
            EditorApplication.Exit(failures == 0 ? 0 : 1);
        }

        private static void RunChecks()
        {
            var fbxSource = Environment.GetEnvironmentVariable("THYLLORE_FBX");
            var sidecarPath = Environment.GetEnvironmentVariable("THYLLORE_SIDECAR");
            var expressionAnimSource = Environment.GetEnvironmentVariable("THYLLORE_EXPRESSION_ANIM");
            var morphSamplesPath = Environment.GetEnvironmentVariable("THYLLORE_MORPH_SAMPLES");

            CheckPackageIdentity();
            Directory.CreateDirectory(AssetDir);
            var fbxAsset = Path.Combine(AssetDir, Path.GetFileName(fbxSource));
            File.Copy(fbxSource, fbxAsset, true);
            AssetDatabase.ImportAsset(fbxAsset, ImportAssetOptions.ForceSynchronousImport);

            EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var sidecar = AvatarSidecarLoader.Load(sidecarPath);
            Check(sidecar.schema == 2, "sidecar schema");
            Check(sidecar.humanoid.Count == 19, $"sidecar humanoid count {sidecar.humanoid.Count}");
            Check(sidecar.visemes.Count == 15, $"sidecar viseme count {sidecar.visemes.Count}");

            var root = Instantiate(fbxAsset);
            AvatarSidecarApplier.ApplyHumanoid(root, sidecar);
            CheckHumanoid(fbxAsset);

            root = Instantiate(fbxAsset);
            AvatarSidecarApplier.ApplyVisemesAndBlink(root, sidecar);
            CheckVisemesAndBlink(root, sidecar);

            AvatarSidecarApplier.ApplySpringChains(root, sidecar);
            CheckSpringChains(root, sidecar);

            CheckExpressions(root, sidecar);
            CheckAnimClip(expressionAnimSource);

            CheckMorphTrackAnimation(root, morphSamplesPath);
        }

        private static void CheckPackageIdentity()
        {
            var package = UnityEditor.PackageManager.PackageInfo.FindForAssembly(typeof(Package).Assembly);
            Check(package != null, "editor scripts come from a package");
            if (package == null) return;
            Check(package.name == Package.Name, $"package name {package.name} == {Package.Name}");
            Check(package.displayName == Package.DisplayName, $"package displayName {package.displayName} == {Package.DisplayName}");
        }

        private static GameObject Instantiate(string fbxAsset)
        {
            foreach (var old in UnityEngine.Object.FindObjectsOfType<GameObject>())
            {
                if (old.transform.parent == null) UnityEngine.Object.DestroyImmediate(old);
            }
            var prefab = AssetDatabase.LoadAssetAtPath<GameObject>(fbxAsset);
            Check(prefab != null, "fbx prefab loaded");
            return (GameObject)PrefabUtility.InstantiatePrefab(prefab);
        }

        private static void CheckHumanoid(string fbxAsset)
        {
            var importer = AssetImporter.GetAtPath(fbxAsset) as ModelImporter;
            Check(importer.animationType == ModelImporterAnimationType.Human, "importer animationType Human");
            Check(importer.humanDescription.human.Length == 19, $"humanDescription bones {importer.humanDescription.human.Length}");

            var avatar = AssetDatabase.LoadAllAssetsAtPath(fbxAsset).OfType<UnityEngine.Avatar>().FirstOrDefault();
            Check(avatar != null, "avatar sub-asset exists");
            if (avatar == null) return;
            Check(avatar.isValid, "avatar isValid");
            Check(avatar.isHuman, "avatar isHuman");

            var hips = importer.humanDescription.human.First(h => h.humanName == "Hips");
            Check(hips.boneName == "Hips", $"Hips -> {hips.boneName}");
            var leftUpperArm = importer.humanDescription.human.First(h => h.humanName == "LeftUpperArm");
            Check(leftUpperArm.boneName == "Upper_arm.L", $"LeftUpperArm -> {leftUpperArm.boneName}");
        }

        private static void CheckVisemesAndBlink(GameObject root, AvatarSidecar sidecar)
        {
            var vcad = root.GetComponent<VRCAvatarDescriptor>();
            Check(vcad != null, "VRCAvatarDescriptor added");
            if (vcad == null) return;

            Check(vcad.lipSync == VRC.SDKBase.VRC_AvatarDescriptor.LipSyncStyle.VisemeBlendShape, "lipSync VisemeBlendShape");
            Check(vcad.VisemeSkinnedMesh != null && vcad.VisemeSkinnedMesh.name == "Body", "viseme mesh Body");
            Check(vcad.VisemeBlendShapes.Length == 15, "15 viseme slots");
            Check(vcad.VisemeBlendShapes[0] == "vrc.v_sil", $"viseme[0]={vcad.VisemeBlendShapes[0]}");
            Check(vcad.VisemeBlendShapes[10] == "vrc.v_aa", $"viseme[10]={vcad.VisemeBlendShapes[10]}");
            Check(vcad.VisemeBlendShapes[14] == "vrc.v_ou", $"viseme[14]={vcad.VisemeBlendShapes[14]}");

            var mesh = vcad.VisemeSkinnedMesh.sharedMesh;
            foreach (var name in vcad.VisemeBlendShapes)
            {
                Check(mesh.GetBlendShapeIndex(name) >= 0, $"viseme blend shape exists on mesh: {name}");
            }

            Check(vcad.enableEyeLook, "eye look enabled");
            Check(vcad.customEyeLookSettings.eyelidType == VRCAvatarDescriptor.EyelidType.Blendshapes, "eyelid type Blendshapes");
            var blinkIndex = mesh.GetBlendShapeIndex("eye_blink");
            Check(vcad.customEyeLookSettings.eyelidsBlendshapes[0] == blinkIndex, $"blink index {vcad.customEyeLookSettings.eyelidsBlendshapes[0]} == {blinkIndex}");
        }

        private static void CheckSpringChains(GameObject root, AvatarSidecar sidecar)
        {
            Check(sidecar.spring_chains.Count > 0, $"sidecar spring chains {sidecar.spring_chains.Count}");
            foreach (var chain in sidecar.spring_chains)
            {
                var bone = root.GetComponentsInChildren<Transform>(true).First(t => t.name == chain.root).GetComponent<VRCPhysBone>();
                Check(bone != null, $"VRCPhysBone added to {chain.root}");
                if (bone == null) continue;
                Check(Mathf.Approximately(bone.pull, Mathf.Clamp01(chain.stiffness)), $"{chain.root} pull {bone.pull}");
                Check(Mathf.Approximately(bone.spring, Mathf.Clamp01(1f - chain.drag)), $"{chain.root} spring {bone.spring}");
                Check(Mathf.Approximately(bone.gravity, Mathf.Clamp01(chain.gravity)), $"{chain.root} gravity {bone.gravity}");
            }
        }

        private static void CheckExpressions(GameObject root, AvatarSidecar sidecar)
        {
            var mesh = root.GetComponentsInChildren<SkinnedMeshRenderer>(true).First(s => s.name == sidecar.expression_mesh).sharedMesh;
            Check(sidecar.expressions.Count > 0, $"sidecar expressions {sidecar.expressions.Count}");
            foreach (var expression in sidecar.expressions)
            {
                foreach (var weight in expression.weights)
                {
                    Check(mesh.GetBlendShapeIndex(weight.Key) >= 0, $"expression {expression.name} channel {weight.Key}={weight.Value}");
                }
            }
        }

        private static AnimationClip ImportAnimClip(string animSource)
        {
            var animAsset = Path.Combine(AssetDir, Path.GetFileName(animSource));
            File.Copy(animSource, animAsset, true);
            AssetDatabase.ImportAsset(animAsset, ImportAssetOptions.ForceSynchronousImport);
            var clip = AssetDatabase.LoadAssetAtPath<AnimationClip>(animAsset);
            Check(clip != null, $"anim clip loaded: {Path.GetFileName(animSource)}");
            return clip;
        }

        private static void CheckAnimClip(string animSource)
        {
            var clip = ImportAnimClip(animSource);
            if (clip == null) return;

            var bindings = AnimationUtility.GetCurveBindings(clip);
            Check(bindings.Length > 0, $"anim curve bindings {bindings.Length}");
            foreach (var binding in bindings)
            {
                var curve = AnimationUtility.GetEditorCurve(clip, binding);
                Debug.Log($"BATCHCHECK INFO binding path='{binding.path}' type={binding.type.Name} prop={binding.propertyName} keys={curve.length}");
            }
            Debug.Log($"BATCHCHECK INFO clip length={clip.length} frameRate={clip.frameRate}");
        }

        private static void CheckMorphTrackAnimation(GameObject root, string morphSamplesPath)
        {
            var dump = JsonConvert.DeserializeObject<MorphTrackDump>(File.ReadAllText(morphSamplesPath));
            Check(dump.samples.Count > 1, $"engine morph samples {dump.samples.Count}");

            var clip = ImportAnimClip(Path.Combine(Path.GetDirectoryName(morphSamplesPath), dump.anim_file));
            if (clip == null) return;
            Check(Mathf.Abs(clip.length - dump.duration) < 1e-3f, $"clip length {clip.length} == engine duration {dump.duration}");

            var renderers = root.GetComponentsInChildren<SkinnedMeshRenderer>(true).ToDictionary(r => r.name);
            var comparedWeights = 0;
            var maxError = 0f;
            var worstCase = "none";

            AnimationMode.StartAnimationMode();
            try
            {
                foreach (var sample in dump.samples)
                {
                    AnimationMode.BeginSampling();
                    AnimationMode.SampleAnimationClip(root, clip, sample.time);
                    AnimationMode.EndSampling();

                    foreach (var expected in sample.weights)
                    {
                        if (!renderers.TryGetValue(expected.source_mesh, out var renderer))
                        {
                            Check(false, $"mesh {expected.source_mesh} exists under avatar root");
                            return;
                        }
                        var blendShapeIndex = renderer.sharedMesh.GetBlendShapeIndex(expected.channel);
                        if (blendShapeIndex < 0)
                        {
                            Check(false, $"blend shape {expected.channel} exists on {expected.source_mesh}");
                            return;
                        }

                        var appliedWeight = renderer.GetBlendShapeWeight(blendShapeIndex) / 100f;
                        var error = Mathf.Abs(appliedWeight - expected.weight);
                        comparedWeights++;
                        if (error > maxError)
                        {
                            maxError = error;
                            worstCase = $"{expected.channel} at {sample.time:F3}s: engine {expected.weight:F4}, unity {appliedWeight:F4}";
                        }
                    }
                }
            }
            finally
            {
                AnimationMode.StopAnimationMode();
            }

            Check(comparedWeights > 0, $"compared {comparedWeights} weights over {dump.samples.Count} sample times");
            Check(maxError <= MorphWeightTolerance, $"morph animation matches engine: max error {maxError:F5} ({worstCase})");
        }
    }
}

#endif
