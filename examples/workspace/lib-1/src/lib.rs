#[cfg(lib_msg)]
pub fn hello() {
    println!(env!("LIB_MSG"));
}

#[test]
fn faulty_test() {
    assert!(false, "this should fail");
}
