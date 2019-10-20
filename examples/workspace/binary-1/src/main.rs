fn main() {
    lib_1::hello();

    println!("Hello from binary 1");

    #[cfg(feature = "the-special-feature")]
    println!("the-special-feature is on");
}
