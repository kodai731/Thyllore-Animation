use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    Vertex,
    TessellationControl,
    TessellationEvaluation,
    Geometry,
    Fragment,
    Compute,
    RayGeneration,
    Intersection,
    AnyHit,
    ClosestHit,
    Miss,
    Callable,
    Task,
    Mesh,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DescriptorKind {
    Sampler,
    CombinedImageSampler,
    SampledImage,
    StorageImage,
    UniformTexelBuffer,
    StorageTexelBuffer,
    UniformBuffer,
    StorageBuffer,
    InputAttachment,
    AccelerationStructure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptorCount {
    Fixed(u32),
    Unbounded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReflectedMember {
    pub name: String,
    pub offset: u32,
    pub size: u32,
    pub type_name: String,
    pub members: Vec<ReflectedMember>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReflectedBlock {
    pub type_name: String,
    pub size: u32,
    pub members: Vec<ReflectedMember>,
}

impl ReflectedMember {
    fn has_same_layout(&self, other: &ReflectedMember) -> bool {
        self.offset == other.offset
            && self.size == other.size
            && members_have_same_layout(&self.members, &other.members)
    }
}

fn members_have_same_layout(left: &[ReflectedMember], right: &[ReflectedMember]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(member, other)| member.has_same_layout(other))
}

impl ReflectedBlock {
    /// Same bytes at the same offsets. Block and member names are not part of the descriptor
    /// contract: a GLSL `buffer Table { T records[]; }` and a Slang `StructuredBuffer<T>` bind the
    /// same buffer while naming the block differently.
    pub fn has_same_layout(&self, other: &ReflectedBlock) -> bool {
        self.size == other.size && members_have_same_layout(&self.members, &other.members)
    }

    /// The struct a block wraps when its only member is one struct filling the block, so a GLSL
    /// `uniform Block { Payload payload; }` and a `buffer_reference` to the same struct reflect alike.
    pub fn single_struct_payload(&self) -> Option<ReflectedBlock> {
        let [member] = self.members.as_slice() else {
            return None;
        };
        if member.members.is_empty() || member.offset != 0 || member.size != self.size {
            return None;
        }
        Some(ReflectedBlock {
            type_name: member.type_name.clone(),
            size: member.size,
            members: member.members.clone(),
        })
    }
}

/// One push constant block as every stage of a pass must declare it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PushConstantLayout {
    pub block: &'static str,
    pub stages: &'static [ShaderStage],
    pub size: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReflectedBinding {
    pub set: u32,
    pub binding: u32,
    pub name: String,
    pub kind: DescriptorKind,
    pub count: DescriptorCount,
    pub block: Option<ReflectedBlock>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShaderBinding {
    pub set: u32,
    pub binding: u32,
    pub kind: DescriptorKind,
    pub count: DescriptorCount,
}

impl ReflectedBinding {
    pub fn shader_binding(&self) -> ShaderBinding {
        ShaderBinding {
            set: self.set,
            binding: self.binding,
            kind: self.kind,
            count: self.count,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShaderReflection {
    pub stages: Vec<ShaderStage>,
    pub bindings: Vec<ReflectedBinding>,
    pub push_constant: Option<ReflectedBlock>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ReflectError {
    #[error("SPIR-V magic number mismatch")]
    InvalidMagic,
    #[error("SPIR-V word stream is truncated")]
    Truncated,
    #[error("SPIR-V version {major}.{minor} is newer than the supported 1.0-1.6")]
    UnsupportedVersion { major: u32, minor: u32 },
    #[error("unknown SPIR-V execution model {0}")]
    UnknownExecutionModel(u32),
    #[error("SPIR-V type id {0} is not defined")]
    MissingType(u32),
    #[error("descriptor variable `{0}` has no Binding decoration")]
    MissingBinding(String),
    #[error("descriptor variable `{0}` has an unsupported type")]
    UnsupportedVariableType(String),
    #[error("push constant variable `{0}` is not a block")]
    PushConstantNotBlock(String),
    #[error("block member of `{0}` has no Offset decoration")]
    MissingMemberOffset(String),
    #[error("array type id {0} has a non-constant length")]
    NonConstantArrayLength(u32),
    #[error("set {set} binding {binding} is declared with conflicting types across shader stages")]
    ConflictingBinding { set: u32, binding: u32 },
}
