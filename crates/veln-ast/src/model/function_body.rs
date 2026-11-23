use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use super::BodyLine;

#[derive(Clone, Debug, Default)]
pub struct FunctionBody(Arc<Vec<BodyLine>>);

impl From<Vec<BodyLine>> for FunctionBody {
    fn from(lines: Vec<BodyLine>) -> Self {
        Self(Arc::new(lines))
    }
}

impl FromIterator<BodyLine> for FunctionBody {
    fn from_iter<T: IntoIterator<Item = BodyLine>>(iter: T) -> Self {
        Vec::from_iter(iter).into()
    }
}

impl Deref for FunctionBody {
    type Target = Vec<BodyLine>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FunctionBody {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

impl<'a> IntoIterator for &'a FunctionBody {
    type Item = &'a BodyLine;
    type IntoIter = std::slice::Iter<'a, BodyLine>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut FunctionBody {
    type Item = &'a mut BodyLine;
    type IntoIter = std::slice::IterMut<'a, BodyLine>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl IntoIterator for FunctionBody {
    type Item = BodyLine;
    type IntoIter = std::vec::IntoIter<BodyLine>;

    fn into_iter(self) -> Self::IntoIter {
        Arc::unwrap_or_clone(self.0).into_iter()
    }
}
