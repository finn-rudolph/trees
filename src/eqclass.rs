use std::collections::HashMap;
use std::convert::identity;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Index, IndexMut};
use std::rc::Rc;

use crate::maps::{TreeBijection, TreeEquivalence, TreeMap};
use crate::tree::DAG;

pub struct EqClassEntry<T: Clone> {
    tree: Rc<DAG<T>>,
    map_to_parent: Option<TreeBijection<T>>,
    parent: usize,
    rank: usize,
}

pub struct EquivalenceClasses<T: Clone> {
    entries: Vec<EqClassEntry<T>>,
    index: HashMap<Rc<DAG<T>>, Vec<usize>>,
}

impl<T: Clone> Index<usize> for EquivalenceClasses<T> {
    type Output = EqClassEntry<T>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl<T: Clone> IndexMut<usize> for EquivalenceClasses<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl<T: Eq + Hash + Clone> EquivalenceClasses<T> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            index: HashMap::new(),
        }
    }

    fn add_entry(&mut self, tree: Rc<DAG<T>>) -> usize {
        let i = self.entries.len();

        self.entries.push(EqClassEntry {
            tree: tree.clone(),
            map_to_parent: None,
            parent: i,
            rank: 0,
        });

        self.index.entry(tree).or_default().push(i);
        i
    }

    fn index_for(&mut self, tree: &Rc<DAG<T>>) -> usize {
        if let Some(indicies) = self.index.get(tree) {
            return *indicies.first().unwrap();
        }

        let i = self.entries.len();
        self.index.insert(tree.clone(), [i].into());

        self.entries.push(EqClassEntry {
            tree: tree.clone(),
            map_to_parent: None,
            parent: i,
            rank: 0,
        });

        i
    }

    fn parent(&self, i: usize) -> usize {
        self.entries[i].parent
    }

    fn parent_entry(&self, i: usize) -> &EqClassEntry<T> {
        &self[self.parent(i)]
    }

    fn compress_parent(&mut self, i: usize) {
        if let Some(to_grand_parent) = &self.parent_entry(i).map_to_parent {
            let composition = self[i].map_to_parent.as_ref().unwrap() * to_grand_parent;
            self[i].map_to_parent = Some(composition);
            self[i].parent = self.parent_entry(i).parent;
        }
    }

    fn find(&mut self, mut i: usize) -> usize {
        while self.parent(i) != i {
            self.compress_parent(i);
            i = self.parent(i);
        }
        i
    }

    fn find_and_track(&mut self, mut i: usize, mut map_to_i: TreeMap<T>) -> (usize, TreeMap<T>) {
        while self.parent(i) != i {
            self.compress_parent(i);
            map_to_i *= &self[i].map_to_parent.as_ref().unwrap().left_to_right;
            i = self.parent(i);
        }

        (i, map_to_i)
    }

    fn find_and_track_immut(&self, mut i: usize, mut map_to_i: TreeMap<T>) -> (usize, TreeMap<T>) {
        while self.parent(i) != i {
            map_to_i *= &self[i].map_to_parent.as_ref().unwrap().left_to_right;
            i = self.parent(i);
        }

        (i, map_to_i)
    }

    pub fn add_equivalence(&mut self, equivalence: TreeEquivalence<T>) {
        let left = self.index_for(&equivalence.left);
        let right = self.index_for(&equivalence.right);

        let mut left_to_right = TreeMap::from_embedding(&self[left].tree, &equivalence.left);
        let right_embedding = TreeMap::from_embedding(&equivalence.right, &self[right].tree);

        left_to_right *= &equivalence.bijection.left_to_right;
        left_to_right *= &right_embedding;

        let bijection = left_to_right.upgrade(&self[left].tree);
        let (left_root, right_to_left_root) = self.find_and_track(left, bijection.right_to_left);
        let bijection = right_to_left_root.upgrade(&self[right].tree);
        let (right_root, left_root_to_right_root) =
            self.find_and_track(right, bijection.right_to_left);

        let mut bijection = left_root_to_right_root.upgrade(&self[left_root].tree);

        if left_root == right_root {
            if !bijection.is_idenity() {
                let mut clone_map = TreeBijection::new();
                let cloned_root = clone_map.track_clone(&self[right_root].tree);
                clone_map *= bijection;
                let i = self.add_entry(cloned_root);

                self[i].map_to_parent = Some(clone_map);
                self[i].parent = right_root;
            }
            return;
        }

        if self[left_root].rank < self[right_root].rank {
            self[left_root].parent = right_root;
            self[left_root].map_to_parent = Some(bijection);
        } else {
            self[right_root].parent = left_root;
            bijection.invert();
            self[right_root].map_to_parent = Some(bijection);
        }

        if self[left_root].rank == self[right_root].rank {
            self[left_root].rank += 1;
        }
    }
}

impl Debug for EquivalenceClasses<()> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            writeln!(f, "Entries: ")?;
            for (i, entry) in self.entries.iter().enumerate() {
                writeln!(f, "Entry: {}", i)?;
                writeln!(f, "\ttree: {:?}", entry.tree)?;
                writeln!(f, "\tparent: {:?}", entry.parent)?;
                writeln!(f, "\trank: {:?}", entry.parent)?;
                writeln!(f, "\tmap: {:?}", entry.map_to_parent)?;
            }
            return Ok(());
        }

        let mut roots = HashMap::new();
        let mut classes = Vec::new();

        for (i, entry) in self.entries.iter().enumerate() {
            if entry.parent == i {
                let label_map = entry.tree.label('a'..='z');

                let labeled_tree = entry.tree.map(&mut |leaf, _| {
                    *label_map.get(&(leaf.as_ref() as *const DAG<()>)).unwrap()
                });

                classes.push(vec![labeled_tree]);
                roots.insert(i, (classes.len() - 1, label_map));
                continue;
            }

            let idenity_map = entry.tree.identity_map();
            let (root, to_root) = self.find_and_track_immut(i, idenity_map);
            let (class_id, label_map) = roots.get(&root).unwrap();

            let labeled_tree = entry.tree.map(&mut |leaf, _| {
                *label_map
                    .get(&(to_root[leaf].as_ref() as *const DAG<()>))
                    .unwrap()
            });

            classes[*class_id].push(labeled_tree);
        }

        writeln!(f, "{} Equivalence Classes", classes.len())?;
        for (i, class) in classes.iter().enumerate() {
            writeln!(f, "Class {}", i)?;

            for tree in class {
                writeln!(f, "\t{}", tree)?;
            }
        }

        Ok(())
    }
}
