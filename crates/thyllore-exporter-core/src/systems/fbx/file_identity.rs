pub(crate) const CREATOR: &str = "Thyllore Animation Engine";

/// Fixed so that every export of the same scene is byte-identical (same value Blender uses).
pub(crate) const DEFAULT_CREATION_TIME: CreationTime = CreationTime {
    year: 1970,
    month: 1,
    day: 1,
    hour: 10,
    minute: 0,
    second: 0,
    millisecond: 0,
};

/// Seed block the FBX SDK mixes the creation time into for both identities.
const SDK_IDENTITY_SEED: [u8; 16] = [
    0x58, 0xab, 0xa9, 0xf0, 0x6c, 0xa2, 0xd8, 0x3f, 0x4d, 0x47, 0x49, 0xa3, 0xb4, 0xb2, 0xe7, 0x3d,
];

/// Second mixing block the FBX SDK applies only to the footer id.
const SDK_FOOTER_KEY: [u8; 16] = [
    0xe2, 0x4f, 0x7b, 0x5f, 0xcd, 0xe4, 0xc8, 0x6d, 0xdb, 0xd8, 0xfb, 0xd7, 0x40, 0x58, 0xc6, 0x78,
];

const XOR_CHAIN_SEED: u8 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CreationTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub millisecond: u16,
}

impl CreationTime {
    /// The `CreationTime` top-level node value, e.g. `1970-01-01 10:00:00:000`.
    pub(crate) fn to_fbx_string(&self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}:{:03}",
            self.year, self.month, self.day, self.hour, self.minute, self.second, self.millisecond
        )
    }

    /// The 16 ASCII digits the FBX SDK mixes into the identity blocks, in its field order
    /// `ss MM hh dd cc yyyy mm` (cc = centiseconds).
    fn to_mixing_digits(&self) -> [u8; 16] {
        let digits = format!(
            "{:02}{:02}{:02}{:02}{:02}{:04}{:02}",
            self.second,
            self.month,
            self.hour,
            self.day,
            self.millisecond / 10,
            self.year,
            self.minute
        );
        digits
            .as_bytes()
            .try_into()
            .expect("creation time fields are range-limited to 16 digits")
    }
}

/// The two 16-byte blocks the FBX SDK derives from the creation time: the `FileId` top-level
/// node and the first block of the binary footer. Readers that validate them (FBX SDK, Unity)
/// reject a file whose blocks do not match its `CreationTime`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileIdentity {
    pub creation_time: CreationTime,
    pub file_id: [u8; 16],
    pub footer_id: [u8; 16],
}

impl FileIdentity {
    pub(crate) fn from_creation_time(creation_time: CreationTime) -> Self {
        let digits = creation_time.to_mixing_digits();

        let file_id = xor_chain(SDK_IDENTITY_SEED, &digits);
        let footer_id = xor_chain(xor_chain(file_id, &SDK_FOOTER_KEY), &digits);

        Self {
            creation_time,
            file_id,
            footer_id,
        }
    }
}

/// Each output byte is the input byte xor the key byte xor the previous output byte.
fn xor_chain(mut block: [u8; 16], key: &[u8; 16]) -> [u8; 16] {
    let mut previous = XOR_CHAIN_SEED;
    for (byte, key_byte) in block.iter_mut().zip(key) {
        *byte ^= previous ^ key_byte;
        previous = *byte;
    }
    block
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLENDER_FILE_ID: [u8; 16] = [
        0x28, 0xb3, 0x2a, 0xeb, 0xb6, 0x24, 0xcc, 0xc2, 0xbf, 0xc8, 0xb0, 0x2a, 0xa9, 0x2b, 0xfc,
        0xf1,
    ];

    const BLENDER_FOOTER_ID: [u8; 16] = [
        0xfa, 0xbc, 0xab, 0x09, 0xd0, 0xc8, 0xd4, 0x66, 0xb1, 0x76, 0xfb, 0x83, 0x1c, 0xf7, 0x26,
        0x7e,
    ];

    #[test]
    fn default_creation_time_formats_like_blender() {
        assert_eq!(
            DEFAULT_CREATION_TIME.to_fbx_string(),
            "1970-01-01 10:00:00:000"
        );
    }

    #[test]
    fn default_creation_time_reproduces_the_blender_identity_blocks() {
        let identity = FileIdentity::from_creation_time(DEFAULT_CREATION_TIME);
        assert_eq!(identity.file_id, BLENDER_FILE_ID);
        assert_eq!(identity.footer_id, BLENDER_FOOTER_ID);
    }

    #[test]
    fn different_creation_times_give_different_blocks() {
        let base = FileIdentity::from_creation_time(DEFAULT_CREATION_TIME);
        let later = FileIdentity::from_creation_time(CreationTime {
            year: 2026,
            month: 10,
            day: 10,
            hour: 12,
            minute: 34,
            second: 56,
            millisecond: 789,
        });
        assert_ne!(base.file_id, later.file_id);
        assert_ne!(base.footer_id, later.footer_id);
    }
}
