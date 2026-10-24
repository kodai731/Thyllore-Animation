#if UNITY_EDITOR

using System;
using System.Collections.Generic;
using System.IO;
using Newtonsoft.Json;

namespace Thyllore.Avatar
{
    [Serializable]
    public class AvatarSidecar
    {
        [JsonProperty("schema")]
        public int schema;

        [JsonProperty("humanoid")]
        public Dictionary<string, string> humanoid = new Dictionary<string, string>();

        [JsonProperty("visemes")]
        public Dictionary<string, string> visemes = new Dictionary<string, string>();

        [JsonProperty("blink")]
        public SidecarBlink blink = new SidecarBlink();

        [JsonProperty("expression_mesh")]
        public string expression_mesh;

        [JsonProperty("expressions")]
        public List<SidecarExpression> expressions = new List<SidecarExpression>();

        [JsonProperty("spring")]
        public SidecarSpring spring = new SidecarSpring();

        [JsonProperty("materials")]
        public Dictionary<string, string> materials = new Dictionary<string, string>();

        [JsonProperty("stats")]
        public AvatarStats stats = new AvatarStats();
    }

    [Serializable]
    public class SidecarBlink
    {
        [JsonProperty("both")]
        public string both;

        [JsonProperty("left")]
        public string left;

        [JsonProperty("right")]
        public string right;
    }

    [Serializable]
    public class SidecarExpression
    {
        [JsonProperty("name")]
        public string name;

        [JsonProperty("weights")]
        public Dictionary<string, float> weights = new Dictionary<string, float>();
    }

    [Serializable]
    public class SidecarSpring
    {
        [JsonProperty("collision_mode")]
        public string collision_mode;

        [JsonProperty("colliders")]
        public List<SidecarCollider> colliders = new List<SidecarCollider>();

        [JsonProperty("collider_groups")]
        public List<SidecarColliderGroup> collider_groups = new List<SidecarColliderGroup>();

        [JsonProperty("chains")]
        public List<SidecarSpringChain> chains = new List<SidecarSpringChain>();
    }

    [Serializable]
    public class SidecarCollider
    {
        [JsonProperty("name")]
        public string name;

        [JsonProperty("bone")]
        public string bone;

        [JsonProperty("offset")]
        public float[] offset;

        [JsonProperty("shape")]
        public string shape;

        [JsonProperty("radius")]
        public float radius;

        [JsonProperty("tail")]
        public float[] tail;

        [JsonProperty("origin")]
        public string origin;
    }

    [Serializable]
    public class SidecarColliderGroup
    {
        [JsonProperty("name")]
        public string name;

        [JsonProperty("colliders")]
        public List<string> colliders = new List<string>();
    }

    [Serializable]
    public class SidecarSpringChain
    {
        [JsonProperty("name")]
        public string name;

        [JsonProperty("center")]
        public string center;

        [JsonProperty("collider_groups")]
        public List<string> collider_groups = new List<string>();

        [JsonProperty("joints")]
        public List<SidecarSpringJoint> joints = new List<SidecarSpringJoint>();
    }

    [Serializable]
    public class SidecarSpringJoint
    {
        [JsonProperty("bone")]
        public string bone;

        [JsonProperty("stiffness")]
        public float stiffness;

        [JsonProperty("drag_force")]
        public float drag_force;

        [JsonProperty("gravity_power")]
        public float gravity_power;

        [JsonProperty("gravity_dir")]
        public float[] gravity_dir;

        [JsonProperty("hit_radius")]
        public float hit_radius;
    }

    [Serializable]
    public class AvatarStats
    {
        [JsonProperty("triangles")]
        public int triangles;

        [JsonProperty("bones")]
        public int bones;

        [JsonProperty("materials")]
        public int materials;

        [JsonProperty("skinned_meshes")]
        public int skinned_meshes;

        [JsonProperty("meshes")]
        public int meshes;

        [JsonProperty("morph_meshes")]
        public int morph_meshes;

        [JsonProperty("spring_chains")]
        public int spring_chains;

        [JsonProperty("spring_transforms")]
        public int spring_transforms;

        [JsonProperty("spring_colliders")]
        public int spring_colliders;

        [JsonProperty("texture_bytes")]
        public long texture_bytes;
    }

    public static class AvatarSidecarLoader
    {
        public const int SupportedSchema = 3;

        public static AvatarSidecar Load(string path)
        {
            var json = File.ReadAllText(path);
            var sidecar = JsonConvert.DeserializeObject<AvatarSidecar>(json);

            if (sidecar == null)
            {
                throw new InvalidDataException($"Avatar sidecar is empty: {path}");
            }

            if (sidecar.schema != SupportedSchema)
            {
                throw new InvalidDataException(
                    $"Unsupported avatar sidecar schema version: {sidecar.schema} (supported: {SupportedSchema})");
            }

            return sidecar;
        }
    }
}

#endif
