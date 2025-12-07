use std::{borrow::Cow, collections::VecDeque, fmt::Debug};

use crate::perm::perms::{PermIndex, Permutation};

/// An implementation of the Schreier–Sims algorithm
/// See for details:
/// - https://en.wikipedia.org/wiki/Schreier%E2%80%93Sims_algorithm
/// - https://arxiv.org/pdf/math/9201304
/// - https://blogs.cs.st-andrews.ac.uk/codima/files/2015/11/CoDiMa2015_Holt.pdf

struct PermutationGroup<'a> {
    stab_point: PermIndex,
    stab_subgroup: Option<Box<PermutationGroup<'a>>>,
    generators: Vec<Permutation<'a>>,
    // these are the invserses of the elements usually in transversal systems, but this makes `contains` faster.
    transversal_inv: Vec<Option<Permutation<'a>>>,
    orbits: Vec<PermIndex>,
}

impl<'a> PermutationGroup<'a> {
    fn inv_coset_repr(&self, orbit: PermIndex) -> &Option<Permutation<'a>> {
        if orbit as usize >= self.transversal_inv.len() {
            &None
        } else {
            &self.transversal_inv[orbit as usize]
        }
    }

    pub fn from_generators(generators: Vec<Permutation<'a>>) -> Self {
        let stab_point = generators
            .iter()
            .map(|perm| perm.nonfix_index())
            .skip_while(|index| index.is_none())
            .next()
            .flatten()
            .expect("No non-identity generator");

        let mut group = Self::new(stab_point);
        for generator in generators {
            group.extend(generator);
        }
        group
    }

    pub fn new(stab_point: PermIndex) -> Self {
        PermutationGroup {
            stab_subgroup: None,
            stab_point,
            generators: Vec::new(),
            transversal_inv: (0..=stab_point)
                .map(|i| {
                    if i == stab_point {
                        Some(Permutation::identity())
                    } else {
                        None
                    }
                })
                .collect(),
            orbits: vec![stab_point],
        }
    }

    pub fn contains(&self, perm: &Permutation<'a>) -> bool {
        self.contains_owned(perm.clone())
    }

    pub fn contains_owned(&self, mut perm: Permutation<'a>) -> bool {
        let orbit = perm.get(self.stab_point);

        if let Some(inv_coset_repr) = self.inv_coset_repr(orbit) {
            perm *= inv_coset_repr;
            if let Some(subgroup) = &self.stab_subgroup {
                subgroup.contains_owned(perm)
            } else {
                perm.is_identity()
            }
        } else {
            false
        }
    }

    pub fn extend(&mut self, generator: Permutation<'a>) {
        if self.contains(&generator) {
            return;
        }

        self.generators.push(generator.clone());
        let mut generator_inv = None;

        fn process_orbit<'a>(
            group: &mut PermutationGroup<'a>,
            generator: &Permutation<'a>,
            generator_inv: &mut Option<Permutation<'a>>,
            orbit: PermIndex,
            queue: &mut VecDeque<PermIndex>,
        ) {
            let inv_coset_repr = group.inv_coset_repr(orbit).as_ref().unwrap();
            let new_orbit = generator.get(orbit);

            if let Some(new_inv_coset_repr) = group.inv_coset_repr(new_orbit) {
                let subgroup_generator = inv_coset_repr.inverse() * generator * new_inv_coset_repr;

                if let Some(non_fixpoint) = subgroup_generator.nonfix_index() {
                    let subgroup = group
                        .stab_subgroup
                        .get_or_insert_with(|| Box::new(PermutationGroup::new(non_fixpoint)));

                    subgroup.extend(subgroup_generator);
                }
            } else {
                let translated_inv_coset_repr =
                    generator_inv.get_or_insert_with(|| generator.inverse()) * inv_coset_repr;

                if (new_orbit as usize) >= group.transversal_inv.len() {
                    group
                        .transversal_inv
                        .resize_with(new_orbit as usize + 1, || None);
                }
                group.transversal_inv[new_orbit as usize] = Some(translated_inv_coset_repr);

                group.orbits.push(new_orbit);
                queue.push_back(new_orbit);
            }
        }

        let mut queue = VecDeque::new();

        for i in 0..self.orbits.len() {
            let orbit = self.orbits[i];
            process_orbit(self, &generator, &mut generator_inv, orbit, &mut queue);
        }

        while let Some(orbit) = queue.pop_front() {
            process_orbit(self, &generator, &mut generator_inv, orbit, &mut queue);
        }
    }
}

impl<'a> Debug for PermutationGroup<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PermutationGroup[{}, {:?}, orbits: {:?}]",
            self.stab_point, self.generators, self.orbits
        )?;

        if f.alternate() {
            write!(f, "\n")?;
            if let Some(subgroup) = &self.stab_subgroup {
                Debug::fmt(&subgroup, f)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group() {
        let group = PermutationGroup::from_generators(vec![
            Permutation::from(vec![1, 2, 0]),
            Permutation::from(vec![3, 1, 2, 0]),
        ]);

        println!("{:#?}", group);
        println!(
            "contains (0 1 2 3): {}",
            group.contains(&Permutation::from(vec![1, 2, 3, 0]))
        );

        println!(
            "contains (0 1 2 3): {}",
            group.contains(&Permutation::from(vec![1, 0, 2, 3]))
        );
    }
}
