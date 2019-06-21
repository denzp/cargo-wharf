use std::fmt::Write;

use failure::Error;

pub trait ToErrorString {
    fn to_error_string(&self) -> String;
}

#[derive(Clone, Debug)]
pub struct OutputRef(pub(crate) String);

impl ToErrorString for Error {
    fn to_error_string(&self) -> String {
        let mut result = String::new();
        write!(result, "{}", self).ok();

        for cause in self.iter_causes() {
            write!(result, "\n  caused by: {}", cause).ok();
        }

        result
    }
}
