#if UNITY_EDITOR

using System.Collections.Generic;
using UnityEngine;
#if THYLLORE_UNIVRM
using UniVRM10;
#else
using VRCPhysBone = VRC.SDK3.Dynamics.PhysBone.Components.VRCPhysBone;
#endif
using UnityEditor;

namespace Thyllore.Avatar
{
    public static class SpringSidecarApplier
    {
        public static void Apply(GameObject root, SidecarSpring spring)
        {
#if THYLLORE_UNIVRM
            ApplyVrm10(root, spring);
#else
            ApplyPhysBone(root, spring);
#endif
        }

        static Vector3 ToUnityVector(float[] a)
        {
            return new Vector3(-a[0], a[1], a[2]);
        }

        /// The sidecar keeps collider vectors in model space; UniVRM wants them in the bone's local space.
        static Vector3 ToBoneLocal(GameObject root, Transform bone, float[] modelVector)
        {
            return bone.InverseTransformVector(root.transform.TransformVector(ToUnityVector(modelVector)));
        }

        /// Radii follow their offsets into bone-local units; a length scales by the mean axis length, as in the engine's rest frame.
        static float ToBoneLocalLength(GameObject root, Transform bone, float modelLength)
        {
            return modelLength * MeanAxisScale(root.transform) / MeanAxisScale(bone);
        }

        static float MeanAxisScale(Transform transform)
        {
            var scale = transform.lossyScale;
            return (Mathf.Abs(scale.x) + Mathf.Abs(scale.y) + Mathf.Abs(scale.z)) / 3f;
        }

        /// The VRM speeds a joint force stands for; a rest-relative force maps `D / L` to the stiffness and has no gravity at rest.
        public static void VrmMagnitudes(SidecarJointForce force, float strandLength, out float stiffness, out float gravityPower)
        {
            switch (force.model)
            {
                case "vrm":
                    stiffness = force.stiffness ?? 0f;
                    gravityPower = force.gravity_power ?? 0f;
                    return;
                case "rest_relative":
                    var relaxation = force.bending_relaxation ?? 0f;
                    stiffness = strandLength > 0f ? relaxation / strandLength : 0f;
                    gravityPower = 0f;
                    return;
                default:
                    throw new System.ArgumentException($"unknown spring force model {force.model}");
            }
        }

        static float StrandLength(List<Transform> jointTransforms)
        {
            var length = 0f;
            for (int i = 0; i + 1 < jointTransforms.Count; i++)
            {
                length += Vector3.Distance(jointTransforms[i].position, jointTransforms[i + 1].position);
            }
            var last = jointTransforms.Count > 0 ? jointTransforms[jointTransforms.Count - 1] : null;
            if (last != null && last.childCount > 0)
            {
                length += Vector3.Distance(last.position, last.GetChild(0).position);
            }
            return length;
        }

        static Transform FindBone(GameObject root, string name)
        {
            foreach (Transform child in root.GetComponentsInChildren<Transform>(true))
            {
                if (child.name == name)
                    return child;
            }

            Debug.LogWarning($"Bone not found: {name}");
            return null;
        }

#if THYLLORE_UNIVRM

        /// The engine extends a leaf joint along its own bone; VRM needs a node there to swing towards.
        static VRM10SpringBoneJoint CreateLeafTailJoint(VRM10SpringBoneJoint last)
        {
            var bone = last.transform;
            var parentPosition = bone.parent != null ? bone.parent.position : bone.position;
            var end = FindOrCreateChild(bone.gameObject, bone.name + "_end");
            end.transform.position = bone.position + (bone.position - parentPosition);

            var joint = end.GetComponent<VRM10SpringBoneJoint>();
            if (joint == null)
            {
                joint = Undo.AddComponent<VRM10SpringBoneJoint>(end);
            }
            joint.m_stiffnessForce = last.m_stiffnessForce;
            joint.m_dragForce = last.m_dragForce;
            joint.m_gravityPower = last.m_gravityPower;
            joint.m_gravityDir = last.m_gravityDir;
            joint.m_jointRadius = last.m_jointRadius;
            return joint;
        }

        static GameObject FindOrCreateChild(GameObject parent, string name)
        {
            foreach (Transform child in parent.transform)
            {
                if (child.name == name)
                    return child.gameObject;
            }

            var go = new GameObject(name);
            Undo.RegisterCreatedObjectUndo(go, "SpringSidecarApplier: create collider group");
            Undo.SetTransformParent(go.transform, parent.transform, "SpringSidecarApplier: set parent");
            go.transform.localPosition = Vector3.zero;
            go.transform.localRotation = Quaternion.identity;
            go.transform.localScale = Vector3.one;
            return go;
        }

        static void ApplyVrm10(GameObject root, SidecarSpring spring)
        {
            var colliderMap = new Dictionary<string, VRM10SpringBoneCollider>();

            ApplyColliders(root, spring.colliders, colliderMap);

            var groups = new List<VRM10SpringBoneColliderGroup>();
            ApplyColliderGroups(root, spring.collider_groups, colliderMap, groups);

            var instance = root.GetComponent<Vrm10Instance>();
            if (instance == null)
            {
                instance = Undo.AddComponent<Vrm10Instance>(root);
            }

            instance.SpringBone.ColliderGroups = groups;

           ApplySprings(root, spring.chains, groups, instance);
        }

        static void ApplyColliders(GameObject root, List<SidecarCollider> colliders, Dictionary<string, VRM10SpringBoneCollider> colliderMap)
        {
            foreach (var sc in colliders)
            {
                var boneTransform = FindBone(root, sc.bone);
                if (boneTransform == null)
                    continue;

                var childGo = FindOrCreateChild(boneTransform.gameObject, sc.name);

                var collider = childGo.GetComponent<VRM10SpringBoneCollider>();
                if (collider == null)
                {
                    collider = Undo.AddComponent<VRM10SpringBoneCollider>(childGo);
                }

                collider.Offset = ToBoneLocal(root, boneTransform, sc.offset);
                collider.Radius = ToBoneLocalLength(root, boneTransform, sc.radius);

                if (sc.shape == "capsule")
                {
                    collider.ColliderType = VRM10SpringBoneColliderTypes.Capsule;
                    collider.Tail = ToBoneLocal(root, boneTransform, sc.tail);
                }
                else
                {
                    collider.ColliderType = VRM10SpringBoneColliderTypes.Sphere;
                }

                colliderMap[sc.name] = collider;
            }
        }

        static void ApplyColliderGroups(GameObject root, List<SidecarColliderGroup> colliderGroups, Dictionary<string, VRM10SpringBoneCollider> colliderMap, List<VRM10SpringBoneColliderGroup> groups)
        {
            foreach (var cg in colliderGroups)
            {
                var groupGo = FindOrCreateChild(root, cg.name);

                var group = groupGo.GetComponent<VRM10SpringBoneColliderGroup>();
                if (group == null)
                {
                    group = Undo.AddComponent<VRM10SpringBoneColliderGroup>(groupGo);
                }

                group.Name = cg.name;

                var colliders = new List<VRM10SpringBoneCollider>();
                foreach (var name in cg.colliders)
                {
                    if (colliderMap.TryGetValue(name, out var c))
                    {
                        colliders.Add(c);
                    }
                }

                group.Colliders = colliders;
                groups.Add(group);
            }
        }

       static void ApplySprings(GameObject root, List<SidecarSpringChain> chains, List<VRM10SpringBoneColliderGroup> groups, Vrm10Instance instance)
        {
            var springs = new List<Vrm10InstanceSpringBone.Spring>();

            for (int i = 0; i < chains.Count; i++)
            {
                var chain = chains[i];

                Transform centerTransform = null;
                if (!string.IsNullOrEmpty(chain.center))
                {
                    centerTransform = FindBone(root, chain.center);
                }

                var colliderGroupList = new List<VRM10SpringBoneColliderGroup>();
                foreach (var name in chain.collider_groups)
                {
                    foreach (var g in groups)
                    {
                        if (g.Name == name)
                        {
                            colliderGroupList.Add(g);
                            break;
                        }
                    }
                }

                var jointTransforms = new List<Transform>();
                foreach (var sj in chain.joints)
                {
                    var jointTransform = FindBone(root, sj.bone);
                    if (jointTransform != null)
                        jointTransforms.Add(jointTransform);
                }
                var strandLength = StrandLength(jointTransforms);

                var joints = new List<VRM10SpringBoneJoint>();

                for (int j = 0; j < chain.joints.Count; j++)
                {
                    var sj = chain.joints[j];
                    var jointTransform = FindBone(root, sj.bone);
                    if (jointTransform == null)
                        continue;

                    var joint = jointTransform.GetComponent<VRM10SpringBoneJoint>();
                    if (joint == null)
                    {
                        joint = Undo.AddComponent<VRM10SpringBoneJoint>(jointTransform.gameObject);
                    }

                    VrmMagnitudes(sj.force, strandLength, out var stiffness, out var gravityPower);
                    joint.m_stiffnessForce = stiffness;
                    joint.m_dragForce = sj.drag_force;
                    joint.m_gravityPower = gravityPower;
                    joint.m_gravityDir = ToUnityVector(sj.gravity_dir);
                    joint.m_jointRadius = ToBoneLocalLength(root, jointTransform, sj.hit_radius);

                    joints.Add(joint);
                }

                if (joints.Count > 0 && joints[joints.Count - 1].transform.childCount == 0)
                {
                    joints.Add(CreateLeafTailJoint(joints[joints.Count - 1]));
                }

                var spring = new Vrm10InstanceSpringBone.Spring(chain.name)
                {
                    ColliderGroups = colliderGroupList,
                    Joints = joints,
                    Center = centerTransform,
                };

                springs.Add(spring);
            }

            instance.SpringBone.Springs = springs;
        }

#else

        static void ApplyPhysBone(GameObject root, SidecarSpring spring)
        {
            foreach (var chain in spring.chains)
            {
                if (chain.joints.Count == 0)
                    continue;

                var firstJoint = chain.joints[0];
                var boneTransform = FindBone(root, firstJoint.bone);
                if (boneTransform == null)
                    continue;

                var physBone = boneTransform.GetComponent<VRCPhysBone>();
                if (physBone == null)
                {
                    physBone = Undo.AddComponent<VRCPhysBone>(boneTransform.gameObject);
                }

                var jointTransforms = new List<Transform>();
                foreach (var sj in chain.joints)
                {
                    var jointTransform = FindBone(root, sj.bone);
                    if (jointTransform != null)
                        jointTransforms.Add(jointTransform);
                }
                VrmMagnitudes(firstJoint.force, StrandLength(jointTransforms), out var stiffness, out var gravityPower);
                physBone.pull = Mathf.Clamp01(stiffness);
                physBone.spring = Mathf.Clamp01(1f - firstJoint.drag_force);
                physBone.gravity = Mathf.Clamp01(gravityPower);
            }

            foreach (var collider in spring.colliders)
            {
                Debug.Log($"PhysBone collider (skipped): bone={collider.bone} shape={collider.shape}");
            }
        }

#endif
    }
}

#endif
