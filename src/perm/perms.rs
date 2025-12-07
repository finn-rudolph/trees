use std::{
    borrow::{Borrow, Cow},
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
        visited: &mut [bool],
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

    pub fn inverse(&self) -> Permutation<'static> {
        let mut inverse_map: Vec<PermIndex> = vec![0; self.perm.len()];

        self.perm.iter().enumerate().for_each(|(i, v)| {
            inverse_map[*v as usize] = i as PermIndex;
        });

        Permutation {
            perm: inverse_map.into(),
        }
    }

    pub fn _storage(&self) -> &Cow<'_, [PermIndex]> {
        &self.perm
    }

    pub fn times(&self, rhs: &Permutation<'_>) -> Permutation<'static> {
        let max_len = self.perm.len().max(rhs.perm.len()) as PermIndex;

        Permutation {
            perm: (0..max_len).map(|i| rhs.get(self.get(i))).collect(),
        }
    }

    pub fn times_assign(&mut self, rhs: &Permutation<'_>) {
        let rhs_len = rhs.perm.len() as PermIndex;
        let self_len = self.perm.len() as PermIndex;
        if rhs_len > self_len {
            self.perm.to_mut().extend(self_len..rhs_len);
        }

        self.perm.to_mut().iter_mut().for_each(|v| *v = rhs.get(*v));
    }
}

impl<'a, B: Borrow<Permutation<'a>>> Mul<B> for Permutation<'_> {
    type Output = Permutation<'static>;
    fn mul(self, rhs: B) -> Self::Output {
        self.times(rhs.borrow())
    }
}

impl<'a, B: Borrow<Permutation<'a>>> Mul<B> for &Permutation<'_> {
    type Output = Permutation<'static>;
    fn mul(self, rhs: B) -> Self::Output {
        self.times(rhs.borrow())
    }
}

impl<'a, B: Borrow<Permutation<'a>>> Mul<B> for &mut Permutation<'_> {
    type Output = Permutation<'static>;
    fn mul(self, rhs: B) -> Self::Output {
        self.times(rhs.borrow())
    }
}

impl<'a, B: Borrow<Permutation<'a>>> MulAssign<B> for Permutation<'_> {
    fn mul_assign(&mut self, rhs: B) {
        self.times_assign(rhs.borrow());
    }
}

impl<'a, T: Into<Cow<'a, [PermIndex]>>> From<T> for Permutation<'a> {
    fn from(value: T) -> Self {
        Self { perm: value.into() }
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
            if !self.perm.is_empty() {
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
