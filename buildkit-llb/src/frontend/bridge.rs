use failure::{bail, format_err, Error};
use futures::compat::*;
use log::*;
use tokio::executor::DefaultExecutor;

use tower_grpc::BoxBody;
use tower_grpc::Request;
use tower_h2::client::Connection;

use buildkit_proto::google::rpc::Status;
use buildkit_proto::moby::buildkit::v1::frontend::{
    client::LlbBridge, result::Result as RefResult, Result as Output, ReturnRequest, SolveRequest,
};

use super::{OutputRef, StdioSocket};
use crate::ops::Terminal;

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

    pub async fn solve<'a, 'b: 'a>(&'a mut self, graph: Terminal<'b>) -> Result<OutputRef, Error> {
        debug!("requesting to solve a graph: {:#?}", graph);

        let request = SolveRequest {
            definition: Some(graph.into_definition()),
            exporter_attr: vec![],
            allow_result_return: true,

            ..Default::default()
        };

        let response = {
            self.client
                .solve(Request::new(request))
                .compat()
                .await?
                .into_inner()
                .result
                .ok_or_else(|| format_err!("Unable to extract solve result"))?
        };

        debug!("got response: {:#?}", response);

        let inner = {
            response
                .result
                .ok_or_else(|| format_err!("Unable to extract solve result"))?
        };

        match inner {
            RefResult::Ref(inner) => Ok(OutputRef(inner)),
            other => bail!("Unexpected solve response: {:?}", other),
        }
    }

    pub(crate) async fn finish_with_success(mut self, output: OutputRef) -> Result<(), Error> {
        let request = ReturnRequest {
            error: None,
            result: Some(Output {
                result: Some(RefResult::Ref(output.0)),
                metadata: Default::default(),
            }),
        };

        debug!("sending a success result: {:#?}", request);
        self.client.r#return(Request::new(request)).compat().await?;

        Ok(())
    }

    pub(crate) async fn finish_with_error<S>(mut self, code: i32, message: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let request = ReturnRequest {
            result: None,
            error: Some(Status {
                code,
                message: message.into(),
                details: vec![],
            }),
        };

        debug!("sending an error result: {:#?}", request);
        self.client.r#return(Request::new(request)).compat().await?;

        Ok(())
    }
}
