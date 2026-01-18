use std::marker::PhantomData;

pub struct HNil;

pub struct HCons<H, T> {
    pub head: H,
    pub tail: T,
}

impl<H, T> HCons<H, T> {
    pub fn new(head: H, tail: T) -> Self {
        Self { head, tail }
    }
}

pub trait HList {
    fn len() -> usize;
}

impl HList for HNil {
    fn len() -> usize {
        0
    }
}

impl<H, T: HList> HList for HCons<H, T> {
    fn len() -> usize {
        1 + T::len()
    }
}

pub trait HListGet<Index> {
    type Output;
    fn get(&self) -> &Self::Output;
}

pub struct Here;
pub struct There<T>(PhantomData<T>);

impl<H, T> HListGet<Here> for HCons<H, T> {
    type Output = H;
    fn get(&self) -> &Self::Output {
        &self.head
    }
}

impl<H, T, Index> HListGet<There<Index>> for HCons<H, T>
where
    T: HListGet<Index>,
{
    type Output = T::Output;
    fn get(&self) -> &Self::Output {
        self.tail.get()
    }
}
