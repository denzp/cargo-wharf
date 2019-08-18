use std::collections::HashMap;
use std::path::PathBuf;

use failure::{bail, format_err, Error, ResultExt};
use futures::compat::*;
use log::*;
use tokio::executor::DefaultExecutor;

use tower_grpc::BoxBody;
use tower_grpc::Request;
use tower_h2::client::Connection;

use buildkit_proto::google::rpc::Status;
use buildkit_proto::moby::buildkit::v1::frontend::{
    client, result::Result as RefResult, ReadFileRequest, Result as Output, ReturnRequest,
    SolveRequest,
};

pub use buildkit_llb::ops::Terminal;
pub use buildkit_proto::moby::buildkit::v1::frontend::FileRange;

use crate::error::ErrorCode;
use crate::oci::ImageSpecification;
use crate::stdio::StdioSocket;
use crate::utils::OutputRef;

type BridgeConnection = tower_request_modifier::RequestModifier<
    Connection<StdioSocket, DefaultExecutor, BoxBody>,
    BoxBody,
>;

#[derive(Clone)]
pub struct Bridge {
    client: client::LlbBridge<BridgeConnection>,
}

impl Bridge {
    pub(crate) fn new(client: BridgeConnection) -> Self {
        Self {
            client: client::LlbBridge::new(client),
        }
    }

    pub async fn solve<'a, 'b: 'a>(&'a mut self, graph: Terminal<'b>) -> Result<OutputRef, Error> {
        debug!("serializing a graph to request");
        let request = SolveRequest {
            definition: Some(graph.into_definition()),
            exporter_attr: vec![],
            allow_result_return: true,

            ..Default::default()
        };

        debug!("requesting to solve a graph");
        let response = {
            self.client
                .solve(Request::new(request))
                .compat()
                .await
                .context("Unable to solve the graph")?
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

    pub async fn read_file<'a, 'b: 'a, P>(
        &'a mut self,
        layer: &'b OutputRef,
        path: P,
        range: Option<FileRange>,
    ) -> Result<Vec<u8>, Error>
    where
        P: Into<PathBuf>,
    {
        let file_path = path.into().display().to_string();
        debug!("requesting a file contents: {:#?}", file_path);

        let request = ReadFileRequest {
            r#ref: layer.0.clone(),
            file_path,
            range,
        };

        let response = {
            self.client
                .read_file(Request::new(request))
                .compat()
                .await
                .context("Unable to read the file")?
                .into_inner()
                .data
        };

        Ok(response)
    }

    pub(crate) async fn finish_with_success(
        mut self,
        output: OutputRef,
        config: Option<ImageSpecification>,
    ) -> Result<(), Error> {
        let mut metadata = HashMap::new();

        if let Some(config) = config {
            metadata.insert("containerimage.config".into(), serde_json::to_vec(&config)?);
        }

        let request = ReturnRequest {
            error: None,
            result: Some(Output {
                result: Some(RefResult::Ref(output.0)),
                metadata,
            }),
        };

        debug!("sending a success result: {:#?}", request);
        self.client.r#return(Request::new(request)).compat().await?;

        // TODO: gracefully shutdown the HTTP/2 connection

        Ok(())
    }

    pub(crate) async fn finish_with_error<S>(
        mut self,
        code: ErrorCode,
        message: S,
    ) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let request = ReturnRequest {
            result: None,
            error: Some(Status {
                code: code as i32,
                message: message.into(),
                details: vec![],
            }),
        };

        debug!("sending an error result: {:#?}", request);
        self.client.r#return(Request::new(request)).compat().await?;

        // TODO: gracefully shutdown the HTTP/2 connection

        Ok(())
    }
}
