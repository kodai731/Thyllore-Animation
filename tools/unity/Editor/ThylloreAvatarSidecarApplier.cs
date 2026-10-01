#if UNITY_EDITOR

using System;
using System.Collections.Generic;
using System.IO;
using UnityEngine;
using UnityEditor;
using VRC.SDK3.Avatars.Components;
using VRC.SDK3.Dynamics.PhysBone.Components;
using VRC.SDKBase;

namespace Thyllore.AvatarTools
{
    public class ThylloreAvatarSidecarWindow : EditorWindow
    {
        private GameObject avatar_root;
        private string sidecar_path;
        private string loaded_path;
        private AvatarSidecar loaded_sidecar;
        private string load_error;

        [MenuItem("Tools/Thyllore/Apply Avatar Sidecar")]
        public static void ShowWindow()
        {
            GetWindow<ThylloreAvatarSidecarWindow>("Apply Avatar Sidecar");
        }

        private void OnGUI()
        {
            GUILayout.Label("Avatar Root", EditorStyles.boldLabel);
            avatar_root = (GameObject)EditorGUILayout.ObjectField(avatar_root, typeof(GameObject), true);

            GUILayout.Space(8);
            GUILayout.Label("Sidecar JSON", EditorStyles.boldLabel);
            sidecar_path = EditorGUILayout.TextField(sidecar_path);
            if (GUILayout.Button("Browse"))
            {
                var selected_path = EditorUtility.OpenFilePanel("Select Avatar Sidecar", "", "json");
                if (!string.IsNullOrEmpty(selected_path))
                {
                    sidecar_path = selected_path;
                    GUI.FocusControl(null);
                }
            }

            ReloadSidecarIfPathChanged();
            if (!string.IsNullOrEmpty(load_error))
            {
                EditorGUILayout.HelpBox($"Failed to load sidecar: {load_error}", MessageType.Error);
            }

            if (loaded_sidecar != null)
            {
                DrawSummary();
                DrawApplyButtons();
            }
        }

        private void ReloadSidecarIfPathChanged()
        {
            if (sidecar_path == loaded_path)
            {
                return;
            }

            loaded_path = sidecar_path;
            loaded_sidecar = null;
            load_error = null;
            if (string.IsNullOrEmpty(sidecar_path) || !File.Exists(sidecar_path))
            {
                return;
            }

            try
            {
                loaded_sidecar = AvatarSidecarLoader.Load(sidecar_path);
            }
            catch (Exception ex)
            {
                load_error = ex.Message;
            }
        }

        private void DrawSummary()
        {
            GUILayout.Space(8);
            GUILayout.Label("Sidecar Summary", EditorStyles.boldLabel);
            EditorGUILayout.LabelField("Humanoid bones", loaded_sidecar.humanoid.Count.ToString());
            EditorGUILayout.LabelField("Visemes", loaded_sidecar.visemes.Count.ToString());
            EditorGUILayout.LabelField("Expressions", loaded_sidecar.expressions.Count.ToString());
            EditorGUILayout.LabelField("Spring chains", loaded_sidecar.spring_chains.Count.ToString());
            EditorGUILayout.LabelField("Expression mesh", loaded_sidecar.expression_mesh ?? "(none)");
        }

        private void DrawApplyButtons()
        {
            GUILayout.Space(8);
            using (new EditorGUI.DisabledScope(avatar_root == null))
            {
                if (GUILayout.Button("Apply Humanoid"))
                {
                    AvatarSidecarApplier.ApplyHumanoid(avatar_root, loaded_sidecar);
                }
                if (GUILayout.Button("Apply Visemes & Blink"))
                {
                    AvatarSidecarApplier.ApplyVisemesAndBlink(avatar_root, loaded_sidecar);
                }
                if (GUILayout.Button("Apply Spring Chains"))
                {
                    AvatarSidecarApplier.ApplySpringChains(avatar_root, loaded_sidecar);
                }
            }
        }
    }

    public static class AvatarSidecarApplier
    {
        private static GameObject FindModelAsset(GameObject root)
        {
            var source = PrefabUtility.GetCorrespondingObjectFromSource(root);
            if (source == null)
            {
                Debug.LogError($"Avatar root \"{root.name}\" is not an instance of a model asset.");
            }
            return source;
        }

        private static ModelImporter FindModelImporter(GameObject modelAsset)
        {
            var assetPath = AssetDatabase.GetAssetPath(modelAsset);
            var importer = AssetImporter.GetAtPath(assetPath) as ModelImporter;
            if (importer == null)
            {
                Debug.LogError($"Not a model asset: {assetPath}");
            }
            return importer;
        }

        private static SkeletonBone[] BuildSkeleton(GameObject modelAsset)
        {
            var skeleton = new List<SkeletonBone>();
            foreach (var transform in modelAsset.GetComponentsInChildren<Transform>(true))
            {
                skeleton.Add(new SkeletonBone
                {
                    name = transform.name,
                    position = transform.localPosition,
                    rotation = transform.localRotation,
                    scale = transform.localScale
                });
            }
            return skeleton.ToArray();
        }

        private static Transform FindTransformByName(GameObject root, string name)
        {
            foreach (Transform t in root.GetComponentsInChildren<Transform>(true))
            {
                if (t.name == name)
                    return t;
            }
            return null;
        }

        public static void ApplyHumanoid(GameObject root, AvatarSidecar sidecar)
        {
            if (root == null)
            {
                Debug.LogError("Avatar root is not assigned.");
                return;
            }

            var modelAsset = FindModelAsset(root);
            if (modelAsset == null)
            {
                return;
            }
            var importer = FindModelImporter(modelAsset);
            if (importer == null)
            {
                return;
            }

            var desc = importer.humanDescription;
            var bones = BuildHumanBones(sidecar);

            desc.human = bones;
            desc.skeleton = BuildSkeleton(modelAsset);
            importer.humanDescription = desc;
            importer.animationType = ModelImporterAnimationType.Human;
            importer.SaveAndReimport();

            Debug.Log($"Applied humanoid mapping ({bones.Length} bones) to {importer.assetPath}");
        }

        private static HumanBone[] BuildHumanBones(AvatarSidecar sidecar)
        {
            var bones = new List<HumanBone>();
            foreach (var entry in sidecar.humanoid)
            {
                if (!Enum.TryParse(entry.Key, out HumanBodyBones bodyBone) || bodyBone == HumanBodyBones.LastBone)
                {
                    Debug.LogWarning($"Unknown humanoid bone \"{entry.Key}\" in sidecar — skipping.");
                    continue;
                }

                bones.Add(new HumanBone
                {
                    humanName = HumanTrait.BoneName[(int)bodyBone],
                    boneName = entry.Value,
                    limit = { useDefaultValues = true }
                });
            }
            return bones.ToArray();
        }

        public static void ApplyVisemesAndBlink(GameObject root, AvatarSidecar sidecar)
        {
            if (root == null)
            {
                Debug.LogError("Avatar root is not assigned.");
                return;
            }

            var renderer = FindExpressionRenderer(root, sidecar.expression_mesh);
            if (renderer == null || renderer.sharedMesh == null)
            {
                Debug.LogError($"Expression mesh \"{sidecar.expression_mesh}\" not found under avatar root.");
                return;
            }

            var vcad = root.GetComponent<VRCAvatarDescriptor>();
            if (vcad == null)
            {
                vcad = Undo.AddComponent<VRCAvatarDescriptor>(root);
                Debug.Log($"Added VRCAvatarDescriptor to {root.name}");
            }

            Undo.RecordObject(vcad, "Apply Visemes & Blink");
            ApplyVisemes(vcad, renderer, sidecar);
            ApplyBlink(vcad, renderer, sidecar);
            EditorUtility.SetDirty(vcad);
        }

        private static SkinnedMeshRenderer FindExpressionRenderer(GameObject root, string meshName)
        {
            foreach (var smr in root.GetComponentsInChildren<SkinnedMeshRenderer>(true))
            {
                if (smr.gameObject.name == meshName)
                    return smr;
            }
            return null;
        }

        private static void ApplyVisemes(VRCAvatarDescriptor vcad, SkinnedMeshRenderer renderer, AvatarSidecar sidecar)
        {
            var visemeOrder = new[]
            {
                "sil", "pp", "ff", "th", "dd", "kk", "ch", "ss", "nn", "rr",
                "aa", "e", "ih", "oh", "ou"
            };

            var blendShapes = new string[visemeOrder.Length];
            for (int i = 0; i < visemeOrder.Length; i++)
            {
                if (sidecar.visemes.TryGetValue(visemeOrder[i], out var name))
                {
                    blendShapes[i] = name;
                }
                else
                {
                    blendShapes[i] = string.Empty;
                    Debug.LogWarning($"Viseme \"{visemeOrder[i]}\" missing from sidecar — leaving empty.");
                }
            }

            vcad.lipSync = VRC_AvatarDescriptor.LipSyncStyle.VisemeBlendShape;
            vcad.VisemeSkinnedMesh = renderer;
            vcad.VisemeBlendShapes = blendShapes;
            Debug.Log($"Applied {sidecar.visemes.Count} visemes from {renderer.name}");
        }

        private static void ApplyBlink(VRCAvatarDescriptor vcad, SkinnedMeshRenderer renderer, AvatarSidecar sidecar)
        {
            var blinkName = !string.IsNullOrEmpty(sidecar.blink.both) ? sidecar.blink.both : sidecar.blink.left;
            if (string.IsNullOrEmpty(blinkName))
            {
                Debug.Log("No blink channel in sidecar, eyelids unchanged.");
                return;
            }

            var blinkIndex = renderer.sharedMesh.GetBlendShapeIndex(blinkName);
            if (blinkIndex < 0)
            {
                Debug.LogWarning($"Blink blend shape \"{blinkName}\" not found on {renderer.name}, eyelids unchanged.");
                return;
            }

            vcad.enableEyeLook = true;
            vcad.customEyeLookSettings.eyelidType = VRCAvatarDescriptor.EyelidType.Blendshapes;
            vcad.customEyeLookSettings.eyelidsSkinnedMesh = renderer;
            vcad.customEyeLookSettings.eyelidsBlendshapes = new[] { blinkIndex, -1, -1 };
            Debug.Log($"Applied blink \"{blinkName}\" (index {blinkIndex})");
        }

        public static void ApplySpringChains(GameObject root, AvatarSidecar sidecar)
        {
            if (root == null)
            {
                Debug.LogError("Avatar root is not assigned.");
                return;
            }

            foreach (var chain in sidecar.spring_chains)
            {
                var transform = FindTransformByName(root, chain.root);
                if (transform == null)
                {
                    Debug.LogWarning($"Spring chain root \"{chain.root}\" not found — skipping.");
                    continue;
                }

                var bone = transform.GetComponent<VRCPhysBone>();
                if (bone == null)
                {
                    bone = Undo.AddComponent<VRCPhysBone>(transform.gameObject);
                    Debug.Log($"Added VRCPhysBone to {chain.root}");
                }

                Undo.RecordObject(bone, "Apply Spring Chain");
                MapSpringToPhysBone(chain, bone);
                EditorUtility.SetDirty(bone);
                Debug.Log($"Applied spring chain to {chain.root} (pull={bone.pull}, spring={bone.spring}, gravity={bone.gravity})");
            }
        }

        private static void MapSpringToPhysBone(SidecarSpringChain chain, VRCPhysBone bone)
        {
            bone.pull = Mathf.Clamp01(chain.stiffness);
            bone.spring = Mathf.Clamp01(1f - chain.drag);
            bone.gravity = Mathf.Clamp01(chain.gravity);
        }
    }
}

#endif
