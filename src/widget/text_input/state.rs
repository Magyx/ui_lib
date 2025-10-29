use std::cell::{Ref, RefMut};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::{cell::RefCell, rc::Rc};

#[derive(Default, Clone)]
pub struct TextEditState {
    pub(crate) caret: usize,
    pub(crate) value: String,
}

#[derive(Clone, Default)]
pub struct TextState(Rc<RefCell<TextEditState>>);

impl TextState {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(TextEditState::default())))
    }

    pub fn from_value<V: Into<String>>(v: V) -> Self {
        let mut s = TextEditState::default();
        s.value = v.into();
        s.caret = s.value.len();
        Self(Rc::new(RefCell::new(s)))
    }

    pub fn len(&self) -> usize {
        self.0.borrow().value.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.borrow().value.is_empty()
    }
    pub fn get(&self) -> String {
        self.0.borrow().value.clone()
    }

    pub fn set<V: Into<String>>(&self, v: V) {
        let mut s = self.0.borrow_mut();
        s.value = v.into();
        s.caret = s.value.len();
    }
    pub fn set_preserving_caret<V: Into<String>>(&self, v: V) {
        let mut s = self.0.borrow_mut();
        let new_val: String = v.into();
        let old_caret = s.caret;
        s.value = new_val;
        s.caret = old_caret.min(s.value.len());
    }
    pub fn clear(&self) {
        let mut s = self.0.borrow_mut();
        s.value.clear();
        s.caret = 0;
    }
    pub fn push_str(&self, x: &str) {
        let mut s = self.0.borrow_mut();
        s.value.push_str(x);
        s.caret = s.value.len();
    }

    pub(crate) fn cell_ref(&self) -> Ref<'_, TextEditState> {
        self.0.borrow()
    }
    pub(crate) fn cell_mut(&mut self) -> RefMut<'_, TextEditState> {
        self.0.borrow_mut()
    }
}

impl Display for TextState {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.get())
    }
}
impl std::fmt::Debug for TextState {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_tuple("TextState").field(&self.get()).finish()
    }
}

impl From<&str> for TextState {
    fn from(s: &str) -> Self {
        TextState::from_value(s)
    }
}
impl From<String> for TextState {
    fn from(s: String) -> Self {
        TextState::from_value(s)
    }
}
impl From<&TextState> for String {
    fn from(ts: &TextState) -> Self {
        ts.get()
    }
}

impl PartialEq<str> for TextState {
    fn eq(&self, other: &str) -> bool {
        self.get() == other
    }
}
impl PartialEq<&str> for TextState {
    fn eq(&self, other: &&str) -> bool {
        self.get() == *other
    }
}
impl PartialEq<String> for TextState {
    fn eq(&self, other: &String) -> bool {
        self.get() == *other
    }
}
impl PartialEq<TextState> for TextState {
    fn eq(&self, other: &TextState) -> bool {
        self.get() == other.get()
    }
}

impl std::ops::AddAssign<&str> for TextState {
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs)
    }
}
