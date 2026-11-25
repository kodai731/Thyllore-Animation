pub(crate) mod components;
pub(crate) mod systems;

pub use components::chain_gen::{
    attachment_or_default, chain_column, chain_name, default_attachment, default_root_of_submesh,
    end_bone_name, joint_name, submesh_label, AzimuthStrandSettings, BodyCollider,
    CardStrandSettings, ChainGenInput, ChainGenOwner, ChainGenParams, ChainGenReport,
    ChainGenRequest, ChainGenSettings, ChainPlacement, ChainWeightRecord, CoveredVertex,
    GarmentSupport, GarmentSupportSettings, GeneratedBoneRecord, GeneratedChain, GeneratedJoint,
    GivenChain, HeadCapsuleRecord, InfluenceTarget, RootAttachment, RootBandRecord, SkinSurface,
    StrandCenterlineRecord, StrandMode, StrandPlacement, VertexInfluences, CHAIN_GEN_SCALAR_PARAMS,
    CHAIN_GEN_UI_PARAMS, SUPPORT_RING_HEIGHT,
};
pub use components::hidden_mask::{HiddenMaskParams, MaskMesh};
pub use components::submesh_role::{
    guess_submesh_role, refresh_submesh_roles, SubmeshKey, SubmeshRole, SubmeshSelector,
};
pub use systems::chain_bones::{append_chain_bones, write_chain_weights, ChainBoneIds};
pub use systems::chain_gen::{
    apply_setting_assignments, cap_root_band, component_labels, compute_chain_weights,
    place_chains, place_chains_with_strands, root_attachment_positions, root_head_capsule,
    setting_names, RootBand,
};
pub use systems::collider_surface::{collider_surfaces, retain_torso_skin, supports_garments};
pub use systems::hidden_mask::{
    find_hidden_body_triangles, find_hidden_body_vertices, find_hidden_triangle_indices,
    mask_mesh_from_skin, merge_mask_meshes, order_visible_triangles_first,
};
pub use systems::joint_radius::joint_half_thicknesses;
