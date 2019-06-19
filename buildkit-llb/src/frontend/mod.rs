use failure::Error;
use futures::compat::*;
use futures::prelude::*;
use log::*;
use tokio::executor::DefaultExecutor;
use tower_h2::client::Connection;

mod bridge;
mod stdio;

pub use self::bridge::Bridge;
pub use self::stdio::StdioSocket;

pub trait Frontend {
    type RunFuture: Future<Output = Result<(), Error>>;

    fn run(self, bridge: Bridge) -> Self::RunFuture;
}

pub async fn run_frontend<F: Frontend>(frontend: F) -> Result<(), Error> {
    let socket = StdioSocket::try_new()?;
    let connection = Connection::handshake(socket, DefaultExecutor::current())
        .compat()
        .await?;

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
    frontend.run(client_bridge).await.unwrap();

    // https://godoc.org/google.golang.org/grpc/codes#Code
    controlling_bridge
        .finish_with_error(2, format!("{:#?}", "TODO"))
        .await;

    Ok(())
}
