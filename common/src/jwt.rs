use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Claims {
    /// issuer
    pub issuer_uid: String,
    /// expiry
    pub exp: i64,
}
