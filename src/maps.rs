use std::{
    borrow::Cow,
    fmt::Debug,
    ops::{Index, Mul, MulAssign, Not},
};

use crate::{labeled::LabeledTerm, term::TermRef};

pub type NodeIndex = usize;

pub struct TermMap<'a> {
    source: TermRef,
    target: TermRef,
    map: Cow<'a, [NodeIndex]>,
}

impl<'a> TermMap<'a> {
    pub fn new(source: TermRef, target: TermRef, map: Cow<'a, [NodeIndex]>) -> Self {
        TermMap {
            source,
            target,
            map,
        }
    }

    fn backward_map(&self) -> Vec<Option<NodeIndex>> {
        let max_len = self.map.iter().max().map_or(0, |v| *v + 1);
        let mut backward_map = vec![None; max_len];

        for (s, t) in self.map.iter().enumerate() {
            backward_map[*t] = Some(s);
        }

        backward_map
    }

    pub fn upgrade(self) -> TermBijection<'a> {
        let backward: Vec<NodeIndex> = self
            .backward_map()
            .iter()
            .map(|x| x.expect("Not an invertable map"))
            .collect();

        TermBijection {
            source: self.source,
            target: self.target,
            forward: self.map,
            backward: backward.into(),
        }
    }
}

impl<'a> Index<NodeIndex> for TermMap<'a> {
    type Output = NodeIndex;
    fn index(&self, index: NodeIndex) -> &Self::Output {
        &self.map[index]
    }
}

impl<'a> MulAssign<&TermMap<'_>> for TermMap<'a> {
    fn mul_assign(&mut self, rhs: &TermMap<'_>) {
        self.map.to_mut().iter_mut().for_each(|v| *v = rhs.map[*v]);
    }
}

impl Mul<&TermMap<'_>> for &TermMap<'_> {
    type Output = TermMap<'static>;
    fn mul(self, rhs: &TermMap) -> Self::Output {
        TermMap {
            source: self.source.clone(),
            target: rhs.target.clone(),
            map: self.map.iter().map(|v| rhs.map[*v]).collect(),
        }
    }
}

impl Mul<&TermBijection<'_>> for &TermMap<'_> {
    type Output = TermMap<'static>;
    fn mul(self, rhs: &TermBijection) -> Self::Output {
        self * &rhs.forward()
    }
}

impl<'a> MulAssign<&TermBijection<'_>> for TermMap<'a> {
    fn mul_assign(&mut self, rhs: &TermBijection<'_>) {
        *self *= &rhs.forward()
    }
}

impl Debug for TermMap<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let backward_map = self.backward_map();
        let formatted_target = self.target.label_with(&mut |index| {
            let target_value: Option<usize> = backward_map[index];
            target_value.map_or(String::from("<unk>"), |v| v.to_string())
        });
        write!(f, "TreeMap[{} -> {}]", self.source, formatted_target)
    }
}

impl Debug for TermBijection<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        assert!(
            self.check_bijection(),
            "TermBijection is not actually a bijection"
        );
        let formatted_target = self
            .target
            .label_with(&mut |index| self.backward()[index].to_string());
        write!(f, "TreeBijection[{} <=> {}]", self.source, formatted_target)
    }
}

pub struct TermBijection<'a> {
    source: TermRef,
    target: TermRef,
    forward: Cow<'a, [NodeIndex]>,
    backward: Cow<'a, [NodeIndex]>,
}

impl<'a> TermBijection<'a> {
    pub fn forward(&self) -> TermMap {
        TermMap {
            source: self.source.clone(),
            target: self.target.clone(),
            map: Cow::Borrowed(&self.forward),
        }
    }

    pub fn backward(&self) -> TermMap {
        TermMap {
            source: self.target.clone(),
            target: self.source.clone(),
            map: Cow::Borrowed(&self.backward),
        }
    }

    pub fn check_bijection(&self) -> bool {
        if self.forward.len() != self.backward.len() {
            return false;
        }

        self.forward
            .iter()
            .enumerate()
            .all(|(i, v)| self.backward[*v] == i)
    }

    pub fn source(&self) -> &TermRef {
        &self.source
    }

    pub fn target(&self) -> &TermRef {
        &self.target
    }

    pub fn invert(&mut self) {
        std::mem::swap(&mut self.forward, &mut self.backward);
        std::mem::swap(&mut self.source, &mut self.target);
    }
}

impl<'a> MulAssign<&TermBijection<'_>> for TermBijection<'a> {
    fn mul_assign(&mut self, rhs: &TermBijection<'_>) {
        self.forward
            .to_mut()
            .iter_mut()
            .for_each(|v| *v = rhs.forward[*v]);

        self.backward = rhs.backward.iter().map(|v| self.backward[*v]).collect();
    }
}

impl<'a> Mul<&TermBijection<'_>> for &TermBijection<'a> {
    type Output = TermBijection<'a>;
    fn mul(self, rhs: &TermBijection) -> Self::Output {
        TermBijection {
            source: self.source.clone(),
            target: rhs.target.clone(),
            forward: self.forward.iter().map(|v| rhs.forward[*v]).collect(),
            backward: rhs.backward.iter().map(|v| self.backward[*v]).collect(),
        }
    }
}

impl Mul<&TermMap<'_>> for &TermBijection<'_> {
    type Output = TermMap<'static>;
    fn mul(self, rhs: &TermMap) -> Self::Output {
        &self.forward() * rhs
    }
}

impl<'b, 'a: 'b> Not for &'a TermBijection<'b> {
    type Output = TermBijection<'b>;
    fn not(self) -> Self::Output {
        TermBijection {
            backward: Cow::Borrowed(&self.forward),
            forward: Cow::Borrowed(&self.backward),
            source: self.target.clone(),
            target: self.source.clone(),
        }
    }
}
