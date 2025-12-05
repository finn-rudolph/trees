use std::hash::Hash;

use crate::term::Term;

pub struct TermByAddress<'a>(&'a Term);

impl<'a> Hash for TermByAddress<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self.0 as *const Term).hash(state);
    }
}

impl AsRef<Term> for TermByAddress<'_> {
    fn as_ref(&self) -> &Term {
        self.0
    }
}

impl<'a> PartialEq for &TermByAddress<'a> {
    fn eq(&self, other: &Self) -> bool {
        (self.0 as *const Term) == (other.0 as *const Term)
    }
}

impl<'a> Eq for &TermByAddress<'a> {}

impl<'a> From<&'a Term> for TermByAddress<'a> {
    fn from(value: &'a Term) -> Self {
        Self(value)
    }
}
