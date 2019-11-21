pub enum PrintKind<S> {
    CustomCommand(S),
    CompileBuildScript(S),
    CompileBinary(S),
    CompileTest(S),
    CompileCrate(S),

    RunBuildScript(S),
}

pub trait PrettyPrintQuery {
    fn pretty_print<S>(&self, kind: PrintKind<S>) -> String
    where
        S: AsRef<str>,
    {
        match kind {
            PrintKind::CustomCommand(display) => format!("Running   `{}`", display.as_ref()),
            PrintKind::CompileBinary(name) => format!("Compiling binary {}", name.as_ref()),
            PrintKind::CompileTest(name) => format!("Compiling test {}", name.as_ref()),
            PrintKind::CompileCrate(name) => format!("Compiling {}", name.as_ref()),

            PrintKind::CompileBuildScript(name) => {
                format!("Compiling {} [build script]", name.as_ref())
            }

            PrintKind::RunBuildScript(name) => {
                format!("Running   {} [build script]", name.as_ref())
            }
        }
    }
}
