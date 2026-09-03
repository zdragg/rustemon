//! Defines the client used to access Pokeapi.

#[cfg(feature = "cache")]
use http_cache_reqwest::{Cache, CacheManager, HttpCache, HttpCacheOptions};
use reqwest::{Client, IntoUrl, Url};
#[cfg(not(feature = "cache"))]
use reqwest_middleware::ClientBuilder;
use reqwest_middleware::ClientWithMiddleware;
#[cfg(feature = "cache")]
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use serde::de::DeserializeOwned;

use crate::error::Error;

// Reexport to ease overloading.
#[cfg(feature = "cache")]
pub use http_cache_reqwest::{CacheMode, CacheOptions};

#[cfg(feature = "cache")]
pub use http_cache_reqwest::{CACacheManager, MokaManager};

/// Environment to target while calling `PokeApi`.
#[derive(Clone, Default)]
pub enum Environment {
    /// Targets the production environment.
    #[default]
    Production,
    /// Targets the stating environment.
    Staging,
    /// Targets a custom environment of `PokeApi`, a local deployment through Docker for example.
    Custom(String),
}

impl TryFrom<Environment> for Url {
    type Error = Error;

    fn try_from(value: Environment) -> Result<Self, Self::Error> {
        match value {
            Environment::Production => Ok(Self::parse("https://pokeapi.co/api/v2/").unwrap()),
            Environment::Staging => Ok(Self::parse("https://staging.pokeapi.co/api/v2/").unwrap()),
            Environment::Custom(mut s) => {
                if !s.ends_with('/') {
                    s.push('/');
                }

                Self::parse(&s).map_err(|_| Error::UrlParse(s))
            }
        }
    }
}

/// Custom client used to call Pokeapi.
#[derive(Debug)]
pub struct RustemonClient {
    /// Inner client.
    pub client: ClientWithMiddleware,
    /// Base URL for the API
    pub base: Url,
}

/// Inner representation of an endpoint's id. Used to ease the api calls.
pub(crate) enum Id<'a> {
    Int(i64),
    Str(&'a str),
}

impl RustemonClient {
    /// Calls the api through the given [Url].
    async fn inner_get<T>(&self, url: Url) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        Ok(self.client.get(url).send().await?.json().await?)
    }

    /// Make a call through the client to the given `endpoint`.
    pub(crate) async fn get_by_endpoint<T>(&self, endpoint: &str) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let url = self
            .base
            .join(endpoint)
            .map_err(|_| Error::UrlParse(format!("{}/{endpoint}", self.base)))?;
        self.inner_get(url).await
    }

    /// Make a call through the client to the given `endpoint`, adding `limit` and `offset` to the query.
    pub(crate) async fn get_with_limit_and_offset<T>(
        &self,
        endpoint: &str,
        limit: i64,
        offset: i64,
    ) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let mut url = self
            .base
            .join(endpoint)
            .map_err(|_| Error::UrlParse(format!("{}/{endpoint}", self.base)))?;
        url.set_query(Some(&format!("limit={limit}&offset={offset}")));
        self.inner_get(url).await
    }

    /// Make a call though the client to the given `endpoint`, targetting a specific resource described by [Id].
    pub(crate) async fn get_by_endpoint_and_id<T>(
        &self,
        endpoint: &str,
        id: Id<'_>,
    ) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let endpoint_id = match id {
            Id::Int(i) => format!("{endpoint}/{i}"),
            Id::Str(s) => format!("{endpoint}/{s}"),
        };
        let url = self
            .base
            .join(&endpoint_id)
            .map_err(|_| Error::UrlParse(format!("{}/{endpoint_id}", self.base)))?;
        self.inner_get(url).await
    }

    /// Make a call through the client from a given [`IntoUrl`].
    pub(crate) async fn get_by_url<T>(&self, url: impl IntoUrl) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        self.inner_get(url.into_url()?).await
    }
}

impl Default for RustemonClient {
    /// Returns a `RustemonClient` with default configuration.
    fn default() -> Self {
        #[cfg(feature = "cache")]
        let client = {
            let manager = CACacheManager::new("./rustemon-cache".into(), false);

            ClientBuilder::new(Client::new())
                .with(Cache(HttpCache {
                    mode: CacheMode::Default,
                    manager,
                    options: HttpCacheOptions::default(),
                }))
                .build()
        };

        #[cfg(not(feature = "cache"))]
        let client = ClientBuilder::new(Client::new()).build();

        Self {
            client,
            base: Url::try_from(Environment::default()).unwrap(),
        }
    }
}
