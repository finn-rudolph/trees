#![feature(coroutines)]
#![feature(coroutine_trait)]
#![feature(stmt_expr_attributes)]

mod iter;
mod tree;

use crate::{
    iter::TreeIterator,
    tree::{DAG, TreeEquivalence},
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

    let equivalence = TreeEquivalence::from_labels(left_tree, right_tree);

    for tree in TreeIterator::<String, _, _>::new('a'..='z', args.leaves) {
        println!("tree: {}", tree);
        let matched = equivalence.left_to_right.all_matches(&tree);
        for the_match in matched {
            let (substituted, _new_equivalence) =
                tree.substitue(&the_match, &equivalence.left_to_right);

            println!("{}", substituted);
        }
    }
}
