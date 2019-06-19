use futures::compat::*;
use log::*;
use tokio::executor::DefaultExecutor;

use tower_grpc::BoxBody;
use tower_grpc::Request;
use tower_h2::client::Connection;

use buildkit_proto::google::rpc::Status;
use buildkit_proto::moby::buildkit::v1::frontend::{
    client::LlbBridge, ReturnRequest, SolveRequest,
};

use crate::ops::Terminal;
use super::StdioSocket;

type BridgeConnection = tower_request_modifier::RequestModifier<
    Connection<StdioSocket, DefaultExecutor, BoxBody>,
    BoxBody,
>;

pub struct Bridge {
    client: LlbBridge<BridgeConnection>,
}

impl Bridge {
    pub(crate) fn new(client: BridgeConnection) -> Self {
        Self {
            client: LlbBridge::new(client),
        }
    }

    pub async fn solve<'a, 'b: 'a>(&'a mut self, graph: Terminal<'b>) {
        debug!("requesting to solve a definition");

        dbg!(self
            .client
            .solve(Request::new(SolveRequest {
                definition: Some(graph.into_definition()),
                exporter_attr: vec![],

                ..Default::default()
            }))
            .compat()
            .await
            .unwrap());
    }

    pub(crate) async fn finish_with_error<S>(mut self, code: i32, message: S)
    where
        S: Into<String>,
    {
        debug!("sending error result");

        dbg!(self
            .client
            .r#return(Request::new(ReturnRequest {
                result: None,
                error: Some(Status {
                    code,
                    message: message.into(),
                    details: vec![],
                }),
            }))
            .compat()
            .await
            .unwrap());
    }
}
