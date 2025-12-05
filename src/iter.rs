use std::rc::Rc;

use crate::term::{Term, TermRef};

pub enum TermIterator {
    InnerIterator(bool, Box<Self>, Box<Self>, usize, usize, TermRef),
    LeafIterator(bool),
}

impl TermIterator {
    pub fn new(leaves: usize) -> Self {
        if leaves == 1 {
            Self::LeafIterator(false)
        } else {
            let mut right = TermIterator::new(leaves - 1);
            let right_subtree = right.next().unwrap();
            Self::InnerIterator(
                false,
                Box::new(TermIterator::LeafIterator(false)),
                Box::new(right),
                1,
                leaves - 1,
                right_subtree,
            )
        }
    }
}

impl Iterator for TermIterator {
    type Item = TermRef;

    fn next(&mut self) -> Option<TermRef> {
        match self {
            Self::LeafIterator(done) => {
                if !*done {
                    *done = true;
                    Some(Rc::new(Term::Variable))
                } else {
                    None
                }
            }
            Self::InnerIterator(done, left, right, left_leaves, right_leaves, right_subtree) => {
                if let Some(left_subtree) = left.next() {
                    return Some(Rc::new(Term::Operation(
                        left_subtree,
                        right_subtree.clone(),
                    )));
                };

                if let Some(subtree) = right.next() {
                    *right_subtree = subtree;
                    *left = Box::new(TermIterator::new(*left_leaves));
                    return self.next();
                };

                *left_leaves += 1;
                *right_leaves -= 1;

                if *right_leaves == 0 {
                    *done = true;
                    return None;
                }

                *left = Box::new(TermIterator::new(*left_leaves));
                *right = Box::new(TermIterator::new(*right_leaves));
                *right_subtree = right.next().unwrap();
                self.next()
            }
        }
    }
}
