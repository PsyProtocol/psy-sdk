use std::{fmt, str::FromStr, task::Context};

use crate::rpc::router::JwtAuthMetadata;
use http::{header::AUTHORIZATION, Request, Response};
use jsonrpsee::server::HttpBody;
use rand::prelude::*;
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize,
};
use tower::Service;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Decoding JWT from hex failed ")]
    DecodeJwtHex(
        #[from]
        #[source]
        hex::FromHexError,
    ),
    #[error("Decoding JWT failed expected length {JWT_SECRET_LENGTH}, but got {0}")]
    DecodeJwtLength(usize),
    #[error("Decoding claim failed")]
    DecodeClaim(#[source] jsonwebtoken::errors::Error),
    #[error("Decoding claim failed")]
    EncodeClaim(#[source] jsonwebtoken::errors::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub const JWT_SECRET_LENGTH: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JwtSecret([u8; JWT_SECRET_LENGTH]);

impl Serialize for JwtSecret {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for JwtSecret {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct JwtVisitor;

        impl Visitor<'_> for JwtVisitor {
            type Value = JwtSecret;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("JWT in hex format")
            }

            fn visit_str<E>(self, value: &str) -> Result<JwtSecret, E>
            where
                E: de::Error,
            {
                JwtSecret::from_str(value).map_err(|err| {
                    E::invalid_value(
                        de::Unexpected::Str(value),
                        &format!("invalid JWT. {err}").as_str(),
                    )
                })
            }
        }

        deserializer.deserialize_str(JwtVisitor)
    }
}

impl std::fmt::Display for JwtSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl std::str::FromStr for JwtSecret {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::from_hex(s)
    }
}

impl rand::distributions::Distribution<JwtSecret> for rand::distributions::Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> JwtSecret {
        JwtSecret::new(rng.gen())
    }
}

impl JwtSecret {
    pub fn new(secret: [u8; JWT_SECRET_LENGTH]) -> Self {
        Self(secret)
    }

    pub fn from_hex(s: impl AsRef<[u8]>) -> Result<Self> {
        let vec = hex::decode(s)?;
        (&*vec)
            .try_into()
            .map(Self::new)
            .map_err(|_| Error::DecodeJwtLength(vec.len()))
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ServerLayer(pub JwtSecret);

impl<S> tower::Layer<S> for ServerLayer {
    type Service = ServerAuth<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ServerAuth { inner, jwt: self.0 }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ServerAuth<S> {
    inner: S,
    jwt: JwtSecret,
}

impl<S> Service<Request<HttpBody>> for ServerAuth<S>
where
    S: Service<Request<HttpBody>, Response = Response<HttpBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: std::fmt::Debug + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::Either<
        S::Future,
        futures::future::Ready<Result<Self::Response, Self::Error>>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<HttpBody>) -> Self::Future {
        // tracing::info!("🔐 JWT middleware called");

        if let Some(Ok(auth_str)) = req.headers().get(AUTHORIZATION).map(|v| v.to_str()) {
            // tracing::info!("🔐 Authorization: {auth_str:?}");

            if let Some(token) = auth_str
                .strip_prefix("Bearer ")
                .or_else(|| auth_str.strip_prefix("bearer "))
            {
                let token = token.trim();
                tracing::info!("🔐 Passing raw token to extensions");

                let jwt_metadata = JwtAuthMetadata {
                    token: token.to_string(),
                };
                req.extensions_mut().insert(jwt_metadata);
            } else {
                tracing::warn!("⚠️ Authorization header not Bearer format");
            }
        } else {
            // tracing::info!("⚠️ No valid Authorization header");
        }

        req.headers_mut().remove(AUTHORIZATION);
        futures::future::Either::Left(self.inner.call(req))
    }
}
