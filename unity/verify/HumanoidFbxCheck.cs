#if UNITY_EDITOR
using System;
using System.IO;
using System.Linq;
using System.Text;
using UnityEditor;
using UnityEngine;

namespace Thyllore.Avatar
{
    public static class HumanoidFbxCheck
    {
        public static void Run()
        {
            try
            {
                var fbxAsset = Environment.GetEnvironmentVariable("THYLLORE_FBX_ASSET");
                if (string.IsNullOrEmpty(fbxAsset))
                {
                    Debug.LogError("HUMANOIDCHECK THYLLORE_FBX_ASSET not set");
                    EditorApplication.Exit(1);
                    return;
                }

                var timesStr = Environment.GetEnvironmentVariable("THYLLORE_POSE_TIMES") ?? "0,1";
                var times = timesStr.Split(',').Select(s => float.Parse(s.Trim())).ToArray();

                var outPath = Environment.GetEnvironmentVariable("THYLLORE_POSE_OUT") ?? "/work/humanoid_pose.json";

                AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);

                var importer = (ModelImporter)AssetImporter.GetAtPath(fbxAsset);
                if (importer == null)
                {
                    Debug.LogError("HUMANOIDCHECK no ModelImporter at " + fbxAsset);
                    EditorApplication.Exit(1);
                    return;
                }

                importer.animationType = ModelImporterAnimationType.Human;
                importer.avatarSetup = ModelImporterAvatarSetup.CreateFromThisModel;
                importer.SaveAndReimport();

                var modelAsset = AssetDatabase.LoadAssetAtPath<GameObject>(fbxAsset);
                HumanoidClipImport.KeepAllTransformCurves(importer, modelAsset);
                importer.SaveAndReimport();

                Debug.Log("HUMANOIDCHECK imported scale=" + importer.fileScale + " useFileScale=" + importer.useFileScale);
                Debug.Log("HUMANOIDCHECK takes=" + importer.importedTakeInfos.Length + " defaultClips=" + importer.defaultClipAnimations.Length);

                var assets = AssetDatabase.LoadAllAssetsAtPath(fbxAsset);
                var avatar = assets.OfType<UnityEngine.Avatar>().FirstOrDefault();
                Debug.Log("HUMANOIDCHECK avatar=" + (avatar != null) + " valid=" + (avatar != null && avatar.isValid) + " human=" + (avatar != null && avatar.isHuman));

                if (avatar == null || !avatar.isHuman)
                {
                    Debug.LogError("HUMANOIDCHECK avatar is not humanoid");
                    EditorApplication.Exit(1);
                    return;
                }

                var clips = assets.OfType<AnimationClip>()
                    .Where(c => !c.name.StartsWith("__preview__")).ToArray();
                foreach (var c in clips)
                {
                    Debug.Log("HUMANOIDCHECK clip=" + c.name + " human=" + c.isHumanMotion + " length=" + c.length);
                }

                if (clips.Length == 0)
                {
                    Debug.LogError("HUMANOIDCHECK no humanoid clips found");
                    EditorApplication.Exit(1);
                    return;
                }

                var prefab = AssetDatabase.LoadAssetAtPath<GameObject>(fbxAsset);
                var root = (GameObject)UnityEngine.Object.Instantiate(prefab);
                var animator = root.GetComponent<Animator>() ?? root.AddComponent<Animator>();
                animator.avatar = avatar;
                animator.applyRootMotion = false;

                animator.Rebind();
                AnimationMode.StartAnimationMode();

                var json = new StringBuilder("{\"poses\":[");
                for (var i = 0; i < times.Length; i++)
                {
                    AnimationMode.BeginSampling();
                    AnimationMode.SampleAnimationClip(root, clips[0], times[i]);
                    AnimationMode.EndSampling();

                    json.Append(i == 0 ? "" : ",").Append("{\"time\":").Append(times[i]).Append(",\"bones\":{");
                    var first = true;
                    foreach (var t in root.GetComponentsInChildren<Transform>())
                    {
                        var p = t.position;
                        json.Append(first ? "" : ",").Append("\"").Append(t.name).Append("\":[")
                            .Append(p.x.ToString("R")).Append(",").Append(p.y.ToString("R")).Append(",").Append(p.z.ToString("R")).Append("]");
                        first = false;
                    }
                    json.Append("}}");
                }
                json.Append("]}");

                AnimationMode.StopAnimationMode();
                UnityEngine.Object.DestroyImmediate(root);

                File.WriteAllText(outPath, json.ToString());
                Debug.Log("HUMANOIDCHECK wrote " + outPath);
            }
            catch (Exception ex)
            {
                Debug.LogError("HUMANOIDCHECK EXCEPTION " + ex);
                EditorApplication.Exit(1);
                return;
            }

            EditorApplication.Exit(0);
        }
    }
}
#endif
