use philharmonic::api::{RequestScope, RequestScopeResolver, ResolverError};
use philharmonic::types::{Identity, Uuid};

const TENANT_ID_HEADER: &str = "x-tenant-id";

pub(crate) struct HeaderBasedScopeResolver;

#[async_trait::async_trait]
impl RequestScopeResolver for HeaderBasedScopeResolver {
    async fn resolve(&self, parts: &http::request::Parts) -> Result<RequestScope, ResolverError> {
        let Some(value) = parts.headers.get(TENANT_ID_HEADER) else {
            return Ok(RequestScope::Operator);
        };
        let value = value.to_str().map_err(|_| ResolverError::Unscoped)?;
        let internal = Uuid::parse_str(value.trim()).map_err(|_| ResolverError::Unscoped)?;
        let identity = Identity {
            internal,
            public: synthetic_public_uuid(internal),
        };
        let tenant = identity
            .typed()
            .map_err(|error| ResolverError::Internal(error.to_string()))?;
        Ok(RequestScope::Tenant(tenant))
    }
}

fn synthetic_public_uuid(internal: Uuid) -> Uuid {
    let mut bytes = *internal.as_bytes();
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}
