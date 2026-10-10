pub trait SnapshotValues {
    fn snapshot_values(&self) -> Vec<f32>;
}

impl SnapshotValues for f32 {
    fn snapshot_values(&self) -> Vec<f32> {
        vec![*self]
    }
}

impl SnapshotValues for u32 {
    fn snapshot_values(&self) -> Vec<f32> {
        vec![*self as f32]
    }
}

impl SnapshotValues for bool {
    fn snapshot_values(&self) -> Vec<f32> {
        vec![u8::from(*self) as f32]
    }
}

impl<const N: usize> SnapshotValues for [f32; N] {
    fn snapshot_values(&self) -> Vec<f32> {
        self.to_vec()
    }
}

impl SnapshotValues for String {
    fn snapshot_values(&self) -> Vec<f32> {
        Vec::new()
    }
}

impl<T: SnapshotValues> SnapshotValues for Option<T> {
    fn snapshot_values(&self) -> Vec<f32> {
        self.as_ref().map(T::snapshot_values).unwrap_or_default()
    }
}
