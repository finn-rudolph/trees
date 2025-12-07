#![feature(stmt_expr_attributes)]

mod bidag;
mod byaddr;
mod eqclass;
mod indexing;
mod iter;
mod labeled;
mod maps;
mod perm;
mod term;

use std::rc::Rc;

use clap::Parser;

use crate::{
    byaddr::TermByAddress, eqclass::EquivalenceClasses, indexing::IndexedTerm, iter::TermIterator,
    labeled::LabeledTerm,
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

    let equiv = left_tree.map_to(right_tree);

    println!("equiv: {:?}", equiv);

    let pattern = IndexedTerm::from(Rc::new(equiv.source().as_ref().clone()));

    let mut eqclasses = EquivalenceClasses::new();

    for term in TermIterator::new(args.leaves) {
        println!("Considering term: {}", term);
        let matches = pattern.matches(&term);
        for matched in matches {
            let result_equiv = term.substitute(TermByAddress::from(matched.as_ref()), &equiv);
            println!(" - equivalence: {:?}", result_equiv);
            eqclasses.add_equiv(result_equiv);
        }
    }

    println!("{:#?}", eqclasses);
}
