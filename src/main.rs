#![feature(stmt_expr_attributes)]

mod bidag;
mod byaddr;
mod indexing;
mod iter;
mod labeled;
mod maps;
mod term;

use std::rc::Rc;

use clap::Parser;

use crate::{
    byaddr::TermByAddress, indexing::IndexedTerm, iter::TermIterator, labeled::LabeledTerm,
};

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
    let (left_tree, right_tree) = (
        LabeledTerm::<String>::parse(left),
        LabeledTerm::<String>::parse(right),
    );

    let equiv = left_tree.map_to(right_tree).upgrade();

    println!("equiv: {:?}", equiv);

    let pattern = IndexedTerm::from(Rc::new(equiv.source().as_ref().clone()));

    for term in TermIterator::new(args.leaves) {
        println!("{}", term);
        let matches = pattern.matches(&term);
        for matched in matches {
            println!(
                "sub: {:?}",
                term.substitute(TermByAddress::from(matched.as_ref()), &equiv)
            );
        }
    }
}
