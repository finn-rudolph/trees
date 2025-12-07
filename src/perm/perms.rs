use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    ops::{Mul, MulAssign},
};

pub type PermIndex = u16;

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct Permutation<'a> {
    perm: Cow<'a, [PermIndex]>,
}

impl<'a> Permutation<'a> {
    fn display_cycle(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        start: PermIndex,
        visited: &mut Vec<bool>,
        identity: &mut bool,
    ) -> std::fmt::Result {
        visited[start as usize] = true;

        let mut index = self.get(start);
        if index == start {
            return Ok(());
        }

        *identity = false;
        write!(f, "({}", start)?;
        loop {
            visited[index as usize] = true;
            write!(f, " {}", index)?;
            index = self.get(index);
            if index == start {
                return write!(f, ")");
            }
        }
    }

    pub fn shallow_clone(&'a self) -> Self {
        Permutation {
            perm: match &self.perm {
                Cow::Borrowed(value) => Cow::Borrowed(value),
                Cow::Owned(value) => Cow::Borrowed(value),
            },
        }
    }

    pub fn nonfix_index(&self) -> Option<PermIndex> {
        for (i, v) in self.perm.iter().enumerate() {
            if i != *v as usize {
                return Some(*v);
            }
        }
        None
    }

    pub fn is_identity(&self) -> bool {
        self.nonfix_index().is_none()
    }

    pub fn get(&self, index: PermIndex) -> PermIndex {
        if (index as usize) >= self.perm.len() {
            index
        } else {
            self.perm[index as usize]
        }
    }

    pub fn identity() -> Self {
        Permutation {
            perm: Vec::new().into(),
        }
    }

    pub fn inverse(&self) -> Permutation<'a> {
        let mut inverse_map: Vec<PermIndex> = vec![0; self.perm.len()];

        self.perm.iter().enumerate().for_each(|(i, v)| {
            inverse_map[*v as usize] = i as PermIndex;
        });

        Permutation {
            perm: inverse_map.into(),
        }
    }
}

impl<'a> Mul<&Permutation<'a>> for &Permutation<'a> {
    type Output = Permutation<'static>;
    fn mul(self, rhs: &Permutation<'a>) -> Self::Output {
        let max_len = self.perm.len().max(rhs.perm.len()) as PermIndex;

        Permutation {
            perm: (0..max_len)
                .into_iter()
                .map(|i| rhs.get(self.get(i)))
                .collect(),
        }
    }
}

impl<'a> Mul<&Permutation<'a>> for Permutation<'a> {
    type Output = Permutation<'static>;
    fn mul(self, rhs: &Permutation<'a>) -> Self::Output {
        &self * rhs
    }
}

impl<'a> Mul<&mut Permutation<'a>> for Permutation<'a> {
    type Output = Permutation<'static>;
    fn mul(self, rhs: &mut Permutation<'a>) -> Self::Output {
        &self * rhs
    }
}

impl<'a> Mul<&Permutation<'a>> for &mut Permutation<'a> {
    type Output = Permutation<'static>;
    fn mul(self, rhs: &Permutation<'a>) -> Self::Output {
        &*self * rhs
    }
}

impl<'a> MulAssign<&Permutation<'a>> for Permutation<'a> {
    fn mul_assign(&mut self, rhs: &Permutation<'a>) {
        let rhs_len = rhs.perm.len() as PermIndex;
        let self_len = self.perm.len() as PermIndex;
        if rhs_len > self_len {
            self.perm.to_mut().extend(self_len..rhs_len);
        }

        self.perm.to_mut().iter_mut().for_each(|v| *v = rhs.get(*v));
    }
}

impl<'a> MulAssign<Permutation<'a>> for Permutation<'a> {
    fn mul_assign(&mut self, rhs: Permutation<'a>) {
        *self *= &rhs;
    }
}

impl From<Vec<PermIndex>> for Permutation<'static> {
    fn from(value: Vec<PermIndex>) -> Self {
        Permutation { perm: value.into() }
    }
}

impl<'a> Debug for Permutation<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl<'a> Display for Permutation<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !f.alternate() {
            let mut visited = vec![false; self.perm.len()];
            let mut identity = true;
            if self.perm.len() != 0 {
                'inf: loop {
                    let mut start_index = 0;
                    while visited[start_index] {
                        start_index += 1;

                        if start_index == self.perm.len() {
                            break 'inf;
                        }
                    }
                    self.display_cycle(f, start_index as PermIndex, &mut visited, &mut identity)?;
                }
            }
            if identity { write!(f, "()") } else { Ok(()) }
        } else {
            write!(f, "(")?;

            for (i, v) in self.perm.iter().enumerate() {
                write!(f, "{}", v)?;
                if i == self.perm.len() - 1 {
                    write!(f, ")")?;
                } else {
                    write!(f, ", ")?;
                }
            }
            Ok(())
        }
    }
}
