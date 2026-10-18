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

                collider.Offset = ToUnityVector(sc.offset);
                collider.Radius = sc.radius;

                if (sc.shape == "capsule")
                {
                    collider.ColliderType = VRM10SpringBoneColliderTypes.Capsule;
                    collider.Tail = ToUnityVector(sc.tail);
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

                    joint.m_stiffnessForce = sj.stiffness;
                    joint.m_dragForce = sj.drag_force;
                    joint.m_gravityPower = sj.gravity_power;
                    joint.m_gravityDir = ToUnityVector(sj.gravity_dir);
                    joint.m_jointRadius = sj.hit_radius;

                    joints.Add(joint);
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

                physBone.pull = Mathf.Clamp01(firstJoint.stiffness);
                physBone.spring = Mathf.Clamp01(1f - firstJoint.drag_force);
                physBone.gravity = Mathf.Clamp01(firstJoint.gravity_power);
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
