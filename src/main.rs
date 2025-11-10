#![feature(coroutines)]
#![feature(coroutine_trait)]
#![feature(stmt_expr_attributes)]

mod tree;

use crate::tree::DAG;
use clap::Parser;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Parser)]
struct Args {
    #[arg(short, long, help = "equivalence")]
    equivalence: String,

    #[arg(
        short,
        long,
        help = "maximum number of leaves of expressions that are tried"
    )]
    leaves: usize,
}

struct TreeIterator<T: Clone + Default> {
    top_level: bool,
    left_leaves: usize,
    leaves: usize,
    finished: bool,
    left: Option<Box<TreeIterator<T>>>,
    right: Option<Box<TreeIterator<T>>>,
    right_subtree: Option<Rc<DAG<T>>>,
}

impl<T: Default + Clone> TreeIterator<T> {
    pub fn new(leaf_count: usize) -> Self {
        let mut iter = Self::new_inner(leaf_count);
        iter.top_level = true;
        iter
    }

    fn new_inner(leaf_count: usize) -> Self {
        if leaf_count == 1 {
            TreeIterator {
                top_level: false,
                left_leaves: 0,
                leaves: 1,
                finished: false,
                left: None,
                right: None,
                right_subtree: None,
            }
        } else {
            let mut right = Box::new(TreeIterator::new_inner(leaf_count - 1));
            TreeIterator {
                top_level: false,
                left_leaves: 1,
                leaves: leaf_count,
                finished: false,
                right_subtree: Some(Rc::new(right.next().unwrap())),
                left: Some(Box::new(TreeIterator::new_inner(1))),
                right: Some(right),
            }
        }
    }

    fn next_inner(&mut self) -> Option<DAG<T>> {
        if self.finished {
            return None;
        }

        if self.leaves == 1 {
            self.finished = true;
            return Some(DAG::Leaf(T::default()));
        }

        if let Some(left_subtree) = self.left.as_mut().unwrap().next() {
            return Some(DAG::<T>::Branch(
                Rc::new(left_subtree),
                self.right_subtree.as_ref().unwrap().clone(),
            ));
        };

        if let Some(right_subtree) = self.right.as_mut().unwrap().next() {
            self.right_subtree = Some(Rc::new(right_subtree));
            self.left = Some(Box::new(TreeIterator::new_inner(self.left_leaves)));
            return self.next_inner();
        };

        self.left_leaves += 1;
        if self.left_leaves == self.leaves {
            self.finished = true;
            return None;
        }

        self.left = Some(Box::new(TreeIterator::new_inner(self.left_leaves)));
        self.right = Some(Box::new(TreeIterator::new_inner(
            self.leaves - self.left_leaves,
        )));
        self.right_subtree = Some(Rc::new(self.right.as_mut().unwrap().next_inner().unwrap()));
        self.next_inner()
    }
}

impl<T: Default + Clone> Iterator for TreeIterator<T> {
    type Item = DAG<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.top_level {
            self.next_inner().clone()
        } else {
            self.next_inner()
        }
    }
}

/*
fn gen_tree(tree :Vec<u8>, leaves, callback) {
    if leaves == n {
        callback(tree)
    }
    place leaf node, tree += [1], return;
    place branch node, tree += [0], gen_tree, gen_tree;
}
*/
fn main() {
    let args = Args::parse();

    let (left, right) = args.equivalence.split_once("=").unwrap();
    let (left_tree, right_tree) = (DAG::<String>::parse(left), DAG::<String>::parse(right));
    for tree in TreeIterator::<u64>::new(15) {
        println!("{:?}", tree)
    }
}
