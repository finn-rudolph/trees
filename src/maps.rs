use std::{
    borrow::Borrow,
    fmt::{Debug, Display},
    ops::{Index, Mul, MulAssign},
};

use crate::{
    perm::perms::{PermIndex, Permutation},
    term::{Term, TermRef},
};

pub type NodeIndex = PermIndex;

#[derive(Clone)]
pub struct TermMap<'a> {
    source: TermRef,
    target: TermRef,
    perm: Permutation<'a>,
}

impl<'a> TermMap<'a> {
    pub fn new(source: TermRef, target: TermRef, perm: Permutation<'a>) -> Self {
        TermMap {
            source,
            target,
            perm,
        }
    }

    pub fn source(&self) -> &TermRef {
        &self.source
    }

    pub fn target(&self) -> &TermRef {
        &self.target
    }

    pub fn backward(&self) -> TermMap<'static> {
        TermMap {
            perm: self.perm.inverse(),
            source: self.target.clone(),
            target: self.source.clone(),
        }
    }

    pub fn perm(&self) -> &Permutation<'a> {
        &self.perm
    }

    pub fn into_perm(self) -> Permutation<'a> {
        self.perm
    }

    pub fn into_backward(self) -> TermMap<'static> {
        TermMap {
            perm: self.perm.inverse(),
            source: self.target,
            target: self.source,
        }
    }
}

impl<'a> Index<NodeIndex> for TermMap<'a> {
    type Output = NodeIndex;
    fn index(&self, index: NodeIndex) -> &Self::Output {
        &self.perm._storage()[index as usize]
    }
}

impl<'a, B: Borrow<TermMap<'a>>> Mul<B> for &TermMap<'_> {
    type Output = TermMap<'static>;
    fn mul(self, rhs: B) -> Self::Output {
        let rhs_ref = rhs.borrow();
        TermMap {
            source: self.source.clone(),
            target: rhs_ref.target.clone(),
            perm: &self.perm * &rhs_ref.perm,
        }
    }
}

impl<'a, B: Borrow<TermMap<'a>>> MulAssign<B> for &mut TermMap<'_> {
    fn mul_assign(&mut self, rhs: B) {
        self.target = rhs.borrow().target().clone();
        self.perm *= &rhs.borrow().perm;
    }
}

impl<'a, B: Borrow<TermMap<'a>>> MulAssign<B> for TermMap<'_> {
    fn mul_assign(&mut self, rhs: B) {
        self.target = rhs.borrow().target().clone();
        self.perm *= &rhs.borrow().perm;
    }
}

impl Debug for TermMap<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let backward = self.perm.inverse();
        let formatted_target = self
            .target
            .label_with(&mut |index| backward.get(index as PermIndex).to_string());
        write!(f, "TreeMap[{} -> {}]", self.source, formatted_target)
    }
}

impl Display for TermMap<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let backward = self.perm.inverse();
        let formatted_target = self
            .target
            .label_with(&mut |index| backward.get(index as PermIndex).to_string());
        write!(f, "{} -> {}", self.source, formatted_target)
    }
}
