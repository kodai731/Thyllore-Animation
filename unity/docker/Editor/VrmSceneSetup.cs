#if UNITY_EDITOR

using System;
using System.Globalization;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UniGLTF;
using UniVRM10;

namespace Thyllore.Avatar
{
    public static class VrmSceneSetup
    {
        private const int SimulatedFrames = 60;
        private const float SimulatedRate = 60f;

        public static void Run()
        {
            try
            {
                Build();
            }
            catch (Exception ex)
            {
                Debug.LogError($"VRMCHECK EXCEPTION {ex}");
                EditorApplication.Exit(1);
                return;
            }

            Debug.Log("VRMCHECK DONE");
            EditorApplication.Exit(0);
        }

        private static void Build()
        {
            var vrmAsset = Environment.GetEnvironmentVariable("THYLLORE_VRM_ASSET");
            AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);

            var scene = EditorSceneManager.NewScene(NewSceneSetup.DefaultGameObjects, NewSceneMode.Single);
            var root = Instantiate(vrmAsset);
            var instance = root.GetComponent<Vrm10Instance>();
            if (instance == null)
            {
                throw new InvalidOperationException($"no Vrm10Instance on the imported prefab: {vrmAsset}");
            }

            ReportMeta(instance);
            ReportHumanoid(root);
            ReportColliders(instance);
            ReportSprings(instance);
            ReportSimulation(instance);

            Selection.activeGameObject = root;
            EditorSceneManager.SaveScene(scene, SceneSetup.ScenePath);
            AssetDatabase.SaveAssets();
        }

        private static GameObject Instantiate(string vrmAsset)
        {
            if (string.IsNullOrEmpty(vrmAsset) || !File.Exists(vrmAsset))
            {
                throw new FileNotFoundException($"vrm asset not found: {vrmAsset}");
            }
            var prefab = AssetDatabase.LoadAssetAtPath<GameObject>(vrmAsset);
            if (prefab == null)
            {
                throw new FileNotFoundException($"vrm asset not imported by UniVRM: {vrmAsset}");
            }
            return (GameObject)PrefabUtility.InstantiatePrefab(prefab);
        }

        private static void ReportMeta(Vrm10Instance instance)
        {
            var meta = instance.Vrm != null ? instance.Vrm.Meta : null;
            var name = meta != null ? meta.Name : "(no meta)";
            Debug.Log($"VRMCHECK meta name='{name}'");
        }

        private static void ReportHumanoid(GameObject root)
        {
            var animator = root.GetComponent<Animator>();
            var isHuman = animator != null && animator.avatar != null && animator.avatar.isHuman;
            Check(isHuman, "humanoid avatar is human");
            if (!isHuman)
            {
                return;
            }

            var missing = HumanBones()
                .Where(bone => animator.GetBoneTransform(bone) == null)
                .Select(bone => bone.ToString())
                .ToArray();
            var mapped = HumanBones().Count() - missing.Length;
            Debug.Log($"VRMCHECK humanoid mapped={mapped} missing=[{string.Join(", ", missing)}]");
            Check(RequiredBones().All(bone => animator.GetBoneTransform(bone) != null), "required humanoid bones mapped");

            var hips = animator.GetBoneTransform(HumanBodyBones.Hips);
            var head = animator.GetBoneTransform(HumanBodyBones.Head);
            var leftFoot = animator.GetBoneTransform(HumanBodyBones.LeftFoot);
            Debug.Log(Invariant($"VRMCHECK humanoid hips={hips.position:F3} head={head.position:F3} leftFoot={leftFoot.position:F3} hipsScale={hips.lossyScale:F3}"));
            Check(head.position.y > hips.position.y && hips.position.y > leftFoot.position.y, "humanoid stands upright (head above hips above feet)");
            Check(Mathf.Abs(hips.lossyScale.x - 1f) < 1e-3f, "hips world scale is 1");
        }

        private static void ReportColliders(Vrm10Instance instance)
        {
            var groups = instance.SpringBone.ColliderGroups;
            var total = groups.Sum(group => group.Colliders.Count);
            Debug.Log($"VRMCHECK colliderGroups={groups.Count} colliders={total}");
            foreach (var group in groups)
            {
                foreach (var collider in group.Colliders)
                {
                    Debug.Log(Invariant($"VRMCHECK collider group='{group.Name}' bone={collider.transform.name} type={collider.ColliderType} radius={collider.Radius:F3} offset={collider.Offset:F3} tail={collider.Tail:F3}"));
                }
            }
            Check(groups.All(group => group.Colliders.All(collider => collider != null)), "every collider group entry resolves to a collider component");
            Check(groups.SelectMany(group => group.Colliders).All(collider => collider.Radius > 0f), "every collider has a positive radius");
        }

        private static void ReportSprings(Vrm10Instance instance)
        {
            var springs = instance.SpringBone.Springs;
            Debug.Log($"VRMCHECK springs={springs.Count} joints={springs.Sum(spring => spring.Joints.Count)}");
            foreach (var spring in springs)
            {
                var radii = string.Join("/", spring.Joints.Select(joint => joint.m_jointRadius.ToString("F3", CultureInfo.InvariantCulture)));
                var groups = string.Join(",", spring.ColliderGroups.Select(group => group.Name));
                Debug.Log($"VRMCHECK spring '{spring.Name}' joints={spring.Joints.Count} radii={radii} groups=[{groups}] center={(spring.Center != null ? spring.Center.name : "none")}");
            }
            Check(springs.All(spring => spring.Joints.All(joint => joint != null)), "every spring joint resolves to a joint component");
            Check(springs.All(spring => spring.Joints.Count >= 2), "every spring has at least two joints");
            Check(springs.All(spring => spring.ColliderGroups.All(group => group != null)), "every spring collider group reference resolves");
            Check(springs.All(spring => IsChainOfParents(spring)), "every spring's joints form a parent-child chain");
        }

        private static bool IsChainOfParents(Vrm10InstanceSpringBone.Spring spring)
        {
            for (var i = 1; i < spring.Joints.Count; i++)
            {
                if (spring.Joints[i].transform.parent != spring.Joints[i - 1].transform)
                {
                    return false;
                }
            }
            return true;
        }

        private static void ReportSimulation(Vrm10Instance instance)
        {
            var joints = instance.SpringBone.Springs.SelectMany(spring => spring.Joints).ToArray();
            var restPositions = joints.Select(joint => joint.transform.position).ToArray();

            var runtime = new Vrm10FastSpringboneRuntimeStandalone();
            runtime.InitializeAsync(instance, new ImmediateCaller()).GetAwaiter().GetResult();
            runtime.ReconstructSpringBone();
            for (var frame = 0; frame < SimulatedFrames; frame++)
            {
                runtime.Process(1f / SimulatedRate);
            }
            runtime.Dispose();

            var drifts = joints.Select((joint, i) => (joint.transform.position - restPositions[i]).magnitude).ToArray();
            var finite = joints.All(joint => IsFinite(joint.transform.position));
            var maxDrift = drifts.Length > 0 ? drifts.Max() : 0f;
            Debug.Log(Invariant($"VRMCHECK simulation frames={SimulatedFrames} maxJointDrift={maxDrift:F4}m"));
            foreach (var i in Enumerable.Range(0, joints.Length).OrderByDescending(i => drifts[i]).Take(5))
            {
                Debug.Log(Invariant($"VRMCHECK drift {joints[i].name} {drifts[i]:F4}m rest={restPositions[i]:F3} settled={joints[i].transform.position:F3}"));
            }
            Check(finite, "spring simulation stays finite");
            Check(maxDrift < 0.1f, "spring joints stay within 10 cm of rest after settling");

            for (var i = 0; i < joints.Length; i++)
            {
                joints[i].transform.position = restPositions[i];
            }
        }

        private static bool IsFinite(Vector3 v)
        {
            return !(float.IsNaN(v.x) || float.IsNaN(v.y) || float.IsNaN(v.z) || float.IsInfinity(v.x) || float.IsInfinity(v.y) || float.IsInfinity(v.z));
        }

        private static System.Collections.Generic.IEnumerable<HumanBodyBones> HumanBones()
        {
            return Enum.GetValues(typeof(HumanBodyBones)).Cast<HumanBodyBones>().Where(bone => bone != HumanBodyBones.LastBone);
        }

        private static HumanBodyBones[] RequiredBones()
        {
            return new[]
            {
                HumanBodyBones.Hips, HumanBodyBones.Spine, HumanBodyBones.Head,
                HumanBodyBones.LeftUpperArm, HumanBodyBones.LeftLowerArm, HumanBodyBones.LeftHand,
                HumanBodyBones.RightUpperArm, HumanBodyBones.RightLowerArm, HumanBodyBones.RightHand,
                HumanBodyBones.LeftUpperLeg, HumanBodyBones.LeftLowerLeg, HumanBodyBones.LeftFoot,
                HumanBodyBones.RightUpperLeg, HumanBodyBones.RightLowerLeg, HumanBodyBones.RightFoot,
            };
        }

        private static void Check(bool condition, string label)
        {
            Debug.Log($"VRMCHECK {(condition ? "PASS" : "FAIL")} {label}");
        }

        private static string Invariant(FormattableString text)
        {
            return text.ToString(CultureInfo.InvariantCulture);
        }
    }
}

#endif
