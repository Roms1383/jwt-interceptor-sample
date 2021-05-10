use crate::domain::gateway::service::users_server::Users;
use crate::domain::gateway::service::User;
use crate::request::GetUserID;

pub struct UsersServerImpl;

#[tonic::async_trait]
impl Users for UsersServerImpl {
    async fn get_user(
        &self,
        request: tonic::Request<()>,
    ) -> Result<tonic::Response<User>, tonic::Status> {
        let id = request.get_uuid_from_request()?;
        Ok(tonic::Response::new(User { id }.into()))
    }
}
