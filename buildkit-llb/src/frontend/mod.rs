use failure::{Error, ResultExt};
use futures::compat::*;
use futures::prelude::*;
use log::*;
use tokio::executor::DefaultExecutor;
use tower_h2::client::Connection;

mod bridge;
mod stdio;
mod utils;

pub use self::bridge::Bridge;
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

    let client_bridge = Bridge::new(connection.clone());
    let controlling_bridge = Bridge::new(connection);

    debug!("running a frontend entrypoint");
    match frontend.run(client_bridge).await {
        Ok(output) => {
            controlling_bridge
                .finish_with_success(output)
                .await
                .context("Unable to send a success result")?;
        }

        Err(error) => {
            // TODO: log full error here...
            error!("Frontend entrypoint failed: {}", error.to_error_string());

            // https://godoc.org/google.golang.org/grpc/codes#Code
            controlling_bridge
                .finish_with_error(2, error.to_string())
                .await
                .context("Unable to send an error result")?;
        }
    }

    Ok(())
}
