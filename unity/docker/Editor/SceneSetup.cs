#if UNITY_EDITOR

using System;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.Animations;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace Thyllore.AvatarTools
{
    public static class SceneSetup
    {
        private const string ScenePath = "Assets/Thyllore/Avatar.unity";
        private const string ControllerPath = "Assets/Thyllore/Expressions.controller";

        public static void Run()
        {
            try
            {
                Build();
            }
            catch (Exception ex)
            {
                Debug.LogError($"SCENESETUP EXCEPTION {ex}");
                EditorApplication.Exit(1);
                return;
            }

            Debug.Log("SCENESETUP DONE");
            EditorApplication.Exit(0);
        }

        public static void Open()
        {
            EditorSceneManager.OpenScene(ScenePath);
            var animator = UnityEngine.Object.FindObjectOfType<Animator>();
            if (animator != null)
            {
                Selection.activeGameObject = animator.gameObject;
            }
        }

        private static void Build()
        {
            var fbxAsset = Environment.GetEnvironmentVariable("THYLLORE_FBX_ASSET");
            var sidecarAsset = Environment.GetEnvironmentVariable("THYLLORE_SIDECAR_ASSET");
            var animDirAsset = Environment.GetEnvironmentVariable("THYLLORE_ANIM_DIR_ASSET");
            AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);

            var scene = EditorSceneManager.NewScene(NewSceneSetup.DefaultGameObjects, NewSceneMode.Single);
            var sidecar = LoadSidecar(sidecarAsset);
            if (sidecar != null)
            {
                var humanoidRoot = Instantiate(fbxAsset);
                AvatarSidecarApplier.ApplyHumanoid(humanoidRoot, sidecar);
                UnityEngine.Object.DestroyImmediate(humanoidRoot);
            }

            var root = Instantiate(fbxAsset);
            if (sidecar != null)
            {
                AvatarSidecarApplier.ApplyVisemesAndBlink(root, sidecar);
                AvatarSidecarApplier.ApplySpringChains(root, sidecar);
            }

            AttachExpressionController(root, animDirAsset);
            Selection.activeGameObject = root;
            EditorSceneManager.SaveScene(scene, ScenePath);
            AssetDatabase.SaveAssets();
        }

        private static AvatarSidecar LoadSidecar(string sidecarAsset)
        {
            if (string.IsNullOrEmpty(sidecarAsset) || !File.Exists(sidecarAsset))
            {
                Debug.LogWarning("No sidecar JSON next to the model, avatar left as imported.");
                return null;
            }
            var sidecar = AvatarSidecarLoader.Load(sidecarAsset);
            Debug.Log($"SCENESETUP sidecar humanoid={sidecar.humanoid.Count} visemes={sidecar.visemes.Count} expressions={sidecar.expressions.Count} spring_chains={sidecar.spring_chains.Count}");
            return sidecar;
        }

        private static GameObject Instantiate(string fbxAsset)
        {
            var prefab = AssetDatabase.LoadAssetAtPath<GameObject>(fbxAsset);
            if (prefab == null)
            {
                throw new FileNotFoundException($"model asset not imported: {fbxAsset}");
            }
            return (GameObject)PrefabUtility.InstantiatePrefab(prefab);
        }

        private static void AttachExpressionController(GameObject root, string animDirAsset)
        {
            var clips = LoadClips(animDirAsset);
            if (clips.Length == 0)
            {
                Debug.LogWarning("No .anim next to the model, no expression controller attached.");
                return;
            }

            AssetDatabase.DeleteAsset(ControllerPath);
            var controller = AnimatorController.CreateAnimatorControllerAtPath(ControllerPath);
            foreach (var clip in clips)
            {
                controller.AddMotion(clip);
            }

            var animator = root.GetComponent<Animator>();
            if (animator == null)
            {
                animator = root.AddComponent<Animator>();
            }
            animator.runtimeAnimatorController = controller;
            Debug.Log($"SCENESETUP controller states={clips.Length} ({string.Join(", ", clips.Select(c => c.name))})");
        }

        private static AnimationClip[] LoadClips(string animDirAsset)
        {
            if (string.IsNullOrEmpty(animDirAsset) || !Directory.Exists(animDirAsset))
            {
                return Array.Empty<AnimationClip>();
            }
            return Directory.GetFiles(animDirAsset, "*.anim")
                .OrderBy(path => path, StringComparer.Ordinal)
                .Select(path => AssetDatabase.LoadAssetAtPath<AnimationClip>(path.Replace('\\', '/')))
                .Where(clip => clip != null)
                .ToArray();
        }
    }
}

#endif
