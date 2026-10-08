/// The encrypted FileId (raw 16 bytes) derived from `CREATION_TIME` by the FBX SDK, not a string.
/// Unity rejects files where this pair does not match.
pub(crate) const FILE_ID_BYTES: [u8; 16] = [
    0x28, 0xb3, 0x2a, 0xeb, 0xb6, 0x24, 0xcc, 0xc2, 0xbf, 0xc8, 0xb0, 0x2a, 0xa9, 0x2b, 0xfc, 0xf1,
];

pub(crate) const CREATION_TIME: &str = "1970-01-01 10:00:00:000";

pub(crate) const CREATOR: &str = "Thyllore Animation Engine";

/// The fixed 16-byte footer ID written by the FBX SDK at the end of the file, not a string.
/// Matches Blender's `io_scene_fbx` `_FOOT_ID`.
pub(crate) const FOOTER_ID: [u8; 16] = [
    0xfa, 0xbc, 0xab, 0x09, 0xd0, 0xc8, 0xd4, 0x66, 0xb1, 0x76, 0xfb, 0x83, 0x1c, 0xf7, 0x26, 0x7e,
];
