use strum::VariantArray;
use thyllore_anim_core::spring_bone::RootAttachment;

/// A root attachment as the addon shows it: its sidecar name, its label, its description and the
/// Blender bone color theme (`THEME01` .. `THEME20`) the chains it generates are drawn with.
#[derive(serde::Serialize)]
pub struct RootAttachmentEntry {
    pub name: String,
    pub label: &'static str,
    pub description: &'static str,
    pub bone_palette: u8,
}

const AUTHORED_CHAIN_PALETTE: u8 = 6;

/// The bone color theme of a chain: one per attachment, another for chains authored by hand.
pub fn chain_bone_palette(attachment: Option<RootAttachment>) -> u8 {
    match attachment {
        Some(RootAttachment::Cap) => 1,
        Some(RootAttachment::Ring) => 4,
        None => AUTHORED_CHAIN_PALETTE,
    }
}

pub fn sidecar_name(attachment: RootAttachment) -> anyhow::Result<String> {
    match serde_json::to_value(attachment)? {
        serde_json::Value::String(name) => Ok(name),
        other => anyhow::bail!("root attachment serializes to {other}, not a string"),
    }
}

pub fn parse_attachment(name: &str) -> anyhow::Result<RootAttachment> {
    serde_json::from_value(serde_json::Value::String(name.to_string()))
        .map_err(|_| anyhow::anyhow!("unknown root attachment: {name}"))
}

pub fn root_attachments() -> anyhow::Result<Vec<RootAttachmentEntry>> {
    RootAttachment::VARIANTS
        .iter()
        .copied()
        .map(|attachment| {
            Ok(RootAttachmentEntry {
                name: sidecar_name(attachment)?,
                label: attachment.label(),
                description: attachment.description(),
                bone_palette: chain_bone_palette(Some(attachment)),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_names_round_trip() {
        for attachment in RootAttachment::VARIANTS.iter().copied() {
            let name = sidecar_name(attachment).unwrap();
            assert_eq!(parse_attachment(&name).unwrap(), attachment);
        }
        assert!(parse_attachment("Cap").is_err());
    }

    #[test]
    fn every_attachment_has_its_own_palette() {
        let entries = root_attachments().unwrap();
        let mut palettes: Vec<u8> = entries.iter().map(|e| e.bone_palette).collect();
        palettes.push(chain_bone_palette(None));
        palettes.sort_unstable();
        palettes.dedup();
        assert_eq!(palettes.len(), RootAttachment::VARIANTS.len() + 1);
    }
}
