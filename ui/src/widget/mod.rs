use std::marker::PhantomData;

pub struct Element<M> {
    _marker: PhantomData<M>,
}

impl<M> Element<M> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}
