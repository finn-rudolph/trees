use std::{marker::PhantomData, rc::Rc};

use crate::tree::DAG;

pub struct TreeIterator<T: Clone, R: Into<T>, I: Iterator<Item = R> + Clone> {
    structure_iter: TreeStructureIterator,
    label_iter: I,
    phantom: PhantomData<T>,
}

impl<T: Clone, R: Into<T>, I: Iterator<Item = R> + Clone> TreeIterator<T, R, I> {
    pub fn new(labels: I, leave_count: usize) -> Self {
        Self {
            label_iter: labels,
            structure_iter: TreeStructureIterator::new(leave_count),
            phantom: PhantomData,
        }
    }
}

impl<T: Clone, R: Into<T>, I: Iterator<Item = R> + Clone> Iterator for TreeIterator<T, R, I> {
    type Item = Rc<DAG<T>>;
    fn next(&mut self) -> Option<Self::Item> {
        let skeleton = Rc::new(self.structure_iter.next()?);
        let mut label_iter = self.label_iter.clone();

        Some(skeleton.map(&mut |_, _| label_iter.next().unwrap().into()))
    }
}

pub enum TreeStructureIterator {
    InnerIterator(bool, Box<Self>, Box<Self>, usize, usize, Rc<DAG<()>>),
    LeafIterator(bool),
}

impl TreeStructureIterator {
    pub fn new(leaves: usize) -> Self {
        if leaves == 1 {
            Self::LeafIterator(false)
        } else {
            let mut right = TreeStructureIterator::new(leaves - 1);
            let right_subtree = right.next().unwrap();
            Self::InnerIterator(
                false,
                Box::new(TreeStructureIterator::LeafIterator(false)),
                Box::new(right),
                1,
                leaves - 1,
                Rc::new(right_subtree),
            )
        }
    }
}

impl Iterator for TreeStructureIterator {
    type Item = DAG<()>;

    fn next(&mut self) -> Option<DAG<()>> {
        match self {
            Self::LeafIterator(done) => {
                if !*done {
                    *done = true;
                    Some(DAG::Leaf(()))
                } else {
                    None
                }
            }
            Self::InnerIterator(done, left, right, left_leaves, right_leaves, right_subtree) => {
                if let Some(left_subtree) = left.next() {
                    return Some(DAG::Branch(Rc::new(left_subtree), right_subtree.clone()));
                };

                if let Some(subtree) = right.next() {
                    *right_subtree = Rc::new(subtree);
                    *left = Box::new(TreeStructureIterator::new(*left_leaves));
                    return self.next();
                };

                *left_leaves += 1;
                *right_leaves -= 1;

                if *right_leaves == 0 {
                    *done = true;
                    return None;
                }

                *left = Box::new(TreeStructureIterator::new(*left_leaves));
                *right = Box::new(TreeStructureIterator::new(*right_leaves));
                *right_subtree = Rc::new(right.next().unwrap());
                self.next()
            }
        }
    }
}
