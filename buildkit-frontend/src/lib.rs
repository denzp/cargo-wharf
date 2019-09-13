#![deny(warnings)]
#![deny(clippy::all)]

use failure::{Error, ResultExt};
use futures::compat::*;
use futures::prelude::*;
use log::*;
use tokio::executor::DefaultExecutor;
use tower_h2::client::Connection;

mod bridge;
mod error;
mod options;
mod stdio;
mod utils;

pub mod oci;

use oci::ImageSpecification;

pub use self::bridge::Bridge;
pub use self::error::ErrorCode;
pub use self::options::Options;
pub use self::stdio::StdioSocket;
pub use self::utils::{ErrorWithCauses, OutputRef};

pub trait Frontend {
    type RunFuture: Future<Output = Result<FrontendOutput, Error>>;

    fn run(self, bridge: Bridge, options: Options) -> Self::RunFuture;
}

pub struct FrontendOutput {
    output: OutputRef,
    image_spec: Option<ImageSpecification>,
}

impl FrontendOutput {
    pub fn with_ref(output: OutputRef) -> Self {
        Self {
            output,
            image_spec: None,
        }
    }

    pub fn with_spec_and_ref(spec: ImageSpecification, output: OutputRef) -> Self {
        Self {
            output,
            image_spec: Some(spec),
        }
    }
}

pub async fn run_frontend<F: Frontend>(frontend: F) -> Result<(), Error> {
    let socket = StdioSocket::try_new()?;
    let connection = {
        Connection::handshake(socket, DefaultExecutor::current())
            .compat()
            .await
            .context("Unable to perform a HTTP/2 handshake")?
    };

    debug!("stdio socket initialized");

    let connection = {
        tower_request_modifier::Builder::new()
            .set_origin("http://localhost")
            .build(connection)
            .unwrap()
    };

    let bridge = Bridge::new(connection);
    let options = Options::analyse();

    debug!("running a frontend entrypoint");
    match frontend.run(bridge.clone(), options).await {
        Ok(output) => {
            bridge
                .finish_with_success(output.output, output.image_spec)
                .await
                .context("Unable to send a success result")?;
        }

        Err(error) => {
            let error = ErrorWithCauses::multi_line(error);

            error!("Frontend entrypoint failed: {}", error);

            // https://godoc.org/google.golang.org/grpc/codes#Code
            bridge
                .finish_with_error(
                    ErrorCode::Unknown,
                    ErrorWithCauses::single_line(error.into_inner()).to_string(),
                )
                .await
                .context("Unable to send an error result")?;
        }
    }

    // TODO: gracefully shutdown the HTTP/2 connection

    Ok(())
}
