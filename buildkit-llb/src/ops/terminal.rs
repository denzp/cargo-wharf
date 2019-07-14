use std::io::{self, Write};

use buildkit_proto::pb::{self, Input};
use prost::Message;

use crate::serialization::{Context, Node, Output, Result};
use crate::utils::OperationOutput;

/// Final operation in the graph. Responsible for printing the complete LLB definition.
#[derive(Debug)]
pub struct Terminal<'a> {
    input: OperationOutput<'a>,
}

impl<'a> Terminal<'a> {
    pub fn with(input: OperationOutput<'a>) -> Self {
        Self { input }
    }

    pub fn into_definition(self) -> pb::Definition {
        let mut ctx = Context::default();

        let (def, metadata) = {
            self.serialize(&mut ctx)
                .unwrap()
                .into_iter()
                .map(|item| (item.bytes, (item.digest, item.metadata)))
                .unzip()
        };

        pb::Definition { def, metadata }
    }

    pub fn write_definition(self, mut writer: impl Write) -> io::Result<()> {
        let mut bytes = Vec::new();
        self.into_definition().encode(&mut bytes).unwrap();

        writer.write_all(&bytes)
    }

    fn serialize(&self, cx: &mut Context) -> Result<Output> {
        let serialized_input = self.input.operation().serialize(cx)?;

        let head = pb::Op {
            inputs: vec![Input {
                digest: serialized_input.head.digest.clone(),
                index: self.input.output().into(),
            }],

            ..Default::default()
        };

        Ok(Output {
            head: Node::new(head, Default::default()),
            tail: serialized_input.into_iter().collect(),
        })
    }
}

#[test]
fn serialization() {
    use crate::prelude::*;

    let context = Source::local("context");
    let builder_image = Source::image("rustlang/rust:nightly");
    let final_image = Source::image("library/alpine:latest");

    let first_command = Command::run("rustc")
        .args(&["--crate-name", "crate-1"])
        .mount(Mount::ReadOnlyLayer(builder_image.output(), "/"))
        .mount(Mount::ReadOnlyLayer(context.output(), "/context"))
        .mount(Mount::Scratch(OutputIdx(0), "/target"));

    let second_command = Command::run("rustc")
        .args(&["--crate-name", "crate-2"])
        .mount(Mount::ReadOnlyLayer(builder_image.output(), "/"))
        .mount(Mount::ReadOnlyLayer(context.output(), "/context"))
        .mount(Mount::Scratch(OutputIdx(0), "/target"));

    let assembly_op = FileSystem::sequence()
        .append(FileSystem::mkdir(
            OutputIdx(0),
            LayerPath::Other(final_image.output(), "/output"),
        ))
        .append(
            FileSystem::copy()
                .from(LayerPath::Other(first_command.output(0), "/target/crate-1"))
                .to(
                    OutputIdx(1),
                    LayerPath::Own(OwnOutputIdx(0), "/output/crate-1"),
                ),
        )
        .append(
            FileSystem::copy()
                .from(LayerPath::Other(
                    second_command.output(0),
                    "/target/crate-2",
                ))
                .to(
                    OutputIdx(2),
                    LayerPath::Own(OwnOutputIdx(1), "/output/crate-2"),
                ),
        );

    crate::check_op!(
        Terminal::with(assembly_op.output(0)),
        |digest| { "sha256:d13a773a61236be3c7d539f3ef6d583095c32d2a2a60deda86e71705f2dbc99b" },
        |description| { vec![] },
        |caps| { vec![] },
        |tail| {
            vec![
                "sha256:0e6b31ceed3e6dc542018f35a53a0e857e6a188453d32a2a5bbe7aa2971c1220",
                "sha256:dee2a3d7dd482dd8098ba543ff1dcb01efd29fcd16fdb0979ef556f38564543a",
                "sha256:a60212791641cbeaa3a49de4f7dff9e40ae50ec19d1be9607232037c1db16702",
                "sha256:782f343f8f4ee33e4f342ed4209ad1a9eb4582485e45251595a5211ebf2b3cbf",
                "sha256:dee2a3d7dd482dd8098ba543ff1dcb01efd29fcd16fdb0979ef556f38564543a",
                "sha256:a60212791641cbeaa3a49de4f7dff9e40ae50ec19d1be9607232037c1db16702",
                "sha256:3418ad515958b5e68fd45c9d6fbc8d2ce7d567a956150d22ff529a3fea401aa2",
                "sha256:13bb644e4ec0cabe836392649a04551686e69613b1ea9c89a1a8f3bc86181791",
            ]
        },
        |inputs| {
            vec![(
                "sha256:13bb644e4ec0cabe836392649a04551686e69613b1ea9c89a1a8f3bc86181791",
                0,
            )]
        },
    );
}
