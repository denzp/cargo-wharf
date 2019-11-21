use buildkit_llb::prelude::*;

use super::print::{PrettyPrintQuery, PrintKind};
use super::WharfDatabase;

use crate::config::{BaseImageConfig, CustomCommand};

pub trait SourceQuery: WharfDatabase + PrettyPrintQuery {
    fn builder_source(&self) -> Option<OperationOutput<'_>> {
        self.source_llb(
            self.config().builder(),
            self.config()
                .builder()
                .setup_commands()
                .map(Vec::as_ref)
                .unwrap_or_default(),
        )
    }

    fn output_source(&self) -> Option<OperationOutput<'_>> {
        self.source_llb(
            self.config().output(),
            self.config()
                .output()
                .pre_install_commands()
                .map(Vec::as_ref)
                .unwrap_or_default(),
        )
    }

    fn source_llb<'a>(
        &self,
        config: &'a dyn BaseImageConfig,
        commands: &'a [CustomCommand],
    ) -> Option<OperationOutput<'a>> {
        if !commands.is_empty() {
            let mut last_output = config.image_source().map(|source| source.output());

            for (name, args, display) in commands.iter().map(From::from) {
                last_output = Some(
                    config
                        .populate_env(Command::run(name))
                        .args(args.iter())
                        .mount(match last_output {
                            Some(output) => Mount::Layer(OutputIdx(0), output, "/"),
                            None => Mount::Scratch(OutputIdx(0), "/"),
                        })
                        .custom_name(self.pretty_print(PrintKind::CustomCommand(display)))
                        .ref_counted()
                        .output(0),
                );
            }

            last_output
        } else {
            config.image_source().map(|source| source.output())
        }
    }
}
