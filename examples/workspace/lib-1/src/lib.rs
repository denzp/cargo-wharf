#[cfg(lib_msg)]
pub fn hello() {
    println!(env!("LIB_MSG"));
}
