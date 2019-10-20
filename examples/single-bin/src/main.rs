use std::collections::BTreeMap;
use std::env::{args, vars};

fn main() {
    println!("Hello from the container!");
    println!("Args: {:?}", args().collect::<Vec<_>>());
    println!("Env: {:#?}", vars().collect::<BTreeMap<_, _>>());

    #[cfg(feature = "feature-1")]
    println!("feature-1 is on");

    #[cfg(feature = "feature-2")]
    println!("feature-2 is on");
}
