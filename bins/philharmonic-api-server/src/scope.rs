//! Header-based request scope resolver with store-backed public ID lookup.

use philharmonic::api::{RequestScope, RequestScopeResolver, ResolverError};
use philharmonic::store::IdentityStore;
use philharmonic::store_sqlx_mysql::SqlStore;
use philharmonic::types::Uuid;
use sqlx::MySqlPool;

const TENANT_ID_HEADER: &str = "x-tenant-id";

pub(crate) struct HeaderBasedScopeResolver {
    store: SqlStore,
}

impl HeaderBasedScopeResolver {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self {
            store: SqlStore::from_pool(pool),
        }
    }
}

#[async_trait::async_trait]
impl RequestScopeResolver for HeaderBasedScopeResolver {
    async fn resolve(&self, parts: &http::request::Parts) -> Result<RequestScope, ResolverError> {
        let Some(value) = parts.headers.get(TENANT_ID_HEADER) else {
            return Ok(RequestScope::Operator);
        };
        let value = value.to_str().map_err(|_| ResolverError::Unscoped)?;
        let public_uuid = Uuid::parse_str(value.trim()).map_err(|_| ResolverError::Unscoped)?;
        let identity = self
            .store
            .resolve_public(public_uuid)
            .await
            .map_err(|error| ResolverError::Internal(error.to_string()))?
            .ok_or(ResolverError::Unscoped)?;
        let tenant = identity
            .typed()
            .map_err(|error| ResolverError::Internal(error.to_string()))?;
        Ok(RequestScope::Tenant(tenant))
    }
}
