use failure::{Error, ResultExt};
use futures::compat::*;
use futures::prelude::*;
use log::*;
use tokio::executor::DefaultExecutor;
use tower_h2::client::Connection;

mod bridge;
mod error;
mod stdio;
mod utils;

pub use self::bridge::Bridge;
pub use self::error::ErrorCode;
pub use self::stdio::StdioSocket;
pub use self::utils::{OutputRef, ToErrorString};

pub trait Frontend {
    type RunFuture: Future<Output = Result<OutputRef, Error>>;

    fn run(self, bridge: Bridge) -> Self::RunFuture;
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

    debug!("running a frontend entrypoint");
    match frontend.run(bridge.clone()).await {
        Ok(output) => {
            bridge
                .finish_with_success(output)
                .await
                .context("Unable to send a success result")?;
        }

        Err(error) => {
            error!("Frontend entrypoint failed: {}", error.to_error_string());

            // https://godoc.org/google.golang.org/grpc/codes#Code
            bridge
                .finish_with_error(ErrorCode::Unknown, error.to_string())
                .await
                .context("Unable to send an error result")?;
        }
    }

    // TODO: gracefully shutdown the HTTP/2 connection

    Ok(())
}
