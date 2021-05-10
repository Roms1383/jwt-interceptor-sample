use tonic::transport::Server;

use crate::interceptor::{InterceptedService, SharedGoogleCertificate};

use common::{domain, server::UsersServerImpl};

pub async fn start() -> std::result::Result<(), tonic::transport::Error> {
    let cache = SharedGoogleCertificate::new();
    let gateway = InterceptedService {
        inner: domain::gateway::service::users_server::UsersServer::new(UsersServerImpl {}),
        lock: cache.clone(),
    };
    Server::builder()
        .add_service(gateway)
        .serve("0.0.0.0:50051".to_string().parse().unwrap())
        .await?;
    Ok(())
}
