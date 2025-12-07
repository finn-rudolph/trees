use std::{collections::HashMap, fmt::Debug};

use crate::{indexing::IndexedTerm, maps::TermMap, perm::group::PermutationGroup, term::TermRef};

type EqClassEntryIndex = usize;

struct EqClassRootEntry {
    term: IndexedTerm,
    rank: usize,
    automorphisms: Option<PermutationGroup<'static>>,
}

impl EqClassRootEntry {
    pub fn into_child(
        self,
        parent: EqClassEntryIndex,
        parent_map: TermMap<'static>,
    ) -> EqClassEntry {
        EqClassEntry::Child(EqClassChildEntry {
            parent,
            parent_map,
            term: self.term,
        })
    }
}

struct EqClassChildEntry {
    term: IndexedTerm,
    parent: EqClassEntryIndex,
    parent_map: TermMap<'static>,
}

enum EqClassEntry {
    Root(EqClassRootEntry),
    Child(EqClassChildEntry),
}

impl EqClassEntry {
    pub fn new_root(term: &TermRef) -> Self {
        EqClassEntry::Root(EqClassRootEntry {
            term: IndexedTerm::from(term.clone()),
            rank: 0,
            automorphisms: None,
        })
    }

    pub fn is_root(&self) -> bool {
        matches!(self, Self::Root(_))
    }

    pub fn as_mut_root(&mut self) -> &mut EqClassRootEntry {
        match self {
            EqClassEntry::Root(root) => root,
            EqClassEntry::Child(_) => panic!("as_root called on non-child entry"),
        }
    }

    pub fn as_mut_child(&mut self) -> &mut EqClassChildEntry {
        match self {
            EqClassEntry::Child(child) => child,
            EqClassEntry::Root(_) => panic!("as_mut_child called on non-child entry"),
        }
    }

    pub fn as_root(&self) -> &EqClassRootEntry {
        match self {
            EqClassEntry::Root(root) => root,
            EqClassEntry::Child(_) => panic!("as_root called on non-child entry"),
        }
    }

    pub fn as_child(&self) -> &EqClassChildEntry {
        match self {
            EqClassEntry::Child(child) => child,
            EqClassEntry::Root(_) => panic!("as_child called on non-child entry"),
        }
    }
}

pub struct EquivalenceClasses {
    entries: Vec<EqClassEntry>,
    by_shape: HashMap<TermRef, EqClassEntryIndex>,
}

impl EquivalenceClasses {
    pub fn new() -> Self {
        EquivalenceClasses {
            entries: Vec::new(),
            by_shape: HashMap::new(),
        }
    }

    fn parent_of(&self, index: EqClassEntryIndex) -> Option<EqClassEntryIndex> {
        match &self.entries[index] {
            EqClassEntry::Root(_) => None,
            EqClassEntry::Child(child) => Some(child.parent),
        }
    }

    fn entry_for_term(&mut self, term: &TermRef) -> EqClassEntryIndex {
        *self.by_shape.entry(term.clone()).or_insert_with(|| {
            let entry = EqClassEntry::new_root(term);
            self.entries.push(entry);
            self.entries.len() - 1
        })
    }

    pub fn add_equiv(&mut self, map: TermMap) {
        let target = self.entry_for_term(map.target());
        let source = self.entry_for_term(map.source());
        let mut source_to_target_root = map;
        let mut target_root = self.find(target, Some(&mut source_to_target_root));
        let mut target_root_to_source_root = source_to_target_root.into_backward();
        let mut source_root = self.find(source, Some(&mut target_root_to_source_root));

        if target_root == source_root {
            let root_entry = self.entries[target_root].as_mut_root();
            let perm = target_root_to_source_root.into_perm();

            if let Some(non_fixpoint) = perm.nonfix_index() {
                root_entry
                    .automorphisms
                    .get_or_insert_with(|| PermutationGroup::new(non_fixpoint))
                    .extend(perm);
            }

            return;
        }

        let [source_entry, target_entry] = self
            .entries
            .get_disjoint_mut([source_root, target_root])
            .unwrap();
        let (mut source_entry, mut target_entry) =
            (source_entry.as_mut_root(), target_entry.as_mut_root());

        if source_entry.rank < target_entry.rank {
            std::mem::swap(&mut source_root, &mut target_root);
            std::mem::swap(&mut source_entry, &mut target_entry);
            target_root_to_source_root = target_root_to_source_root.into_backward();
        } else if source_entry.rank == target_entry.rank {
            source_entry.rank += 1;
        }

        // FIXME: Is there really no better way to do this?
        if let EqClassEntry::Root(target_owned) = self.entries.swap_remove(target_root) {
            let last_index = self.entries.len();
            self.entries
                .push(target_owned.into_child(source_root, target_root_to_source_root));
            self.entries.swap(target_root, last_index);
        } else {
            unreachable!()
        }
    }

    fn find(
        &mut self,
        mut index: EqClassEntryIndex,
        mut tracking_map: Option<&mut TermMap>,
    ) -> EqClassEntryIndex {
        loop {
            match self.parent_of(index) {
                None => {
                    return index;
                }
                Some(parent) => {
                    let [index_entry, parent_entry] = self
                        .entries
                        .get_disjoint_mut([index, parent])
                        .expect("child entry has itself as parent");
                    let child_mut = index_entry.as_mut_child();

                    if let EqClassEntry::Child(parent_inner) = parent_entry {
                        child_mut.parent_map *= &parent_inner.parent_map;
                        child_mut.parent = parent_inner.parent;
                    }

                    index = child_mut.parent;

                    if let Some(map) = &mut tracking_map {
                        **map *= &child_mut.parent_map;
                    }
                }
            }
        }
    }

    fn find_immut(
        &self,
        mut index: EqClassEntryIndex,
        mut tracking_map: &mut TermMap,
    ) -> EqClassEntryIndex {
        loop {
            match &self.entries[index] {
                EqClassEntry::Child(child) => {
                    tracking_map *= &child.parent_map;
                    index = child.parent;
                }
                EqClassEntry::Root(_) => {
                    return index;
                }
            }
        }
    }
}

impl Debug for EquivalenceClasses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut classes = HashMap::new();

        for (i, entry) in self.entries.iter().enumerate() {
            match entry {
                EqClassEntry::Root(_) => {
                    classes.insert(i, Vec::new());
                }
                EqClassEntry::Child(child) => {
                    let mut map_to_root = child.term.term().identity_map();
                    let root = self.find_immut(i, &mut map_to_root);

                    classes
                        .entry(root)
                        .or_insert_with(|| Vec::new())
                        .push(map_to_root);
                }
            }
        }

        writeln!(f, "{} Equivalence Classes:", classes.len())?;
        for (i, (root_index, maps)) in classes.iter().enumerate() {
            writeln!(f, "Class {}:", i)?;
            let root_entry = self.entries[*root_index].as_root();
            writeln!(f, "\tTerm   : {}", root_entry.term.term())?;
            if let Some(morphs) = &root_entry.automorphisms {
                writeln!(f, "\tMorphs : {:?}", morphs)?;
            }

            if maps.len() > 0 {
                writeln!(f, "\tChildren ({}):", maps.len())?;
                for map in maps {
                    writeln!(f, "\t\t{}", map.backward())?;
                }
            }
        }
        Ok(())
    }
}
