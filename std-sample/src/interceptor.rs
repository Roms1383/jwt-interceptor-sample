use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use colored::*;
use hyper::{header::HeaderValue, Request, Response};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use std::{collections::HashMap, sync::Arc};
use std::{
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
    task::{Context, Poll},
};
use tonic::{
    body::BoxBody,
    transport::{Body, NamedService},
    Status,
};
use tower::Service;
use uuid::Uuid;

use common::constant::METADATA_USER_ID;
use common::{jwt::Claims, kid::GoogleCertificate};

#[derive(Debug, Clone)]
pub struct SharedGoogleCertificate(Arc<RwLock<GoogleCertificate>>);

pub trait ReadWrite {
    fn read(&self) -> RwLockReadGuard<GoogleCertificate>;
    fn write(&self) -> RwLockWriteGuard<GoogleCertificate>;
}

impl SharedGoogleCertificate {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(GoogleCertificate {
            kids: HashMap::new(),
            expires: Utc::now() - Duration::seconds(1), // makes sure it get refreshed on creation
        })))
    }
}

impl ReadWrite for SharedGoogleCertificate {
    fn read(&self) -> RwLockReadGuard<GoogleCertificate> {
        self.0.read().unwrap()
    }
    fn write(&self) -> RwLockWriteGuard<GoogleCertificate> {
        self.0.write().unwrap()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct InterceptedService<S> {
    pub(crate) inner: S,
    pub(crate) lock: SharedGoogleCertificate,
}

impl<S> Service<Request<Body>> for InterceptedService<S>
where
    S: Service<Request<Body>, Response = Response<BoxBody>> + NamedService + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut svc = self.inner.clone();
        let lock = self.lock.clone();

        Box::pin(async move {
            let req_id = uuid::Uuid::new_v4();
            let kids = match get_certificate(&lock, &req_id).await {
                Ok(v) => v,
                Err(_) => {
                    return Ok(Status::internal("Unable to retrieve GoogleAPIs KID").to_http());
                }
            };
            match authorize(req, kids, &req_id) {
                Ok(req) => svc.call(req).await,
                Err(e) => match e.code() {
                    tonic::Code::Unauthenticated | tonic::Code::PermissionDenied => Ok(e.to_http()),
                    _ => Ok(Status::internal("Unable to process with authorization").to_http()),
                },
            }
        })
    }
}

impl<S: NamedService> NamedService for InterceptedService<S> {
    const NAME: &'static str = S::NAME;
}

async fn get_certificate(
    lock: &SharedGoogleCertificate,
    uid: &Uuid,
) -> anyhow::Result<HashMap<String, DecodingKey<'static>>> {
    println!(
        "GET CERTIFICATES ({}) at {}",
        uid.to_string().green(),
        Utc::now().to_rfc3339()
    );
    let lock = lock.clone();
    let mut cert = {
        let guard = lock.read();
        (*guard).clone()
    };
    if cert.expires <= Utc::now() {
        println!(
            "UPDATE KIDS ({}) at {}",
            uid.to_string().on_bright_green().black(),
            Utc::now().to_rfc3339()
        );
        let response = reqwest::blocking::get(
        "https://www.googleapis.com/robot/v1/metadata/x509/securetoken@system.gserviceaccount.com"
      )?;
        let kids = {
            let from = response.json::<HashMap<String, String>>()?;
            GoogleCertificate::convert_kids(from)
        };
        cert = {
            let mut guard = lock.write();
            (*guard).expires = Utc::now() + Duration::seconds(3); // fake expiration for tests
            (*guard).kids = kids;
            (*guard).clone()
        };
    }
    Ok(cert.kids.clone())
}

fn authorize(
    mut request: Request<Body>,
    kids: HashMap<String, DecodingKey<'static>>,
    uid: &Uuid,
) -> Result<Request<Body>, Status> {
    println!(
        "AUTHORIZE ({}) at {}\n",
        uid.to_string().red(),
        Utc::now().to_rfc3339()
    ); // it's gonna fail short after it
    let token = match request.headers().get("authorization") {
        Some(token) => match token.to_str() {
            Ok(token) => token,
            Err(_) => return Err(Status::unauthenticated("Token malformed (string)")),
        },
        None => return Err(Status::unauthenticated("Token not found")),
    };
    let kid = match decode_header(token) {
        Ok(header) => match header.kid {
            Some(v) => v,
            None => {
                return Err(Status::unauthenticated(
                    "Token malformed (missing KID header)",
                ))
            }
        },
        Err(_) => return Err(Status::unauthenticated("Token malformed (header)")),
    };
    let key = match kids.get(&kid) {
        Some(v) => v,
        None => return Err(Status::unauthenticated("Token invalid (no matching KID)")),
    };
    let claims = match decode::<Claims>(token, &key, &Validation::new(Algorithm::RS256)) {
        Ok(data) => data.claims,
        Err(_) => return Err(Status::unauthenticated("Claims invalid")),
    };
    let expires = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(claims.exp, 0), Utc);
    if Utc::now() > expires {
        return Err(Status::unauthenticated("Token expired"));
    }
    if claims.issuer_uid.len() == 0 {
        return Err(Status::unauthenticated("Issuer UID empty"));
    }
    let uid = match HeaderValue::from_str(&claims.issuer_uid) {
        Ok(uid) => uid,
        Err(_) => return Err(Status::unknown("Internal service error")),
    };
    request.headers_mut().insert(METADATA_USER_ID, uid);
    Ok(request)
}
