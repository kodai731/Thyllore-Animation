#if UNITY_EDITOR

using UnityEditor;
using UnityEngine;

namespace Thyllore.Avatar
{
    public static class HumanoidClipImport
    {
        public static void KeepAllTransformCurves(ModelImporter importer, GameObject modelAsset)
        {
            var mask = new AvatarMask();
            mask.AddTransformPath(modelAsset.transform, true);

            var clips = importer.defaultClipAnimations;
            for (int i = 0; i < clips.Length; i++)
            {
                var clip = clips[i];
                clip.ConfigureClipFromMask(mask);
                clip.maskType = ClipAnimationMaskType.CreateFromThisModel;
                clips[i] = clip;
            }
            importer.clipAnimations = clips;
        }
    }
}

#endif
