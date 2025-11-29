#![feature(coroutines)]
#![feature(coroutine_trait)]
#![feature(stmt_expr_attributes)]

mod eqclass;
mod iter;
mod maps;
mod transform;
mod tree;

use std::rc::Rc;

use crate::{
    eqclass::EquivalenceClasses,
    iter::{TreeIterator, TreeStructureIterator},
    maps::TreeEquivalence,
    tree::DAG,
};
use clap::Parser;

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

fn main() {
    let args = Args::parse();

    let (left, right) = args.equivalence.split_once("=").unwrap();
    let (left_tree, right_tree) = (DAG::<String>::parse(left), DAG::<String>::parse(right));

    let mut classes = EquivalenceClasses::<()>::new();

    let equivalence = TreeEquivalence::from_labels(left_tree, right_tree).remove_labels();

    classes.add_equivalence(equivalence.clone());

    for tree in TreeStructureIterator::new(args.leaves) {
        let tree = Rc::new(tree.clone());
        println!("tree: {:?}", tree);
        let transform = equivalence.right_to_left();

        let matched = transform.matches(&tree);
        for the_match in matched {
            let (substituted, new_equivalence) = transform.apply(&tree, &the_match);
            println!("{:#?}", substituted);
            println!("{:#?}", new_equivalence);
            classes.add_equivalence(new_equivalence);
            // println!("{}", new_equivalence.right_to_left);
        }
    }

    println!("{:?}", classes);
}
