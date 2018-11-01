use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cargo::util::CargoResult;
use handlebars::{Handlebars, HelperDef};
use serde_derive::Serialize;

use crate::engine::OutputMode;
use crate::graph::BuildGraph;

mod utils;
use self::utils::container_tools_version;

mod helpers;
use self::helpers::*;

const DEFAULT_TOOLS_IMAGE: &str = "denzp/cargo-container-tools";
const DEFAULT_TOOLS_STAGE: &str = "container-tools";

#[derive(Default, Serialize)]
struct DummyContext;

pub struct DockerfilePrinter {
    mode: OutputMode,
    handlebars: Handlebars,
    template_path: PathBuf,
}

impl DockerfilePrinter {
    pub fn new<P>(mode: OutputMode, template_path: P) -> Self
    where
        P: AsRef<Path>,
    {
        DockerfilePrinter {
            mode,
            handlebars: Handlebars::new(),
            template_path: template_path.as_ref().into(),
        }
    }

    pub fn write(mut self, graph: BuildGraph, writer: &mut Write) -> CargoResult<()> {
        let graph = Arc::new(graph);

        self.handlebars.register_helper(
            "generate_build_stages_with",
            Box::new(DockerfileHelper(BuildStagesHelper::new(graph.clone()))),
        );

        let (binaries_helper, tests_helper): (Box<HelperDef>, Box<HelperDef>) = match self.mode {
            OutputMode::All => (
                Box::new(DockerfileHelper(BinariesHelper::new(graph.clone()))),
                Box::new(DockerfileHelper(TestsHelper::new(graph))),
            ),

            OutputMode::Binaries => (
                Box::new(DockerfileHelper(BinariesHelper::new(graph.clone()))),
                Box::new(DockerfileHelper(DummyHelper)),
            ),

            OutputMode::Tests => (
                Box::new(DockerfileHelper(DummyHelper)),
                Box::new(DockerfileHelper(TestsHelper::new(graph))),
            ),
        };

        self.handlebars.register_helper("binaries", binaries_helper);
        self.handlebars.register_helper("tests", tests_helper);

        self.write_header(writer)?;

        self.handlebars
            .register_template_file("Dockerfile", &self.template_path)?;

        self.handlebars
            .render_to_write("Dockerfile", &DummyContext::default(), writer)?;

        Ok(())
    }

    fn write_header(&mut self, writer: &mut Write) -> CargoResult<()> {
        writeln!(writer, "# syntax = tonistiigi/dockerfile:runmount20180618")?;

        writeln!(
            writer,
            "FROM {}:{} as {}",
            DEFAULT_TOOLS_IMAGE,
            container_tools_version()?,
            DEFAULT_TOOLS_STAGE
        )?;

        writeln!(writer, "")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use maplit::btreemap;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::config::Config;
    use crate::graph::BuildGraph;
    use crate::plan::{Invocation, TargetKind};

    #[test]
    fn it_should_generate_dockerfile_for_all_targets() -> CargoResult<()> {
        let config = Config::from_workspace_root("../examples/workspace")?;
        let invocations = default_invocations(&config);
        let graph = BuildGraph::from_invocations(&invocations, &config)?;

        let mut output = Vec::new();

        DockerfilePrinter::new(OutputMode::All, "../examples/workspace/Dockerfile.hbs")
            .write(graph, &mut output)?;

        let output_contents = String::from_utf8_lossy(&output);

        assert_eq!(
            output_contents.lines().collect::<Vec<_>>(),
            include_str!("../../../../tests/simple.all.dockerfile")
                .lines()
                .collect::<Vec<_>>()
        );

        Ok(())
    }

    #[test]
    fn it_should_generate_dockerfile_for_tests() -> CargoResult<()> {
        let config = Config::from_workspace_root("../examples/workspace")?;
        let invocations = default_invocations(&config);
        let graph = BuildGraph::from_invocations(&invocations, &config)?;

        let mut output = Vec::new();

        DockerfilePrinter::new(OutputMode::Tests, "../examples/workspace/Dockerfile.hbs")
            .write(graph, &mut output)?;

        let output_contents = String::from_utf8_lossy(&output);

        assert_eq!(
            output_contents.lines().collect::<Vec<_>>(),
            include_str!("../../../../tests/simple.tests.dockerfile")
                .lines()
                .collect::<Vec<_>>()
        );

        Ok(())
    }

    #[test]
    fn it_should_generate_dockerfile_for_binaries() -> CargoResult<()> {
        let config = Config::from_workspace_root("../examples/workspace")?;
        let invocations = default_invocations(&config);
        let graph = BuildGraph::from_invocations(&invocations, &config)?;

        let mut output = Vec::new();

        DockerfilePrinter::new(OutputMode::Binaries, "../examples/workspace/Dockerfile.hbs")
            .write(graph, &mut output)?;

        let output_contents = String::from_utf8_lossy(&output);

        assert_eq!(
            output_contents.lines().collect::<Vec<_>>(),
            include_str!("../../../../tests/simple.binaries.dockerfile")
                .lines()
                .collect::<Vec<_>>()
        );

        Ok(())
    }

    fn default_invocations(config: &Config) -> Vec<Invocation> {
        vec![
            Invocation {
                package_name: "bitflags".into(),
                package_version: "1.0.4".parse().unwrap(),

                program: String::from("rustc"),
                args: vec![String::from("--crate-name"), String::from("bitflags")],
                env: btreemap!{ "CARGO_MANIFEST_DIR".into() => "any".into() },

                outputs: vec![config.get_local_outdir().join("debug/deps/bitflags.rlib")],

                ..Default::default()
            },
            Invocation {
                package_name: "log".into(),
                package_version: "0.4.5".parse().unwrap(),

                program: String::from("rustc"),
                args: vec![String::from("--crate-name"), String::from("log")],
                env: btreemap!{ "ANY_ENV".into() => "'quotes\" and multiple \nlines".into() },

                deps: vec![0],
                outputs: vec![config.get_local_outdir().join("debug/deps/log.rlib")],
                links: btreemap!{
                    config.get_local_outdir().join("log.rlib") => config.get_local_outdir().join("debug/deps/log.rlib")
                },

                ..Default::default()
            },
            Invocation {
                package_name: "binary-1".into(),
                package_version: "0.1.0".parse().unwrap(),

                target_kind: vec![TargetKind::Bin],

                program: String::from("rustc"),
                args: vec![String::from("--crate-name"), String::from("binary-1")],

                deps: vec![1],
                outputs: vec![config.get_local_outdir().join("debug/deps/binary-1-hash")],
                links: btreemap!{
                    config.get_local_outdir().join("debug/binary-1") => config.get_local_outdir().join("debug/deps/binary-1-hash")
                },

                ..Default::default()
            },
            Invocation {
                package_name: "binary-2".into(),
                package_version: "0.1.0".parse().unwrap(),

                target_kind: vec![TargetKind::Bin],

                program: String::from("rustc"),
                args: vec![String::from("--crate-name"), String::from("binary-2")],

                deps: vec![0],
                outputs: vec![config.get_local_outdir().join("debug/deps/binary-2-hash")],
                links: btreemap!{
                    config.get_local_outdir().join("debug/binary-2") => config.get_local_outdir().join("debug/deps/binary-2-hash")
                },

                ..Default::default()
            },
            Invocation {
                package_name: "binary-1".into(),
                package_version: "0.1.0".parse().unwrap(),

                target_kind: vec![TargetKind::Bin],

                program: String::from("rustc"),
                args: vec![
                    String::from("--crate-name"),
                    String::from("binary-1"),
                    String::from("--test"),
                ],

                deps: vec![1],
                outputs: vec![{
                    config
                        .get_local_outdir()
                        .join("debug/deps/binary-1-test-hash")
                }],
                links: btreemap!{
                    config.get_local_outdir().join("debug/binary-1-test-hash") => config.get_local_outdir().join("debug/deps/binary-1-test-hash")
                },

                ..Default::default()
            },
            Invocation {
                package_name: "binary-2".into(),
                package_version: "0.1.0".parse().unwrap(),

                target_kind: vec![TargetKind::Bin],

                program: String::from("rustc"),
                args: vec![
                    String::from("--crate-name"),
                    String::from("binary-2"),
                    String::from("--test"),
                ],

                deps: vec![0],
                outputs: vec![{
                    config
                        .get_local_outdir()
                        .join("debug/deps/binary-2-test-hash")
                }],
                links: btreemap!{
                    config.get_local_outdir().join("debug/binary-2-test-hash") => config.get_local_outdir().join("debug/deps/binary-2-test-hash")
                },

                ..Default::default()
            },
        ]
    }
}
