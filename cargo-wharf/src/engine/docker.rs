use cargo::core::GitReference;

use crate::graph::{BuildGraph, SourceKind};

#[derive(Default)]
pub struct DockerfilePrinter;

impl DockerfilePrinter {
    pub fn print(&self, graph: &BuildGraph) {
        println!("FROM rustlang/rust:nightly as builder");
        println!("");

        for (index, node) in graph.nodes() {
            println!("FROM builder as builder-node-{}", node.id());
            println!("WORKDIR /rust-src");

            match node.source() {
                SourceKind::RegistryUrl(url) => {
                    println!(
                        "RUN curl -L {} | tar -xvzC /rust-src --strip-components=1",
                        url
                    );
                }

                SourceKind::ContextPath => {
                    println!("COPY . /rust-src");
                }

                SourceKind::GitCheckout { repo, reference } => {
                    let checkout = match reference {
                        GitReference::Tag(name) => name,
                        GitReference::Branch(name) => name,
                        GitReference::Rev(name) => name,
                    };

                    println!(
                        r#"RUN git clone {} /rust-src && git checkout {}"#,
                        repo, checkout
                    );
                }
            }

            for dependency in graph.dependencies(index) {
                // TODO(denzp): copy dependencies recursively
                for output in dependency.get_exports_iter() {
                    println!(
                        "COPY --from=builder-node-{id} {path} {path}",
                        id = dependency.id(),
                        path = output.display(),
                    );
                }
            }

            for env in &node.command().env {
                println!(
                    r#"ENV {} "{}""#,
                    env.0,
                    env.1.trim().replace('\n', " \\\n").replace('"', "\\\"")
                );
            }

            // TODO(denzp): should go through unique paths only
            for path in node.get_exports_iter() {
                println!(
                    r#"RUN ["mkdir", "-p", "{}"]"#,
                    path.parent().unwrap().display()
                );
            }

            println!(r#"RUN ["{}"{}]"#, node.command().program, {
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
            });

            for (destination, source) in node.get_links_iter() {
                println!(
                    r#"RUN ["ln", "-sf", "{}", "{}"]"#,
                    source.as_relative_for(destination).display(),
                    destination.display()
                );
            }

            println!("");
        }
    }
}
