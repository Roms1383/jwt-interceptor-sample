mod interceptor;
mod server;

#[tokio::main]
async fn main() -> std::result::Result<(), tonic::transport::Error> {
    server::start().await
}
