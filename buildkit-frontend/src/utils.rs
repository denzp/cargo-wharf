use std::fmt;

use failure::Error;

#[derive(Clone, Debug)]
pub struct OutputRef(pub(crate) String);

pub struct ErrorWithCauses(pub Error, &'static str);

impl ErrorWithCauses {
    pub fn multi_line(error: Error) -> Self {
        Self(error, "\n  caused by: ")
    }

    pub fn single_line(error: Error) -> Self {
        Self(error, " => caused by: ")
    }

    pub fn into_inner(self) -> Error {
        self.0
    }
}

impl fmt::Display for ErrorWithCauses {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)?;

        for cause in self.0.iter_causes() {
            write!(f, "{}{}", self.1, cause)?;
        }

        Ok(())
    }
}
