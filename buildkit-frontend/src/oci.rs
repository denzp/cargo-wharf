use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;

use chrono::prelude::*;
use serde_derive::{Deserialize, Serialize};

// https://github.com/opencontainers/image-spec/blob/v1.0.1/config.md

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageSpecification {
    /// An combined date and time at which the image was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<DateTime<Utc>>,

    /// Gives the name and/or email address of the person or entity which created and is responsible for maintaining the image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// The CPU architecture which the binaries in this image are built to run on.
    pub architecture: Architecture,

    /// The name of the operating system which the image is built to run on.
    pub os: OperatingSystem,

    /// The execution parameters which should be used as a base when running a container using the image.
    /// This field can be `None`, in which case any execution parameters should be specified at creation of the container.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ImageConfig>,

    /// The rootfs key references the layer content addresses used by the image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rootfs: Option<ImageRootfs>,

    /// Describes the history of each layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<LayerHistoryItem>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Architecture {
    /// 64-bit x86, the most mature port
    Amd64,

    /// 32-bit x86
    I386,

    /// 32-bit ARM
    ARM,

    /// 64-bit ARM
    ARM64,

    /// PowerPC 64-bit, little-endian
    PPC64le,

    /// PowerPC 64-bit, big-endian
    PPC64,

    /// MIPS 64-bit, little-endian
    Mips64le,

    /// MIPS 64-bit, big-endian
    Mips64,

    /// MIPS 32-bit, little-endian
    Mipsle,

    /// MIPS 32-bit, big-endian
    Mips,

    /// IBM System z 64-bit, big-endian
    S390x,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperatingSystem {
    Darwin,
    Dragonfly,
    Freebsd,
    Linux,
    Netbsd,
    Openbsd,
    Plan9,
    Solaris,
    Windows,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ImageConfig {
    /// The username or UID which is a platform-specific structure that allows specific control over which user the process run as.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// A set of ports to expose from a container running this image.
    #[serde(with = "golang_map")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exposed_ports: Option<Vec<ExposedPort>>,

    /// Environment variables for the process to run with.
    #[serde(with = "env_variables")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,

    /// A list of arguments to use as the command to execute when the container starts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entrypoint: Option<Vec<String>>,

    /// Default arguments to the entrypoint of the container.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmd: Option<Vec<String>>,

    /// A set of directories describing where the process is likely write data specific to a container instance.
    #[serde(with = "golang_map")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<String>>,

    /// Sets the current working directory of the entrypoint process in the container.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,

    /// The field contains arbitrary metadata for the container.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,

    /// The field contains the system call signal that will be sent to the container to exit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_signal: Option<Signal>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageRootfs {
    /// Must be set to `RootfsType::Layers`.
    #[serde(rename = "type")]
    pub diff_type: RootfsType,

    /// An array of layer content hashes (DiffIDs), in order from first to last.
    pub diff_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerHistoryItem {
    /// A combined date and time at which the layer was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<DateTime<Utc>>,

    /// The author of the build point.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// The command which created the layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    /// A custom message set when creating the layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    /// This field is used to mark if the history item created a filesystem diff.
    /// It is set to true if this history item doesn't correspond to an actual layer in the rootfs section
    /// (for example, Dockerfile's ENV command results in no change to the filesystem).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_layer: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String")]
#[serde(into = "String")]
pub enum ExposedPort {
    Tcp(u16),
    Udp(u16),
}

impl TryFrom<String> for ExposedPort {
    type Error = std::num::ParseIntError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let postfix_len = value.len() - 4;

        match &value[postfix_len..] {
            "/tcp" => Ok(ExposedPort::Tcp(value[..postfix_len].parse()?)),
            "/udp" => Ok(ExposedPort::Udp(value[..postfix_len].parse()?)),

            _ => Ok(ExposedPort::Tcp(value.parse()?)),
        }
    }
}

impl Into<String> for ExposedPort {
    fn into(self) -> String {
        match self {
            ExposedPort::Tcp(port) => format!("{}/tcp", port),
            ExposedPort::Udp(port) => format!("{}/udp", port),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RootfsType {
    Layers,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Signal {
    SIGHUP,
    SIGINT,
    SIGQUIT,
    SIGILL,
    SIGTRAP,
    SIGABRT,
    SIGBUS,
    SIGFPE,
    SIGKILL,
    SIGUSR1,
    SIGSEGV,
    SIGUSR2,
    SIGPIPE,
    SIGALRM,
    SIGTERM,
    SIGSTKFLT,
    SIGCHLD,
    SIGCONT,
    SIGSTOP,
    SIGTSTP,
    SIGTTIN,
    SIGTTOU,
    SIGURG,
    SIGXCPU,
    SIGXFSZ,
    SIGVTALRM,
    SIGPROF,
    SIGWINCH,
    SIGIO,
    SIGPWR,
    SIGSYS,
    SIGEMT,
    SIGINFO,
}

mod golang_map {
    use serde::de::{MapAccess, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::marker::PhantomData;

    use super::*;

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<Vec<T>>, D::Error>
    where
        T: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(MapVisitor::default())
    }

    pub fn serialize<T, S>(value: &Option<Vec<T>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        serializer.collect_map(
            value
                .as_ref()
                .unwrap()
                .iter()
                .map(|key| (key, HashMap::<(), ()>::with_capacity(0))),
        )
    }

    struct MapVisitor<T> {
        marker: PhantomData<T>,
    }

    impl<T> Default for MapVisitor<T> {
        fn default() -> Self {
            Self {
                marker: PhantomData,
            }
        }
    }

    impl<'de, T> Visitor<'de> for MapVisitor<T>
    where
        T: Deserialize<'de>,
    {
        type Value = Option<Vec<T>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a nonempty sequence of numbers")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut seq = Vec::with_capacity(access.size_hint().unwrap_or(0));

            while let Some((key, _)) = access.next_entry::<T, serde_json::Value>()? {
                seq.push(key);
            }

            Ok(Some(seq))
        }
    }
}

mod env_variables {
    use serde::de::{Error, SeqAccess, Visitor};
    use serde::{Deserializer, Serializer};

    use super::*;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<HashMap<String, String>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(SeqVisitor::default())
    }

    pub fn serialize<S>(
        value: &Option<HashMap<String, String>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(
            value
                .as_ref()
                .unwrap()
                .iter()
                .map(|(key, value)| format!("{}={}", key, value)),
        )
    }

    #[derive(Default)]
    struct SeqVisitor;

    impl<'de> Visitor<'de> for SeqVisitor {
        type Value = Option<HashMap<String, String>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a nonempty sequence of numbers")
        }

        fn visit_seq<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: SeqAccess<'de>,
        {
            let mut kv = HashMap::with_capacity(access.size_hint().unwrap_or(0));

            while let Some(mut pair) = access.next_element::<String>()? {
                let separator = pair
                    .find('=')
                    .ok_or_else(|| Error::custom("Expected a pair ENV_NAME=Env_VALUE"))?;

                let value: String = pair.drain(separator + 1..).collect();
                pair.truncate(separator);

                let key = pair;

                kv.insert(key, value);
            }

            Ok(Some(kv))
        }
    }
}

#[test]
fn serialization() {
    use pretty_assertions::assert_eq;

    let ref_json = include_str!("../tests/oci-image-spec.json");
    let ref_spec = ImageSpecification {
        created: Some("2015-10-31T22:22:56.015925234Z".parse().unwrap()),
        author: Some("Alyssa P. Hacker <alyspdev@example.com>".into()),
        architecture: Architecture::Amd64,
        os: OperatingSystem::Linux,
        rootfs: Some(ImageRootfs {
            diff_type: RootfsType::Layers,
            diff_ids: vec![
                "sha256:c6f988f4874bb0add23a778f753c65efe992244e148a1d2ec2a8b664fb66bbd1".into(),
                "sha256:5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef".into(),
            ],
        }),
        history: Some(vec![
            LayerHistoryItem {
                created: Some("2015-10-31T22:22:54.690851953Z".parse().unwrap()),
                created_by: Some("/bin/sh -c #(nop) ADD file in /".into()),
                author: None,
                comment: None,
                empty_layer: None,
            },
            LayerHistoryItem {
                created: Some("2015-10-31T22:22:55.613815829Z".parse().unwrap()),
                created_by: Some("/bin/sh -c #(nop) CMD [\"sh\"]".into()),
                author: None,
                comment: None,
                empty_layer: Some(true),
            },
        ]),

        config: Some(ImageConfig {
            user: Some("alice".into()),
            exposed_ports: Some(vec![ExposedPort::Tcp(8080), ExposedPort::Udp(8081)]),
            env: Some(
                vec![(
                    String::from("PATH"),
                    String::from("/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"),
                )]
                .into_iter()
                .collect(),
            ),
            entrypoint: Some(vec!["/bin/my-app-binary".into()]),
            cmd: Some(vec![
                "--foreground".into(),
                "--config".into(),
                "/etc/my-app.d/default.cfg".into(),
            ]),
            volumes: Some(vec![
                "/var/job-result-data".into(),
                "/var/log/my-app-logs".into(),
            ]),
            working_dir: Some("/home/alice".into()),
            labels: Some(
                vec![(
                    String::from("com.example.project.git.url"),
                    String::from("https://example.com/project.git"),
                )]
                .into_iter()
                .collect(),
            ),
            stop_signal: Some(Signal::SIGKILL),
        }),
    };

    assert_eq!(serde_json::to_string_pretty(&ref_spec).unwrap(), ref_json);
    assert_eq!(
        serde_json::from_str::<ImageSpecification>(ref_json).unwrap(),
        ref_spec
    );
}

#[test]
fn min_serialization() {
    use pretty_assertions::assert_eq;

    let ref_json = include_str!("../tests/oci-image-spec-min.json");
    let ref_spec = ImageSpecification {
        created: None,
        author: None,

        architecture: Architecture::Amd64,
        os: OperatingSystem::Linux,
        rootfs: Some(ImageRootfs {
            diff_type: RootfsType::Layers,
            diff_ids: vec![
                "sha256:c6f988f4874bb0add23a778f753c65efe992244e148a1d2ec2a8b664fb66bbd1".into(),
                "sha256:5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef".into(),
            ],
        }),

        history: None,
        config: None,
    };

    assert_eq!(serde_json::to_string_pretty(&ref_spec).unwrap(), ref_json);
    assert_eq!(
        serde_json::from_str::<ImageSpecification>(ref_json).unwrap(),
        ref_spec
    );
}
