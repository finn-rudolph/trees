use std::{
    fmt::{Debug, Display},
    rc::Rc,
};

use crate::{
    bidag::{BinaryChildren, FromChildren},
    byaddr::TermByAddress,
    labeled::LabeledTermRef,
    maps::{NodeIndex, TermMap},
    perm::perms::PermIndex,
};

#[derive(Eq, PartialEq, Hash, Clone)]
pub enum Term {
    Variable,
    Operation(TermRef, TermRef),
}

pub type TermRef = Rc<Term>;

impl Term {
    pub fn label<T, I: Iterator<Item = T>>(
        self: &TermRef,
        mut iter: I,
    ) -> Option<LabeledTermRef<T>> {
        self.try_map(&mut |_| iter.next())
    }

    pub fn label_with<T, F: FnMut(usize) -> T>(
        self: &TermRef,
        mut labeler: F,
    ) -> LabeledTermRef<T> {
        let mut count = 0;
        self.map(
            &mut #[inline(always)]
            |_leaf| {
                let label = labeler(count);
                count += 1;
                label
            },
        )
    }

    pub fn counted_clone(&self) -> (TermRef, NodeIndex) {
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
        backward_map: &TermMap<'_>,
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
                if &TermByAddress::from(self.as_ref()) == match_root {
                    let offset_leaf_index = *leaf_index;
                    backward_map
                        .source()
                        .counted_replace_leaves(&mut |_, target_leaf_index| {
                            let translated_index = backward_map[target_leaf_index];
                            let (replacement, start, end) =
                                &replacements[translated_index as usize];
                            computed_map
                                .extend((start + offset_leaf_index)..(end + offset_leaf_index));
                            *leaf_index += end - start;
                            replacement.clone()
                        })
                } else {
                    let left_result = left.insert_replacements_helper(
                        match_root,
                        replacements,
                        backward_map,
                        leaf_index,
                        computed_map,
                    );
                    let right_result = right.insert_replacements_helper(
                        match_root,
                        replacements,
                        backward_map,
                        leaf_index,
                        computed_map,
                    );

                    Rc::new(Term::Operation(left_result, right_result))
                }
            }
        }
    }

    pub fn identity_map(self: &TermRef) -> TermMap<'static> {
        let (_, leaf_count) = self.counted_clone();
        TermMap::new(
            self.clone(),
            self.clone(),
            (0..leaf_count as PermIndex).collect::<Vec<_>>().into(),
        )
    }

    pub fn substitute(
        self: &TermRef,
        match_root: TermByAddress,
        map: &TermMap<'_>,
    ) -> TermMap<'static> {
        // replacements[i] = (replacement, a, b) such that replacment is a copy of the tree at
        // the i-th leaf of the embedded source. The origial tree has the leaves [a, b) in `match_root`.
        let mut replacements = Vec::new();
        let mut replacement_leaf_index = 0;

        map.source().propagate(
            match_root.as_ref(),
            &mut |_, embedded_node| {
                embedded_node
                    .children()
                    .expect("match_root not embedded here")
            },
            &mut |_, embedded_node| {
                let (replacement, replace_size) = embedded_node.counted_clone();
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
            &map.backward(),
            &mut result_leaf_index,
            &mut computed_map,
        );

        let result_map_backward = TermMap::new(result, self.clone(), computed_map.into());
        result_map_backward.into_backward()
    }
}

impl BinaryChildren for Term {
    fn children(&self) -> Option<(&Self, &Self)> {
        match self {
            Term::Variable => None,
            Term::Operation(left, right) => Some((left, right)),
        }
    }
}

impl BinaryChildren for Rc<Term> {
    fn children(&self) -> Option<(&Self, &Self)> {
        match self.as_ref() {
            Term::Variable => None,
            Term::Operation(left, right) => Some((left, right)),
        }
    }
}

impl FromChildren<()> for TermRef {
    fn from_children(left: Self, right: Self) -> Self {
        Rc::new(Term::Operation(left, right))
    }

    fn from_leaf(_value: ()) -> Self {
        Rc::new(Term::Variable)
    }
}

impl Debug for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut leaf_count = 0;
        write!(f, "Term[")?;
        self.display_helper(
            f,
            &mut |_, f| write!(f, "("),
            &mut |_, f| write!(f, ")"),
            &mut |_, f| write!(f, " * "),
            &mut |_, f| {
                leaf_count += 1;
                write!(f, "{}", leaf_count - 1)
            },
        )?;
        write!(f, "]")
    }
}

impl Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut leaf_count = 0;
        self.display_helper(
            f,
            &mut |_, f| write!(f, "("),
            &mut |_, f| write!(f, ")"),
            &mut |_, f| write!(f, " * "),
            &mut |_, f| {
                leaf_count += 1;
                write!(f, "{}", leaf_count - 1)
            },
        )
    }
}
