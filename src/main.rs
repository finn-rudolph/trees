#![feature(stmt_expr_attributes)]

mod bidag;
mod iter;
mod maps;
mod term;

use clap::Parser;

use crate::iter::TermIterator;

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
    // for term in TermIterator::new(args.leaves) {
    //     println!("{}", term);
    // }
}
