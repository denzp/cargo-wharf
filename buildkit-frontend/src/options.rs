use std::env;

pub struct Options {
    inner: Vec<String>,
}

impl Options {
    pub fn analyse() -> Self {
        let values = env::vars().filter_map(|(key, value)| {
            if key.starts_with("BUILDKIT_FRONTEND_OPT_") {
                Some(value)
            } else {
                None
            }
        });

        Self {
            inner: values.collect(),
        }
    }
}

impl Into<Vec<String>> for Options {
    fn into(self) -> Vec<String> {
        self.inner
    }
}
