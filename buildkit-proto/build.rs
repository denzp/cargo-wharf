const PROTOS: &[&str] = &["proto/github.com/moby/buildkit/frontend/gateway/pb/gateway.proto"];

fn main() {
    tower_grpc_build::Config::new()
        .enable_server(true)
        .enable_client(true)
        .build(PROTOS, &["proto"])
        .unwrap_or_else(|e| panic!("protobuf compilation failed: {}", e));
}
