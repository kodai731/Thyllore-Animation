#if UNITY_EDITOR

using System.IO;
using Newtonsoft.Json;

namespace Thyllore.Avatar
{
    public static class AvatarSidecarLoader
    {
        public const int SupportedSchema = SidecarSchema.SupportedVersion;

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
