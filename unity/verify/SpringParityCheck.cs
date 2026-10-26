#if UNITY_EDITOR
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEngine;
using UniGLTF;
using UniVRM10;

namespace Thyllore.Avatar
{
    public static class SpringParityCheck
    {
        static string RequireEnv(string name)
        {
            var value = Environment.GetEnvironmentVariable(name);
            if (string.IsNullOrEmpty(value))
                throw new InvalidOperationException("SPRINGPARITY " + name + " not set");
            return value;
        }

        public static void Run()
        {
            try
            {
                var fbxAsset = RequireEnv("THYLLORE_FBX_ASSET");
                var sidecarPath = RequireEnv("THYLLORE_SIDECAR");
                var outPath = RequireEnv("THYLLORE_TAILS_OUT");

                var rate = float.Parse(RequireEnv("THYLLORE_SAMPLE_RATE"), CultureInfo.InvariantCulture);
                var frameCount = int.Parse(RequireEnv("THYLLORE_FRAME_COUNT"));

                var root = LoadAndSetup(fbxAsset, sidecarPath);
                var clip = GetClip(fbxAsset);

                var instance = root.GetComponent<Vrm10Instance>();
                var runtime = new Vrm10FastSpringboneRuntimeStandalone();
                runtime.InitializeAsync(instance, new ImmediateCaller()).GetAwaiter().GetResult();
                runtime.ReconstructSpringBone();

                var rows = SampleFrames(root, clip, frameCount, rate, runtime);

                WriteCsv(outPath, rows);
                Debug.Log("SPRINGPARITY wrote " + rows.Count + " rows to " + outPath);

                runtime.Dispose();
                UnityEngine.Object.DestroyImmediate(root);
            }
            catch (Exception ex)
            {
                Debug.LogError("SPRINGPARITY " + ex);
                EditorApplication.Exit(1);
                return;
            }

            EditorApplication.Exit(0);
        }

        static GameObject LoadAndSetup(string fbxAsset, string sidecarPath)
        {
            AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);

            var importer = (ModelImporter)AssetImporter.GetAtPath(fbxAsset);
            if (importer == null)
                throw new InvalidOperationException("SPRINGPARITY no ModelImporter at " + fbxAsset);

            importer.animationType = ModelImporterAnimationType.Generic;
            importer.importAnimation = true;
            importer.SaveAndReimport();

            var prefab = AssetDatabase.LoadAssetAtPath<GameObject>(fbxAsset);
            var root = (GameObject)UnityEngine.Object.Instantiate(prefab);

            var sidecar = AvatarSidecarLoader.Load(sidecarPath);
            SpringSidecarApplier.Apply(root, sidecar.spring);
            LogJointFrames(root);

            return root;
        }

        static void LogJointFrames(GameObject root)
        {
            foreach (var joint in root.GetComponentsInChildren<VRM10SpringBoneJoint>(true))
            {
                var t = joint.transform;
                var child = t.childCount > 0 ? t.GetChild(0) : null;
                Debug.Log(string.Format(CultureInfo.InvariantCulture,
                    "SPRINGPARITY joint {0} localPos={1} localRot={2} lossyScale={3} child={4} childLocalPos={5} childPos={6}",
                    t.name, t.localPosition.ToString("F4"), t.localRotation.eulerAngles.ToString("F2"),
                    t.lossyScale.ToString("F3"),
                    child != null ? child.name : "none",
                    child != null ? child.localPosition.ToString("F4") : "-",
                    child != null ? child.position.ToString("F4") : "-"));
            }
        }

        static AnimationClip GetClip(string fbxAsset)
        {
            var assets = AssetDatabase.LoadAllAssetsAtPath(fbxAsset);
            var clip = assets.OfType<AnimationClip>()
                .FirstOrDefault(c => !c.name.StartsWith("__preview__"));
            if (clip == null)
                throw new InvalidOperationException("SPRINGPARITY no animation clip found");
            return clip;
        }

        static List<string> SampleFrames(GameObject root, AnimationClip clip, int frameCount, float rate, Vrm10FastSpringboneRuntimeStandalone runtime)
        {
            var rows = new List<string>();

            AnimationMode.StartAnimationMode();

            var instance = root.GetComponent<Vrm10Instance>();

            for (int frame = 0; frame < frameCount; frame++)
            {
                var t = frame / rate;

                AnimationMode.BeginSampling();
                AnimationMode.SampleAnimationClip(root, clip, t);
                AnimationMode.EndSampling();

                runtime.Process(1f / rate);

                foreach (var spring in instance.SpringBone.Springs)
                {
                    for (int j = 0; j < spring.Joints.Count - 1; j++)
                    {
                        var tail = spring.Joints[j + 1].transform.position;
                        var line = string.Format(CultureInfo.InvariantCulture, "{0},{1},{2},{3},{4},{5}",
                            frame, spring.Name, spring.Joints[j].name, -tail.x, tail.y, tail.z);
                        rows.Add(line);
                    }
                }
            }

            AnimationMode.StopAnimationMode();

            return rows;
        }

        static void WriteCsv(string path, List<string> rows)
        {
            var lines = new List<string> { "frame,chain,bone,x,y,z" };
            lines.AddRange(rows);
            File.WriteAllLines(path, lines);
        }
    }
}
#endif
