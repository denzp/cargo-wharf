use std::io::Write;
use std::path::{Path, PathBuf};

use cargo::core::GitReference;
use cargo::util::CargoResult;
use handlebars::{Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext};
use petgraph::graph::NodeIndex;
use serde_derive::Serialize;

use crate::engine::OutputMode;
use crate::graph::{BuildGraph, Node, NodeKind, SourceKind};

use super::utils::{container_tools_version, find_unique_base_paths};

const DEFAULT_TOOLS_IMAGE: &str = "denzp/cargo-container-tools";
const DEFAULT_TOOLS_STAGE: &str = "container-tools";

pub struct DockerfilePrinter<'a, W: Write> {
    handlebars: Handlebars,
    mode: OutputMode,
    graph: &'a BuildGraph,
    writer: W,
}

#[derive(Default, Serialize)]
struct DummyContext;

struct GenerateBuildStagesHelper;
struct BinariesHelper;
struct TestsHelper;

impl HelperDef for GenerateBuildStagesHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        rc: &mut RenderContext,
        out: &mut Output,
    ) -> HelperResult {
        out.write("TODO")?;
        Ok(())
    }
}

impl HelperDef for BinariesHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        rc: &mut RenderContext,
        out: &mut Output,
    ) -> HelperResult {
        out.write("TODO")?;
        Ok(())
    }
}

impl HelperDef for TestsHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        rc: &mut RenderContext,
        out: &mut Output,
    ) -> HelperResult {
        out.write("TODO")?;
        Ok(())
    }
}

impl<'a, W: Write> DockerfilePrinter<'a, W> {
    pub fn new(mode: OutputMode, graph: &'a BuildGraph, writer: W) -> Self {
        let mut handlebars = Handlebars::new();

        handlebars.register_helper(
            "generate_build_stages_with",
            Box::new(GenerateBuildStagesHelper),
        );

        handlebars.register_helper("binaries", Box::new(BinariesHelper));
        handlebars.register_helper("tests", Box::new(TestsHelper));

        DockerfilePrinter {
            handlebars,
            mode,
            graph,
            writer,
        }
    }

    pub fn write<P: AsRef<Path>>(mut self, template_path: P) -> CargoResult<()> {
        writeln!(
            self.writer,
            "# syntax = tonistiigi/dockerfile:runmount20180618"
        )?;
        writeln!(
            self.writer,
            "FROM {}:{} as {}",
            DEFAULT_TOOLS_IMAGE,
            container_tools_version()?,
            DEFAULT_TOOLS_STAGE
        )?;
        writeln!(self.writer, "")?;

        self.handlebars
            .register_template_file("Dockerfile", template_path)?;

        self.handlebars
            .render_to_write("Dockerfile", &DummyContext::default(), self.writer)?;

        // writeln!(self.writer, "FROM rustlang/rust:nightly as builder")?;

        // for (index, node) in self.graph.nodes() {
        //     self.write_stage_header(index)?;
        //     self.write_stage_sources(node)?;
        //     self.write_stage_deps_copy(index)?;
        //     self.write_stage_env_vars(node)?;
        //     self.write_stage_tree_creation(node)?;
        //     self.write_stage_command(node)?;
        //     self.write_stage_links(node)?;
        // }

        // match self.mode {
        //     OutputMode::All => {
        //         self.write_binaries_stage()?;
        //         self.write_tests_stage()?;
        //     }

        //     OutputMode::Binaries => {
        //         self.write_binaries_stage()?;
        //     }

        //     OutputMode::Tests => {
        //         self.write_tests_stage()?;
        //     }
        // }

        Ok(())
    }

    fn write_stage_header(&mut self, index: NodeIndex<u32>) -> CargoResult<()> {
        writeln!(self.writer, "")?;
        writeln!(
            self.writer,
            "FROM builder as builder-node-{}",
            index.index()
        )?;
        writeln!(self.writer, "WORKDIR /rust-src")?;

        Ok(())
    }

    fn write_stage_deps_copy(&mut self, index: NodeIndex<u32>) -> CargoResult<()> {
        for (inner_index, dependency) in self.graph.dependencies(index) {
            self.write_stage_deps_copy(inner_index)?;

            for output in dependency.get_exports_iter() {
                writeln!(
                    self.writer,
                    "COPY --from=builder-node-{id} {path} {path}",
                    id = inner_index.index(),
                    path = output.display(),
                )?;
            }
        }

        Ok(())
    }

    fn write_stage_sources(&mut self, node: &Node) -> CargoResult<()> {
        match node.source() {
            SourceKind::ContextPath => writeln!(self.writer, "COPY . /rust-src")?,

            SourceKind::RegistryUrl(url) => writeln!(
                self.writer,
                "RUN curl -L {} | tar -xvzC /rust-src --strip-components=1",
                url
            )?,

            SourceKind::GitCheckout { repo, reference } => {
                let checkout = match reference {
                    GitReference::Tag(name) => name,
                    GitReference::Branch(name) => name,
                    GitReference::Rev(name) => name,
                };

                writeln!(
                    self.writer,
                    r#"RUN git clone {} /rust-src && git checkout {}"#,
                    repo, checkout
                )?
            }
        }

        Ok(())
    }

    fn write_stage_env_vars(&mut self, node: &Node) -> CargoResult<()> {
        for env in &node.command().env {
            writeln!(
                self.writer,
                r#"ENV {} "{}""#,
                env.0,
                env.1
                    .trim()
                    .replace('\n', "\\n")
                    .replace('"', "\\\"")
                    .replace('\'', "\\'")
            )?;
        }

        Ok(())
    }

    fn write_stage_tree_creation(&mut self, node: &Node) -> CargoResult<()> {
        for path in find_unique_base_paths(node.get_exports_iter()) {
            writeln!(self.writer, r#"RUN ["mkdir", "-p", "{}"]"#, path.display())?;
        }

        Ok(())
    }

    fn write_stage_command(&mut self, node: &Node) -> CargoResult<()> {
        writeln!(self.writer, r#"RUN ["{}"{}]"#, node.command().program, {
            let args = {
                node.command()
                    .args
                    .iter()
                    .map(|arg| arg.replace('"', "\\\""))
                    .collect::<Vec<_>>()
                    .join(r#"", ""#)
            };

            if !args.is_empty() {
                format!(r#", "{}""#, args)
            } else {
                String::new()
            }
        })?;

        Ok(())
    }

    fn write_stage_links(&mut self, node: &Node) -> CargoResult<()> {
        for (destination, source) in node.get_links_iter() {
            writeln!(
                self.writer,
                r#"RUN ["ln", "-sf", "{}", "{}"]"#,
                source.as_relative_for(destination).display(),
                destination.display()
            )?;
        }

        Ok(())
    }

    fn write_binaries_stage(&mut self) -> CargoResult<()> {
        writeln!(self.writer, "")?;
        writeln!(self.writer, "FROM debian:stable-slim")?;

        for (index, node) in self.graph.nodes().filter(is_binary) {
            for (destination, source) in node.get_links_iter() {
                let final_path =
                    PathBuf::from("/usr/local/bin").join(destination.file_name().unwrap());

                writeln!(
                    self.writer,
                    "COPY --from=builder-node-{} {} {}",
                    index.index(),
                    source.display(),
                    final_path.display()
                )?;

                writeln!(
                    self.writer,
                    "RUN --mount=target=/usr/bin/cargo-ldd,source=/usr/local/bin/cargo-ldd,from={} [\"cargo-ldd\", \"{}\"]",
                    DEFAULT_TOOLS_STAGE,
                    final_path.display()
                )?;
            }
        }

        Ok(())
    }

    fn write_tests_stage(&mut self) -> CargoResult<()> {
        writeln!(self.writer, "")?;
        writeln!(self.writer, "FROM debian:stable-slim")?;
        writeln!(self.writer, "WORKDIR /rust-tests")?;

        let mut binaries = vec![];

        for (index, node) in self.graph.nodes().filter(is_test) {
            for builder_path in node.get_outputs_iter() {
                let final_path =
                    PathBuf::from("/rust-tests").join(builder_path.file_name().unwrap());

                writeln!(
                    self.writer,
                    "COPY --from=builder-node-{} {} {}",
                    index.index(),
                    builder_path.display(),
                    final_path.display()
                )?;

                binaries.push(final_path);
            }
        }

        writeln!(
            self.writer,
            "COPY --from={} /usr/local/bin/cargo-test-runner /usr/bin/cargo-test-runner",
            DEFAULT_TOOLS_STAGE,
        )?;

        writeln!(self.writer, "ENTRYPOINT [\"cargo-test-runner\"{}]", {
            let paths = {
                binaries
                    .iter()
                    .map(|item| item.display().to_string())
                    .collect::<Vec<_>>()
                    .join(r#"", ""#)
            };

            if !paths.is_empty() {
                format!(r#", "{}""#, paths)
            } else {
                String::new()
            }
        })?;

        Ok(())
    }
}

fn is_binary(pair: &(NodeIndex<u32>, &Node)) -> bool {
    pair.1.kind() == &NodeKind::Binary
}

fn is_test(pair: &(NodeIndex<u32>, &Node)) -> bool {
    pair.1.kind() == &NodeKind::Test
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

        DockerfilePrinter::new(OutputMode::All, &graph, &mut output)
            .write("../examples/workspace/Dockerfile.handlebars")?;

        let output_contents = String::from_utf8_lossy(&output);

        assert_eq!(
            output_contents.lines().collect::<Vec<_>>(),
            include_str!("../../../tests/simple.all.dockerfile")
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

        DockerfilePrinter::new(OutputMode::Tests, &graph, &mut output)
            .write("../examples/workspace/Dockerfile.hbs")?;

        let output_contents = String::from_utf8_lossy(&output);

        assert_eq!(
            output_contents.lines().collect::<Vec<_>>(),
            include_str!("../../../tests/simple.tests.dockerfile")
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

        DockerfilePrinter::new(OutputMode::Binaries, &graph, &mut output)
            .write("../examples/workspace/Dockerfile.hbs")?;

        let output_contents = String::from_utf8_lossy(&output);

        assert_eq!(
            output_contents.lines().collect::<Vec<_>>(),
            include_str!("../../../tests/simple.binaries.dockerfile")
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
