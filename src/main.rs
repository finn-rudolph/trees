mod tree;

use clap::Parser;

use crate::tree::DAG;

#[derive(Parser)]
struct Args {
    #[arg(short, long, help = "tree")]
    tree: String,
}

fn main() {
    let args = Args::parse();

    let root = DAG::<String>::parse(&args.tree);
    println!("{}", root);
}
