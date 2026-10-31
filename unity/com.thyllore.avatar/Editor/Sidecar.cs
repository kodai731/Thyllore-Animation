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

        [JsonProperty("spring_chains")]
        public List<SidecarSpringChain> spring_chains = new List<SidecarSpringChain>();

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
    public class SidecarSpringChain
    {
        [JsonProperty("root")]
        public string root;

        [JsonProperty("stiffness")]
        public float stiffness;

        [JsonProperty("gravity")]
        public float gravity;

        [JsonProperty("drag")]
        public float drag;

        [JsonProperty("colliders")]
        public List<string> colliders = new List<string>();
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
        public const int SupportedSchema = 2;

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
