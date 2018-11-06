use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use cargo::core::GitReference;
use petgraph::graph::NodeIndex;

use handlebars::{
    Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext, RenderError,
    Renderable,
};

use super::utils::find_unique_base_paths;
use super::DEFAULT_TOOLS_STAGE;
use crate::graph::{BuildGraph, Command, Node, NodeKind, SourceKind};

const DEFAULT_LDD_TOOL_PATH: &str = "/usr/local/bin/cargo-ldd";
const DEFAULT_BUILDSCRIPT_TOOL_PATH: &str = "/usr/local/bin/cargo-buildscript";

pub struct DockerfileHelper<H: HandlebarsHelper>(pub H);

pub trait HandlebarsHelper {
    const KEEPS_TEMPLATE: bool;

    fn expand(&self, helper: &Helper, writer: &mut Write) -> HelperResult;
}

pub struct DummyHelper;

pub struct BuildStagesHelper {
    graph: Arc<BuildGraph>,
}

pub struct BinariesHelper {
    graph: Arc<BuildGraph>,
}

pub struct TestsHelper {
    graph: Arc<BuildGraph>,
}

impl HandlebarsHelper for BuildStagesHelper {
    const KEEPS_TEMPLATE: bool = false;

    fn expand(&self, helper: &Helper, writer: &mut Write) -> HelperResult {
        let builder_name = {
            helper
                .param(0)
                .map(|v| v.value())
                .ok_or_else(|| RenderError::new("Builder name parameter is requried"))?
                .as_str()
                .ok_or_else(|| RenderError::new("Builder name parameter must be a string"))?
        };

        for (index, node) in self.graph.nodes() {
            self.write_header(index, writer, builder_name)?;
            self.write_sources(node, writer)?;
            self.write_deps_copy(index, writer)?;
            self.write_env_vars(node, writer)?;
            self.write_tree_creation(node, writer)?;
            self.write_command(node, writer)?;
            self.write_links(node, writer)?;
        }

        Ok(())
    }
}

impl BuildStagesHelper {
    pub fn new(graph: Arc<BuildGraph>) -> Self {
        BuildStagesHelper { graph }
    }

    fn write_header(
        &self,
        index: NodeIndex<u32>,
        writer: &mut Write,
        builder_name: &str,
    ) -> HelperResult {
        writeln!(writer, "")?;
        writeln!(
            writer,
            "FROM {} as builder-node-{}",
            builder_name,
            index.index()
        )?;

        writeln!(writer, "WORKDIR /rust-src")?;

        Ok(())
    }

    fn write_deps_copy(&self, index: NodeIndex<u32>, writer: &mut Write) -> HelperResult {
        for dep in self.find_deps_recursive(index) {
            writeln!(writer, "COPY --from=builder-node-{0} {1} {1}", dep.0, dep.1)?;
        }

        Ok(())
    }

    fn find_deps_recursive(&self, index: NodeIndex<u32>) -> BTreeSet<(usize, String)> {
        let mut result = BTreeSet::new();

        for (inner_index, dependency) in self.graph.dependencies(index) {
            result.append(&mut self.find_deps_recursive(inner_index));

            for output in dependency.get_exports_iter() {
                result.insert((inner_index.index(), output.display().to_string()));
            }
        }

        result
    }

    fn write_sources(&self, node: &Node, writer: &mut Write) -> HelperResult {
        match node.source() {
            SourceKind::ContextPath => writeln!(writer, "COPY . /rust-src")?,

            SourceKind::RegistryUrl(url) => writeln!(
                writer,
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
                    writer,
                    "RUN git clone {} /rust-src && git checkout {}",
                    repo, checkout
                )?
            }
        }

        Ok(())
    }

    fn write_env_vars(&self, node: &Node, writer: &mut Write) -> HelperResult {
        if let Command::Simple(ref command) = node.command() {
            for env in &command.env {
                writeln!(
                    writer,
                    "ENV {} \"{}\"",
                    env.0,
                    escape_argument(env.1.trim()),
                )?;
            }
        }

        Ok(())
    }

    fn write_tree_creation(&self, node: &Node, writer: &mut Write) -> HelperResult {
        for path in find_unique_base_paths(node.get_exports_iter()) {
            writeln!(writer, "RUN [\"mkdir\", \"-p\", \"{}\"]", path.display())?;
        }

        Ok(())
    }

    fn write_command(&self, node: &Node, writer: &mut Write) -> HelperResult {
        match node.command() {
            Command::Simple(command) if command.args.is_empty() => {
                writeln!(writer, "RUN [\"{}\"]", command.program)?
            }

            Command::Simple(command) => {
                writeln!(writer, "RUN [\"{}\", \"{}\"]", command.program, {
                    command
                        .args
                        .iter()
                        .map(escape_argument)
                        .collect::<Vec<_>>()
                        .join("\", \"")
                })?
            }

            Command::WithBuildscript {
                buildscript,
                command,
            } => {
                if let Some(out_dir) = buildscript.env.get("OUT_DIR") {
                    writeln!(writer, "RUN [\"mkdir\", \"-p\", \"{}\"]", out_dir)?;
                }

                writeln!(
                    writer,
                    "RUN [\"sh\", \"-c\", \"echo '{}' > /tmp/.buildscript-env\"]",
                    escape_argument(&serde_json::to_string(&buildscript.env)?),
                )?;

                writeln!(
                    writer,
                    "RUN [\"sh\", \"-c\", \"echo '{}' > /tmp/.rustc-args\"]",
                    escape_argument(&serde_json::to_string(&command.args)?),
                )?;

                writeln!(
                    writer,
                    "RUN [\"sh\", \"-c\", \"echo '{}' > /tmp/.rustc-env\"]",
                    escape_argument(&serde_json::to_string(&command.env)?),
                )?;

                writeln!(
                    writer,
                    r#"RUN --mount=target={tool},source={tool},from={tools_stage} ["{tool}", "{}", "--buildscript-env", "/tmp/.buildscript-env", "--rustc-args", "/tmp/.rustc-args", "--rustc-env", "/tmp/.rustc-env"]"#,
                    buildscript.program,
                    tool = DEFAULT_BUILDSCRIPT_TOOL_PATH,
                    tools_stage = DEFAULT_TOOLS_STAGE,
                )?;
            }
        };

        Ok(())
    }

    fn write_links(&self, node: &Node, writer: &mut Write) -> HelperResult {
        for (destination, source) in node.get_links_iter() {
            writeln!(
                writer,
                "RUN [\"ln\", \"-sf\", \"{}\", \"{}\"]",
                source.as_relative_for(destination).display(),
                destination.display()
            )?;
        }

        Ok(())
    }
}

impl HandlebarsHelper for BinariesHelper {
    const KEEPS_TEMPLATE: bool = true;

    fn expand(&self, _: &Helper, writer: &mut Write) -> HelperResult {
        for (index, node) in self.graph.nodes().filter(is_binary) {
            for (destination, source) in node.get_links_iter() {
                let final_path =
                    PathBuf::from("/usr/local/bin").join(destination.file_name().unwrap());

                writeln!(
                    writer,
                    "COPY --from=builder-node-{} {} {}",
                    index.index(),
                    source.display(),
                    final_path.display()
                )?;

                writeln!(
                    writer,
                    "RUN --mount=target={tool},source={tool},from={tools_stage} [\"{tool}\", \"{}\"]",
                    final_path.display(),
                    tool = DEFAULT_LDD_TOOL_PATH,
                    tools_stage = DEFAULT_TOOLS_STAGE,
                )?;
            }
        }

        Ok(())
    }
}

impl BinariesHelper {
    pub fn new(graph: Arc<BuildGraph>) -> Self {
        BinariesHelper { graph }
    }
}

impl HandlebarsHelper for TestsHelper {
    const KEEPS_TEMPLATE: bool = true;

    fn expand(&self, _: &Helper, writer: &mut Write) -> HelperResult {
        let mut binaries = vec![];

        for (index, node) in self.graph.nodes().filter(is_test) {
            for builder_path in node.get_outputs_iter() {
                let final_path =
                    PathBuf::from("/rust-tests").join(builder_path.file_name().unwrap());

                writeln!(
                    writer,
                    "COPY --from=builder-node-{} {} {}",
                    index.index(),
                    builder_path.display(),
                    final_path.display()
                )?;

                binaries.push(final_path);
            }
        }

        writeln!(
            writer,
            "COPY --from={} /usr/local/bin/cargo-test-runner /usr/bin/cargo-test-runner",
            DEFAULT_TOOLS_STAGE,
        )?;

        writeln!(writer, "ENTRYPOINT [\"cargo-test-runner\", \"{}\"]", {
            binaries
                .iter()
                .map(|item| item.display().to_string())
                .collect::<Vec<_>>()
                .join("\", \"")
        })?;

        Ok(())
    }
}

impl TestsHelper {
    pub fn new(graph: Arc<BuildGraph>) -> Self {
        TestsHelper { graph }
    }
}

impl HandlebarsHelper for DummyHelper {
    const KEEPS_TEMPLATE: bool = false;

    fn expand(&self, _: &Helper, _: &mut Write) -> HelperResult {
        Ok(())
    }
}

impl<T> HelperDef for DockerfileHelper<T>
where
    T: HandlebarsHelper + Send + Sync,
{
    fn call<'reg: 'rc, 'rc>(
        &self,
        helper: &Helper<'reg, 'rc>,
        r: &'reg Handlebars,
        ctx: &Context,
        rc: &mut RenderContext<'reg>,
        output: &mut Output,
    ) -> HelperResult {
        if T::KEEPS_TEMPLATE {
            helper
                .template()
                .map(|t| t.render(r, ctx, rc, output))
                .unwrap_or(Ok(()))?;
        }

        let mut buffer = Vec::with_capacity(1024);

        self.0.expand(helper, &mut buffer)?;
        output.write(String::from_utf8_lossy(&buffer).trim())?;

        Ok(())
    }
}

fn is_binary(pair: &(NodeIndex<u32>, &Node)) -> bool {
    pair.1.kind() == &NodeKind::Binary
}

fn is_test(pair: &(NodeIndex<u32>, &Node)) -> bool {
    pair.1.kind() == &NodeKind::Test
}

fn escape_argument<S: AsRef<str>>(input: S) -> String {
    input
        .as_ref()
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\'', "\\'")
}
