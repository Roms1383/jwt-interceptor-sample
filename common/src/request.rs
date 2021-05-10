use crate::constant::METADATA_USER_ID;

pub trait GetUserID {
    fn get_uuid_from_request(&self) -> Result<String, tonic::Status>;
}

impl<T> GetUserID for tonic::Request<T> {
    fn get_uuid_from_request(&self) -> Result<String, tonic::Status> {
        match self
            .metadata()
            .get(METADATA_USER_ID)
            .map(|metadata| metadata.to_str())
            .map(|maybe_raw| maybe_raw.unwrap())
            .map(|raw| String::from(raw))
        {
            Some(uuid) => Ok(uuid),
            None => Err(tonic::Status::unauthenticated("Please authenticate first")),
        }
    }
}
