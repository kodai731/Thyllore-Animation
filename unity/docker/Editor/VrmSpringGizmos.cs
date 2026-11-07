#if UNITY_EDITOR

using UnityEditor;
using UnityEngine;
using UniVRM10;

namespace Thyllore.Avatar
{
    public static class VrmSpringGizmos
    {
        private static readonly Color JointColor = new Color(1f, 0.85f, 0.1f, 0.9f);
        private static readonly Color ColliderColor = new Color(0.2f, 0.9f, 1f, 0.9f);
        private static readonly Color InsideColliderColor = new Color(1f, 0.4f, 0.4f, 0.9f);

        [DrawGizmo(GizmoType.NonSelected | GizmoType.Selected | GizmoType.Pickable)]
        private static void DrawSpringBones(Vrm10Instance instance, GizmoType gizmoType)
        {
            foreach (var group in instance.SpringBone.ColliderGroups)
            {
                foreach (var collider in group.Colliders)
                {
                    if (collider != null)
                    {
                        DrawCollider(collider);
                    }
                }
            }

            foreach (var spring in instance.SpringBone.Springs)
            {
                for (var i = 0; i < spring.Joints.Count; i++)
                {
                    var joint = spring.Joints[i];
                    if (joint == null)
                    {
                        continue;
                    }
                    Gizmos.color = JointColor;
                    Gizmos.DrawWireSphere(joint.transform.position, joint.m_jointRadius * joint.transform.lossyScale.x);
                    if (i + 1 < spring.Joints.Count && spring.Joints[i + 1] != null)
                    {
                        Gizmos.DrawLine(joint.transform.position, spring.Joints[i + 1].transform.position);
                    }
                }
            }
        }

        private static void DrawCollider(VRM10SpringBoneCollider collider)
        {
            var transform = collider.transform;
            var scale = transform.lossyScale.x;
            var head = transform.TransformPoint(collider.Offset);
            var radius = collider.Radius * scale;
            var inside = collider.ColliderType == VRM10SpringBoneColliderTypes.SphereInside
                || collider.ColliderType == VRM10SpringBoneColliderTypes.CapsuleInside;
            Gizmos.color = inside ? InsideColliderColor : ColliderColor;

            switch (collider.ColliderType)
            {
                case VRM10SpringBoneColliderTypes.Sphere:
                case VRM10SpringBoneColliderTypes.SphereInside:
                    Gizmos.DrawWireSphere(head, radius);
                    break;
                case VRM10SpringBoneColliderTypes.Capsule:
                case VRM10SpringBoneColliderTypes.CapsuleInside:
                    DrawCapsule(head, transform.TransformPoint(collider.Tail), radius);
                    break;
                case VRM10SpringBoneColliderTypes.Plane:
                    Gizmos.DrawLine(head, head + transform.TransformDirection(collider.Normal) * 0.1f);
                    break;
            }
        }

        private static void DrawCapsule(Vector3 head, Vector3 tail, float radius)
        {
            Gizmos.DrawWireSphere(head, radius);
            Gizmos.DrawWireSphere(tail, radius);

            var axis = tail - head;
            if (axis.sqrMagnitude < 1e-12f)
            {
                return;
            }
            var side = Vector3.Cross(axis.normalized, Vector3.up);
            if (side.sqrMagnitude < 1e-6f)
            {
                side = Vector3.Cross(axis.normalized, Vector3.right);
            }
            side = side.normalized * radius;
            var forward = Vector3.Cross(axis.normalized, side).normalized * radius;
            Gizmos.DrawLine(head + side, tail + side);
            Gizmos.DrawLine(head - side, tail - side);
            Gizmos.DrawLine(head + forward, tail + forward);
            Gizmos.DrawLine(head - forward, tail - forward);
        }
    }
}

#endif
