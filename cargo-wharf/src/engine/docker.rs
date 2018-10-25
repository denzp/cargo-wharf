use std::io::Write;

use cargo::core::GitReference;
use cargo::util::CargoResult;
use petgraph::graph::NodeIndex;

use crate::engine::OutputMode;
use crate::graph::{BuildGraph, SourceKind};
use crate::path::TargetPath;

pub struct DockerfilePrinter {
    mode: OutputMode,
}

impl DockerfilePrinter {
    pub fn new(mode: OutputMode) -> Self {
        DockerfilePrinter { mode }
    }

    pub fn write(&self, graph: &BuildGraph, mut writer: impl Write) -> CargoResult<()> {
        writeln!(writer, "FROM rustlang/rust:nightly as builder")?;

        for (index, node) in graph.nodes() {
            writeln!(writer, "")?;

            writeln!(writer, "FROM builder as builder-node-{}", index.index())?;
            writeln!(writer, "WORKDIR /rust-src")?;

            match node.source() {
                SourceKind::RegistryUrl(url) => {
                    writeln!(
                        writer,
                        "RUN curl -L {} | tar -xvzC /rust-src --strip-components=1",
                        url
                    )?;
                }

                SourceKind::ContextPath => {
                    writeln!(writer, "COPY . /rust-src")?;
                }

                SourceKind::GitCheckout { repo, reference } => {
                    let checkout = match reference {
                        GitReference::Tag(name) => name,
                        GitReference::Branch(name) => name,
                        GitReference::Rev(name) => name,
                    };

                    writeln!(
                        writer,
                        r#"RUN git clone {} /rust-src && git checkout {}"#,
                        repo, checkout
                    )?;
                }
            }

            self.print_copy_deps(graph, &mut writer, index)?;

            for env in &node.command().env {
                writeln!(
                    writer,
                    r#"ENV {} "{}""#,
                    env.0,
                    env.1
                        .trim()
                        .replace('\n', "\\n")
                        .replace('"', "\\\"")
                        .replace('\'', "\\'")
                )?;
            }

            for path in Self::uniq_base_paths(node.get_exports_iter()) {
                writeln!(writer, r#"RUN ["mkdir", "-p", "{}"]"#, path.display())?;
            }

            writeln!(writer, r#"RUN ["{}"{}]"#, node.command().program, {
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

            for (destination, source) in node.get_links_iter() {
                writeln!(
                    writer,
                    r#"RUN ["ln", "-sf", "{}", "{}"]"#,
                    source.as_relative_for(destination).display(),
                    destination.display()
                )?;
            }
        }

        Ok(())
    }

    fn print_copy_deps(
        &self,
        graph: &BuildGraph,
        writer: &mut impl Write,
        index: NodeIndex<u32>,
    ) -> CargoResult<()> {
        for (inner_index, dependency) in graph.dependencies(index) {
            self.print_copy_deps(graph, writer, inner_index)?;

            for output in dependency.get_exports_iter() {
                writeln!(
                    writer,
                    "COPY --from=builder-node-{id} {path} {path}",
                    id = inner_index.index(),
                    path = output.display(),
                )?;
            }
        }

        Ok(())
    }

    fn uniq_base_paths<'a>(
        paths: impl Iterator<Item = &'a TargetPath>,
    ) -> impl Iterator<Item = TargetPath> {
        let mut input_paths = paths.map(|item| item.parent().unwrap()).collect::<Vec<_>>();
        let mut output_paths = vec![];

        input_paths.sort();

        let remaining = input_paths.iter().fold(None, |last, current| match last {
            Some(last) => {
                if current.starts_with(&last) {
                    Some(current)
                } else {
                    output_paths.push(unsafe { TargetPath::from_path(last) });
                    Some(current)
                }
            }

            None => Some(current),
        });

        if let Some(remaining) = remaining {
            output_paths.push(unsafe { TargetPath::from_path(remaining) });
        }

        output_paths.into_iter()
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
    fn it_should_group_target_paths() {
        let paths = unsafe {
            vec![
                TargetPath::from_path("/rust-out/debug/lib.rlib"),
                TargetPath::from_path("/rust-out/root.rlib"),
                TargetPath::from_path("/rust-out/debug/nested/path.log"),
                TargetPath::from_path("/rust-out/release/another/lib.rlib"),
                TargetPath::from_path("/rust-out/debug/super/nested/path.log"),
                TargetPath::from_path("/rust-out/release/nested/lib.rlib"),
            ]
        };

        assert_eq!(
            DockerfilePrinter::uniq_base_paths(paths.iter()).collect::<Vec<_>>(),
            unsafe {
                vec![
                    TargetPath::from_path("/rust-out/debug/nested"),
                    TargetPath::from_path("/rust-out/debug/super/nested"),
                    TargetPath::from_path("/rust-out/release/another"),
                    TargetPath::from_path("/rust-out/release/nested"),
                ]
            }
        );
    }

    #[test]
    fn it_should_generate_dockerfile_for_all_targets() -> CargoResult<()> {
        let config = Config::from_workspace_root("../examples/workspace")?;
        let invocations = default_invocations(&config);
        let graph = BuildGraph::from_invocations(&invocations, &config)?;

        let mut output = Vec::new();
        DockerfilePrinter::new(OutputMode::All).write(&graph, &mut output)?;

        let output_contents = String::from_utf8_lossy(&output);

        assert_eq!(
            output_contents.lines().collect::<Vec<_>>(),
            include_str!("../../../tests/simple.all.dockerfile")
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
