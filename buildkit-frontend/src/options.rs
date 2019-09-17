use std::collections::BTreeMap;
use std::env;
use std::io::Cursor;

#[derive(Debug, PartialEq)]
pub struct Options {
    inner: BTreeMap<String, OptionValue>,
}

#[derive(Debug, PartialEq)]
enum OptionValue {
    Flag(bool),
    Arguments(Vec<String>),
}

impl Options {
    pub fn analyse() -> Self {
        let env_iter = env::vars().filter_map(|(key, value)| {
            if key.starts_with("BUILDKIT_FRONTEND_OPT_") {
                Some(value)
            } else {
                None
            }
        });

        Self::from(env_iter)
    }

    pub fn has<S>(&self, name: S) -> bool
    where
        S: AsRef<str>,
    {
        match self.inner.get(name.as_ref()) {
            Some(container) => match container {
                OptionValue::Flag(exists) => *exists,
                OptionValue::Arguments(_) => true,
            },

            None => false,
        }
    }

    pub fn is_flag_set<S>(&self, name: S) -> bool
    where
        S: AsRef<str>,
    {
        match self.inner.get(name.as_ref()) {
            Some(container) => match container {
                OptionValue::Flag(flag) => *flag,
                OptionValue::Arguments(_) => false,
            },

            None => false,
        }
    }

    pub fn has_value<S1, S2>(&self, name: S1, value: S2) -> bool
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        match self.inner.get(name.as_ref()) {
            Some(container) => match container {
                OptionValue::Flag(_) => false,
                OptionValue::Arguments(values) => values.iter().any(|item| item == value.as_ref()),
            },

            None => false,
        }
    }

    pub fn get<S>(&self, name: S) -> Option<&str>
    where
        S: AsRef<str>,
    {
        match self.inner.get(name.as_ref()) {
            Some(container) => match container {
                OptionValue::Flag(_) => None,
                OptionValue::Arguments(values) => values.iter().map(String::as_str).next(),
            },

            None => None,
        }
    }

    fn extract_name_and_value(mut raw_value: String) -> (String, OptionValue) {
        if raw_value.starts_with("build-arg:") {
            raw_value = raw_value.trim_start_matches("build-arg:").into();
        }

        let delimiter_pos = raw_value.find('=');

        match delimiter_pos {
            None => (raw_value, OptionValue::Flag(true)),

            Some(pos) if &raw_value[pos + 1..] == "false" => {
                (raw_value[..pos].into(), OptionValue::Flag(false))
            }

            Some(pos) if &raw_value[pos + 1..] == "true" => {
                (raw_value[0..pos].into(), OptionValue::Flag(true))
            }

            Some(pos) => {
                let mut builder = csv::ReaderBuilder::new();
                builder.has_headers(false);

                let values = {
                    builder
                        .from_reader(Cursor::new(&raw_value[pos + 1..]))
                        .deserialize::<Vec<String>>()
                        .next()
                        .and_then(|result| result.ok())
                };

                match values {
                    Some(values) => (raw_value[..pos].into(), OptionValue::Arguments(values)),
                    None => (raw_value, OptionValue::Flag(true)),
                }
            }
        }
    }
}

impl<T> From<T> for Options
where
    T: Iterator<Item = String>,
{
    fn from(iter: T) -> Self {
        Self {
            inner: iter.map(Self::extract_name_and_value).collect(),
        }
    }
}

#[test]
fn options_parsing() {
    assert_eq!(
        Options::extract_name_and_value("name".into()),
        ("name".into(), OptionValue::Flag(true))
    );

    assert_eq!(
        Options::extract_name_and_value("name=true".into()),
        ("name".into(), OptionValue::Flag(true))
    );

    assert_eq!(
        Options::extract_name_and_value("name=false".into()),
        ("name".into(), OptionValue::Flag(false))
    );

    assert_eq!(
        Options::extract_name_and_value("name=".into()),
        ("name=".into(), OptionValue::Flag(true))
    );

    assert_eq!(
        Options::extract_name_and_value("name=value".into()),
        ("name".into(), OptionValue::Arguments(vec!["value".into()]))
    );

    assert_eq!(
        Options::extract_name_and_value("name=de=limiter".into()),
        (
            "name".into(),
            OptionValue::Arguments(vec!["de=limiter".into()])
        )
    );

    assert_eq!(
        Options::extract_name_and_value("name=false,true".into()),
        (
            "name".into(),
            OptionValue::Arguments(vec!["false".into(), "true".into()])
        )
    );

    assert_eq!(
        Options::extract_name_and_value("name=value1,value2,value3".into()),
        (
            "name".into(),
            OptionValue::Arguments(vec!["value1".into(), "value2".into(), "value3".into()])
        )
    );

    assert_eq!(
        Options::extract_name_and_value("name=value1,val=ue2,value3".into()),
        (
            "name".into(),
            OptionValue::Arguments(vec!["value1".into(), "val=ue2".into(), "value3".into()])
        )
    );

    assert_eq!(
        Options::extract_name_and_value("name=\"value1,value2\",value3".into()),
        (
            "name".into(),
            OptionValue::Arguments(vec!["value1,value2".into(), "value3".into()])
        )
    );

    assert_eq!(
        Options::extract_name_and_value("build-arg:name".into()),
        ("name".into(), OptionValue::Flag(true))
    );

    assert_eq!(
        Options::extract_name_and_value("build-arg:name=value".into()),
        ("name".into(), OptionValue::Arguments(vec!["value".into()]))
    );
}

#[test]
fn has_method() {
    let options = Options::from(
        vec![
            "option1",
            "option2=true",
            "option3=false",
            "option4=true,false",
        ]
        .into_iter()
        .map(String::from),
    );

    assert_eq!(options.has("option1"), true);
    assert_eq!(options.has("option2"), true);
    assert_eq!(options.has("option3"), false);
    assert_eq!(options.has("option4"), true);
}

#[test]
fn has_value_method() {
    let options = Options::from(
        vec!["option1", "option2=true", "option3=true,false,any_other"]
            .into_iter()
            .map(String::from),
    );

    assert_eq!(options.has_value("option1", ""), false);
    assert_eq!(options.has_value("option1", "any_other"), false);
    assert_eq!(options.has_value("option2", ""), false);
    assert_eq!(options.has_value("option2", "any_other"), false);
    assert_eq!(options.has_value("option3", "true"), true);
    assert_eq!(options.has_value("option3", "false"), true);
    assert_eq!(options.has_value("option3", "any_other"), true);
    assert_eq!(options.has_value("option3", "missing"), false);
}
