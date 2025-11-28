use std::{collections::HashSet, ops::Index, rc::Rc};

use crate::{
    maps::{TreeBijection, TreeEquivalence, TreeMap},
    tree::DAG,
};

pub struct TreeTransform<'a, T: Clone> {
    equivalence: &'a TreeEquivalence<T>,
    source_left: bool,
}

impl<'a, T: Clone> Index<&DAG<T>> for TreeTransform<'a, T> {
    type Output = Rc<DAG<T>>;

    fn index(&self, index: &DAG<T>) -> &Self::Output {
        if self.source_left {
            self.equivalence.bijection.lookup_right(index)
        } else {
            self.equivalence.bijection.lookup_left(index)
        }
    }
}

impl<'a, T: Clone> TreeTransform<'a, T> {
    pub fn new(equivalence: &'a TreeEquivalence<T>, source_left: bool) -> Self {
        TreeTransform {
            equivalence,
            source_left,
        }
    }

    fn source(&self) -> &Rc<DAG<T>> {
        if self.source_left {
            &self.equivalence.left
        } else {
            &self.equivalence.right
        }
    }

    fn target(&self) -> &Rc<DAG<T>> {
        if self.source_left {
            &self.equivalence.right
        } else {
            &self.equivalence.left
        }
    }

    fn source_embedding(&self, root: &Rc<DAG<T>>) -> TreeMap<T> {
        TreeMap::from_embedding(self.source(), root)
    }

    pub fn matches(&self, tree: &Rc<DAG<T>>) -> Vec<Rc<DAG<T>>> {
        let mut matched = Vec::new();

        let pattern_table = if self.source_left {
            &self.equivalence.left_pattern_table
        } else {
            &self.equivalence.right_pattern_table
        };

        tree.reduce(
            &mut |node, left_labels, right_labels| -> HashSet<usize> {
                let mut labels = HashSet::<usize>::from([0]);
                for ((left_label, right_label), label) in pattern_table {
                    if left_labels.contains(left_label) && right_labels.contains(right_label) {
                        if *label == pattern_table.len() {
                            matched.push(node.clone());
                        }
                        labels.insert(*label);
                    }
                }
                labels
            },
            &mut |_leaf, _value| [0].into(),
        );

        matched
    }

    pub fn apply(&self, root: &Rc<DAG<T>>, at: &Rc<DAG<T>>) -> (Rc<DAG<T>>, TreeEquivalence<T>) {
        let embedding = self.source_embedding(at);
        let mut result_map = TreeBijection::new();

        let replacement = self
            .target()
            .replace_leaves(&mut |leaf, _| result_map.track_clone(&embedding[&self[leaf]]));

        let result = result_map.track_insert(root, at, &replacement);
        let equivalence = TreeEquivalence::new(result.clone(), root.clone(), result_map);

        (result, equivalence)
    }
}
