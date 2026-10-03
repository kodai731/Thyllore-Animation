#if UNITY_EDITOR

using System;
using System.IO;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace Thyllore.Avatar
{
    public static class FacePreview
    {
        private const int ImageSize = 1024;
        private const float FaceDistancePerHeight = 0.36f;
        private const float WideDistancePerHeight = 1.6f;
        private const float FaceOffsetUpPerHeight = 0.025f;
        private const float WideOffsetDownPerHeight = 0.4f;

        public static void Capture()
        {
            try
            {
                Run();
            }
            catch (Exception ex)
            {
                Debug.LogError($"FACEPREVIEW EXCEPTION {ex}");
                EditorApplication.Exit(1);
                return;
            }

            Debug.Log("FACEPREVIEW DONE");
            EditorApplication.Exit(0);
        }

        private static void Run()
        {
            var outputDir = Environment.GetEnvironmentVariable("THYLLORE_PREVIEW_DIR");
            var sidecarAsset = Environment.GetEnvironmentVariable("THYLLORE_SIDECAR_ASSET");
            var facing = Environment.GetEnvironmentVariable("THYLLORE_PREVIEW_FACING") == "-z" ? Vector3.back : Vector3.forward;
            Directory.CreateDirectory(outputDir);

            EditorSceneManager.OpenScene(SceneSetup.ScenePath);
            var animator = UnityEngine.Object.FindObjectOfType<Animator>();
            if (animator == null)
            {
                throw new InvalidOperationException("no Animator in the scene");
            }
            var root = animator.gameObject;
            var height = CollectBounds(root).size.y;
            LogRenderers(root);

            var faceCamera = CreateCamera(height * FaceDistancePerHeight);
            var wideCamera = CreateCamera(height * WideDistancePerHeight);
            AttachLight(faceCamera);
            CaptureBoth(faceCamera, wideCamera, animator, height, facing, outputDir, "neutral");

            var sidecar = AvatarSidecarLoader.Load(sidecarAsset);
            var renderer = FindRenderer(root, sidecar.expression_mesh);
            foreach (var expression in sidecar.expressions)
            {
                ApplyExpression(renderer, expression);
                using (new BakedStandIn(renderer))
                {
                    CaptureBoth(faceCamera, wideCamera, animator, height, facing, outputDir, expression.name);
                }
                ClearExpression(renderer, expression);
            }
            Debug.Log($"FACEPREVIEW captured {sidecar.expressions.Count + 1} expression(s) to {outputDir}");
        }

        private static void LogRenderers(GameObject root)
        {
            foreach (var renderer in root.GetComponentsInChildren<Renderer>())
            {
                var materials = new System.Collections.Generic.List<string>();
                foreach (var material in renderer.sharedMaterials)
                {
                    var texture = material != null && material.mainTexture != null ? material.mainTexture.name : "-";
                    materials.Add($"{(material != null ? material.name : "null")}[{texture}]");
                }
                Debug.Log($"FACEPREVIEW renderer {renderer.name} materials={string.Join(",", materials)}");
            }
        }

        private static Bounds CollectBounds(GameObject root)
        {
            var bounds = new Bounds(root.transform.position, Vector3.zero);
            foreach (var renderer in root.GetComponentsInChildren<Renderer>())
            {
                bounds.Encapsulate(renderer.bounds);
            }
            return bounds;
        }

        private static SkinnedMeshRenderer FindRenderer(GameObject root, string meshName)
        {
            foreach (var renderer in root.GetComponentsInChildren<SkinnedMeshRenderer>(true))
            {
                if (renderer.gameObject.name == meshName)
                {
                    return renderer;
                }
            }
            throw new InvalidOperationException($"expression mesh {meshName} not found");
        }

        /// Weights go straight onto the renderer so the bind pose stays; the .anim path is covered by
        /// BatchCheck and the Animation window.
        private static void ApplyExpression(SkinnedMeshRenderer renderer, SidecarExpression expression)
        {
            foreach (var weight in expression.weights)
            {
                var index = renderer.sharedMesh.GetBlendShapeIndex(weight.Key);
                if (index < 0)
                {
                    Debug.LogWarning($"FACEPREVIEW {expression.name}: blend shape {weight.Key} not on {renderer.name}");
                    continue;
                }
                renderer.SetBlendShapeWeight(index, weight.Value * 100f);
            }
        }

        private static void ClearExpression(SkinnedMeshRenderer renderer, SidecarExpression expression)
        {
            foreach (var weight in expression.weights)
            {
                var index = renderer.sharedMesh.GetBlendShapeIndex(weight.Key);
                if (index >= 0)
                {
                    renderer.SetBlendShapeWeight(index, 0f);
                }
            }
        }

        /// Renders a CPU-baked copy of the skinned mesh: the container's software GL does not run
        /// Unity's GPU blend shapes, so the skinned renderer itself would show the neutral face.
        private sealed class BakedStandIn : IDisposable
        {
            private readonly SkinnedMeshRenderer source;
            private readonly GameObject standIn;

            public BakedStandIn(SkinnedMeshRenderer source)
            {
                this.source = source;
                var baked = new Mesh();
                source.BakeMesh(baked);
                standIn = new GameObject($"{source.name}_baked");
                standIn.transform.SetPositionAndRotation(source.transform.position, source.transform.rotation);
                standIn.AddComponent<MeshFilter>().sharedMesh = baked;
                standIn.AddComponent<MeshRenderer>().sharedMaterials = source.sharedMaterials;
                source.enabled = false;
            }

            public void Dispose()
            {
                source.enabled = true;
                UnityEngine.Object.DestroyImmediate(standIn.GetComponent<MeshFilter>().sharedMesh);
                UnityEngine.Object.DestroyImmediate(standIn);
            }
        }

        /// Bone transforms follow the sampled pose while renderer bounds do not, so every capture
        /// re-aims at the head bone.
        private static void CaptureBoth(
            Camera faceCamera, Camera wideCamera, Animator animator, float height, Vector3 facing,
            string outputDir, string name)
        {
            var head = animator.isHuman ? animator.GetBoneTransform(HumanBodyBones.Head) : null;
            var headPosition = head != null ? head.position : animator.transform.position + Vector3.up * height * 0.9f;

            AimCamera(faceCamera, headPosition + Vector3.up * (height * FaceOffsetUpPerHeight), facing);
            Capture(faceCamera, Path.Combine(outputDir, $"{name}.png"));
            AimCamera(wideCamera, headPosition - Vector3.up * (height * WideOffsetDownPerHeight), facing);
            Capture(wideCamera, Path.Combine(outputDir, $"{name}_wide.png"));
        }

        private static Camera CreateCamera(float distance)
        {
            var camera = new GameObject("FacePreviewCamera").AddComponent<Camera>();
            camera.fieldOfView = 20f;
            camera.nearClipPlane = distance * 0.02f;
            camera.farClipPlane = distance * 10f;
            camera.clearFlags = CameraClearFlags.SolidColor;
            camera.backgroundColor = new Color(0.25f, 0.25f, 0.25f);
            camera.targetTexture = new RenderTexture(ImageSize, ImageSize, 24);
            return camera;
        }

        private static void AimCamera(Camera camera, Vector3 target, Vector3 facing)
        {
            var distance = camera.farClipPlane / 10f;
            camera.transform.position = target + facing * distance;
            camera.transform.LookAt(target);
        }

        private static void AttachLight(Camera camera)
        {
            var light = camera.gameObject.AddComponent<Light>();
            light.type = LightType.Directional;
            light.intensity = 0.55f;
        }

        private static void Capture(Camera camera, string path)
        {
            camera.Render();
            var previous = RenderTexture.active;
            RenderTexture.active = camera.targetTexture;
            var image = new Texture2D(ImageSize, ImageSize, TextureFormat.RGB24, false);
            image.ReadPixels(new Rect(0, 0, ImageSize, ImageSize), 0, 0);
            image.Apply();
            RenderTexture.active = previous;
            File.WriteAllBytes(path, image.EncodeToPNG());
            UnityEngine.Object.DestroyImmediate(image);
            Debug.Log($"FACEPREVIEW wrote {path}");
        }
    }
}

#endif
