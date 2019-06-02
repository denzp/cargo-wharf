use std::io::stdout;

use buildkit_llb::prelude::*;

fn main() {
    let builder_image =
        Source::image("library/alpine:latest").custom_name("Using alpine:latest as a builder");

    let command = {
        Command::run("/bin/sh")
            .args(&["-c", "echo 'test string 5' > /out/file0"])
            .custom_name("create a dummy file")
            .mount(Mount::ReadOnlyLayer(builder_image.output(), "/"))
            .mount(Mount::Scratch(OutputIndex(0), "/out"))
    };

    let file1 = {
        FileSystem::copy(command.output(0), "/file0", command.output(0), "/file1")
            .custom_name("copy the dummy file to other location (1)")
    };

    let file2 = {
        FileSystem::copy(command.output(0), "/file0", file1.output(), "/file2")
            .custom_name("copy the dummy file to other location (2)")
    };

    Terminal::with(file2.output())
        .write_definition(stdout())
        .unwrap()
}
