use std::fs::metadata;

fn main() {
    // Check if build dependencies were installed. Used for the integration testing.

    metadata("/usr/bin/protoc")
        .expect("Unable to find `/usr/bin/protoc`. Custom builder setup failed!");

    metadata("/tmp/custom-output")
        .expect("Unable to find `/tmp/custom-output`. Custom builder setup failed!");
}
