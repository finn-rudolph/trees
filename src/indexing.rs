use crate::bidag::BinaryChildren;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};

use crate::term::TermRef;

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

impl IndexedTerm {
    pub fn term(&self) -> &TermRef {
        &self.term
    }
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
    pub fn matches(&self, term: &TermRef) -> Vec<TermRef> {
        let mut matched = Vec::new();

        term.reduce(
            &mut |node, left_labels, right_labels| -> HashSet<usize> {
                let mut labels = HashSet::<usize>::from([0]);
                for ((left_label, right_label), label) in &self.index.0 {
                    if left_labels.contains(left_label) && right_labels.contains(right_label) {
                        if *label == self.index.0.len() {
                            matched.push(node.clone());
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

impl Debug for IndexedTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IndexedTerm[{}]", self.term)
    }
}
