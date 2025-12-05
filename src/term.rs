use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    hash::Hash,
    rc::Rc,
};

use crate::{
    bidag::{BinaryChildren, BinaryDirectedAcyclicGraph},
    maps::{NodeIndex, TermBijection, TermMap},
};

#[derive(Eq, PartialEq, Hash)]
pub enum Term {
    Variable,
    Operation(TermRef, TermRef),
}

pub struct TermByAddress(Rc<Term>);

impl Hash for TermByAddress {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self.0.as_ref() as *const Term).hash(state);
    }
}

impl PartialEq for &TermByAddress {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for &TermByAddress {}

impl From<TermRef> for TermByAddress {
    fn from(value: TermRef) -> Self {
        Self(value)
    }
}

pub type TermRef = Rc<Term>;

impl Term {
    pub fn label<T, I: Iterator<Item = T>>(
        self: &TermRef,
        mut iter: I,
    ) -> Option<LabeledTermRef<T>> {
        self.try_map(&mut |_| iter.next())
    }

    pub fn label_with<T, F: FnMut(&TermRef, usize) -> T>(
        self: &TermRef,
        mut labeler: F,
    ) -> LabeledTermRef<T> {
        let mut count = 0;
        self.map(
            &mut #[inline(always)]
            |leaf| {
                let label = labeler(leaf, count);
                count += 1;
                label
            },
        )
    }

    pub fn clone_with_leaf_count(self: &TermRef) -> (TermRef, usize) {
        let mut leaf_count = 0;
        (
            self.replace_leaves(&mut |_| {
                leaf_count += 1;
                Rc::new(Term::Variable)
            }),
            leaf_count,
        )
    }

    fn insert_replacements_helper(
        self: &TermRef,
        match_root: &TermByAddress,
        replacements: &Vec<(TermRef, NodeIndex, NodeIndex)>,
        bijection: &TermBijection<'_>,
        leaf_index: &mut NodeIndex,
        computed_map: &mut Vec<NodeIndex>,
    ) -> TermRef {
        match self.children() {
            None => {
                computed_map.push(*leaf_index);
                *leaf_index += 1;
                Rc::new(Self::Variable)
            }
            Some((left, right)) => {
                if &TermByAddress::from(self.clone()) == match_root {
                    bijection
                        .target()
                        .replace_leaves_with_count(&mut |_, target_leaf_index| {
                            let translated_index = bijection.backward()[target_leaf_index];
                            let (replacement, start, end) = &replacements[translated_index];
                            computed_map.extend((start + *leaf_index)..(end + *leaf_index));
                            *leaf_index += end - start;
                            replacement.clone()
                        })
                } else {
                    let left_result = left.insert_replacements_helper(
                        match_root,
                        replacements,
                        bijection,
                        leaf_index,
                        computed_map,
                    );
                    let right_result = right.insert_replacements_helper(
                        match_root,
                        replacements,
                        bijection,
                        leaf_index,
                        computed_map,
                    );

                    Rc::new(Term::Operation(left_result, right_result))
                }
            }
        }
    }

    pub fn substitute<'a>(
        self: &TermRef,
        match_root: TermByAddress,
        bijection: TermBijection<'_>,
    ) -> TermBijection<'static> {
        // replacements[i] = (replacement, a, b) such that replacment is a copy of the tree at
        // the i-th leaf of the embedded source. The origial tree has the leaves [a, b) in `match_root`.
        let mut replacements = Vec::new();
        let mut replacement_leaf_index = 0;

        bijection.source().propagate(
            &match_root.0,
            &mut |_, embedded_node| {
                embedded_node
                    .children()
                    .expect("match_root not embedded here")
            },
            &mut |_, embedded_node| {
                let (replacement, replace_size) = embedded_node.clone_with_leaf_count();
                replacements.push((
                    replacement,
                    replacement_leaf_index,
                    replacement_leaf_index + replace_size,
                ));
                replacement_leaf_index += replace_size;
            },
        );

        let mut computed_map = Vec::new();
        let mut result_leaf_index = 0;
        let result = self.insert_replacements_helper(
            &match_root,
            &replacements,
            &bijection,
            &mut result_leaf_index,
            &mut computed_map,
        );

        let result_map_backward = TermMap::new(self.clone(), result, computed_map.into());
        let mut result_bijection = result_map_backward.upgrade();
        result_bijection.invert();
        result_bijection
    }
}

impl BinaryDirectedAcyclicGraph<()> for TermRef {
    fn children(&self) -> Option<(&Self, &Self)> {
        match self.as_ref() {
            Term::Variable => None,
            Term::Operation(left, right) => Some((left, right)),
        }
    }

    fn from_children(left: Self, right: Self) -> Self {
        Rc::new(Term::Operation(left, right))
    }

    fn from_leaf(_value: ()) -> Self {
        Rc::new(Term::Variable)
    }
}

pub type LabeledTermRef<T> = Rc<LabeledTerm<T>>;

pub enum LabeledTerm<T> {
    Variable(T),
    Operation(Rc<LabeledTerm<T>>, Rc<LabeledTerm<T>>),
}

impl<T> BinaryDirectedAcyclicGraph<T> for Rc<LabeledTerm<T>> {
    fn children(&self) -> Option<(&Self, &Self)> {
        match self.as_ref() {
            LabeledTerm::Variable(_value) => None,
            LabeledTerm::Operation(left, right) => Some((left, right)),
        }
    }

    fn from_children(left: Self, right: Self) -> Self {
        Rc::new(LabeledTerm::Operation(left, right))
    }

    fn from_leaf(value: T) -> Self {
        Rc::new(LabeledTerm::Variable(value))
    }
}

pub struct TermIndexing(HashMap<(usize, usize), usize>);

impl From<&TermRef> for TermIndexing {
    fn from(value: &TermRef) -> Self {
        let mut table = HashMap::new();

        value.reduce(
            &mut |_node, left_label, right_label| {
                if let Some(label) = table.get(&(left_label, right_label)) {
                    *label
                } else {
                    table.insert((left_label, right_label), table.len() + 1);
                    table.len()
                }
            },
            &mut |_| 0,
        );
        TermIndexing(table)
    }
}

pub struct IndexedTerm {
    term: TermRef,
    index: TermIndexing,
}

impl From<TermRef> for IndexedTerm {
    fn from(value: TermRef) -> Self {
        Self {
            index: TermIndexing::from(&value),
            term: value,
        }
    }
}

impl IndexedTerm {
    // there is room for optimization here: Use BTreeSet instead of HashSet and
    // use max/min values to abort loop over `index' early. also `index' could
    // be stored in Vec instead (we only lookup in term index creation).
    pub fn matches(&mut self, term: &TermRef) -> Vec<TermByAddress> {
        let mut matched = Vec::new();

        term.reduce(
            &mut |node, left_labels, right_labels| -> HashSet<usize> {
                let mut labels = HashSet::<usize>::from([0]);
                for ((left_label, right_label), label) in &self.index.0 {
                    if left_labels.contains(left_label) && right_labels.contains(right_label) {
                        if *label == self.index.0.len() {
                            matched.push(TermByAddress::from(node.clone()));
                        }
                        labels.insert(*label);
                    }
                }
                labels
            },
            &mut |_| [0].into(),
        );

        matched
    }
}
