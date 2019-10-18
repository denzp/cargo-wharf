fn main() {
    println!("cargo:rustc-env=LIB_MSG=Hello from build script");
    println!("cargo:rustc-cfg=lib_msg");
}
