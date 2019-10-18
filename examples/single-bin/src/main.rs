use std::collections::BTreeMap;
use std::env::{args, vars};

fn main() {
    println!("Hello from the container!");
    println!("Args: {:?}", args().collect::<Vec<_>>());
    println!("Env: {:#?}", vars().collect::<BTreeMap<_, _>>());
}
