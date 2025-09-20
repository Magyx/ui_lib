use wayland_backend::smallvec::SmallVec;

pub enum Emit<M> {
    None,
    One(M),
    Many(SmallVec<[M; 2]>),
}

impl<M> Emit<M> {
    pub fn none() -> Self {
        Self::None
    }
    pub fn one(m: M) -> Self {
        Self::One(m)
    }
    pub fn many<I: IntoIterator<Item = M>>(it: I) -> Self {
        let mut v: SmallVec<[M; 2]> = SmallVec::new();
        v.extend(it);
        Self::Many(v)
    }
}

impl<M> IntoIterator for Emit<M> {
    type Item = M;
    type IntoIter = std::vec::IntoIter<M>;
    fn into_iter(self) -> Self::IntoIter {
        match self {
            Emit::None => Vec::new().into_iter(),
            Emit::One(m) => vec![m].into_iter(),
            Emit::Many(v) => v.into_vec().into_iter(),
        }
    }
}
