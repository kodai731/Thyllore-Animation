pub trait UboPack<T> {
    fn pack(&self, ubo: &mut T);
}
