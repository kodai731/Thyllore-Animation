pub type PipelineId = usize;

#[derive(Clone, Debug)]
pub struct PipelineRef(pub PipelineId);

impl PipelineRef {
    pub fn new(id: PipelineId) -> Self {
        Self(id)
    }

    pub fn id(&self) -> PipelineId {
        self.0
    }
}

#[derive(Default, Debug)]
pub struct PipelineManager {
    count: usize,
}

impl PipelineManager {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    pub fn allocate_id(&mut self) -> PipelineId {
        let id = self.count;
        self.count += 1;
        id
    }

    pub fn count(&self) -> usize {
        self.count
    }
}
