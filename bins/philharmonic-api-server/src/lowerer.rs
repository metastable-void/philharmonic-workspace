use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use coset::CborSerializable;
use philharmonic::connector_client::{AeadAadInputs, LowererSigningKey, encrypt_payload};
use philharmonic::connector_common::{ConnectorTokenClaims, RealmPublicKey};
use philharmonic::policy::{Sck, TenantEndpointConfig, sck_decrypt};
use philharmonic::store::{ContentStore, EntityStoreExt, IdentityStore};
use philharmonic::store_sqlx_mysql::SqlStore;
use philharmonic::types::{EntityId, JsonValue, ScalarValue, Sha256, UnixMillis, Uuid};
use philharmonic::workflow::{
    ConfigLowerer, ConfigLoweringError, SubjectContext, WorkflowInstance,
};
use rand_core::OsRng;
use serde_json::json;

pub(crate) struct ConnectorConfigLowerer {
    signing_key: LowererSigningKey,
    realm_keys: HashMap<String, RealmPublicKey>,
    issuer: String,
    token_lifetime_ms: u64,
    store: SqlStore,
    sck: Arc<Sck>,
    connector_router_url: String,
    connector_domain_suffix: String,
}

impl ConnectorConfigLowerer {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        signing_key: LowererSigningKey,
        realm_keys: HashMap<String, RealmPublicKey>,
        issuer: String,
        token_lifetime_ms: u64,
        store: SqlStore,
        sck: Arc<Sck>,
        connector_router_url: String,
        connector_domain_suffix: String,
    ) -> Self {
        Self {
            signing_key,
            realm_keys,
            issuer,
            token_lifetime_ms,
            store,
            sck,
            connector_router_url,
            connector_domain_suffix,
        }
    }
}

#[async_trait]
impl ConfigLowerer for ConnectorConfigLowerer {
    async fn lower(
        &self,
        abstract_config: &JsonValue,
        instance_id: EntityId<WorkflowInstance>,
        step_seq: u64,
        subject: &SubjectContext,
    ) -> Result<JsonValue, ConfigLoweringError> {
        let bindings = abstract_config.as_object().ok_or_else(|| {
            ConfigLoweringError::InvalidConfig("abstract config must be a JSON object".to_string())
        })?;

        let realm = self
            .realm_keys
            .keys()
            .next()
            .ok_or_else(|| {
                ConfigLoweringError::InvalidConfig("no realm public keys configured".to_string())
            })?
            .clone();
        let realm_key = &self.realm_keys[&realm];

        let tenant = subject.tenant_id;
        let inst = instance_id.internal().as_uuid();
        let kid = self.signing_key.kid();

        let mut endpoints = serde_json::Map::new();

        for (name, value) in bindings {
            let uuid_str = value.as_str().ok_or_else(|| {
                ConfigLoweringError::InvalidConfig(format!(
                    "abstract config value for '{name}' must be an endpoint config UUID string"
                ))
            })?;
            let public_id = Uuid::parse_str(uuid_str).map_err(|error| {
                ConfigLoweringError::InvalidConfig(format!(
                    "invalid endpoint config UUID for '{name}': {error}"
                ))
            })?;

            let identity = self
                .store
                .resolve_public(public_id)
                .await
                .map_err(|error| {
                    ConfigLoweringError::Backend(format!(
                        "endpoint config lookup failed for '{name}': {error}"
                    ))
                })?
                .ok_or_else(|| {
                    ConfigLoweringError::InvalidConfig(format!(
                        "endpoint config '{uuid_str}' not found for '{name}'"
                    ))
                })?;
            let endpoint_id: EntityId<TenantEndpointConfig> =
                identity.typed().map_err(|error| {
                    ConfigLoweringError::Backend(format!(
                        "endpoint config identity type error for '{name}': {error}"
                    ))
                })?;

            let latest = self
                .store
                .get_latest_revision_typed::<TenantEndpointConfig>(endpoint_id)
                .await
                .map_err(|error| {
                    ConfigLoweringError::Backend(format!(
                        "endpoint config revision lookup failed: {error}"
                    ))
                })?
                .ok_or_else(|| {
                    ConfigLoweringError::Backend("endpoint config has no revisions".to_string())
                })?;

            let key_version = latest
                .scalar_attrs
                .get("key_version")
                .and_then(|v| match v {
                    ScalarValue::I64(n) => Some(*n),
                    _ => None,
                })
                .ok_or_else(|| {
                    ConfigLoweringError::Backend(
                        "endpoint config missing key_version scalar".to_string(),
                    )
                })?;

            let encrypted_hash = latest
                .content_attrs
                .get("encrypted_config")
                .copied()
                .ok_or_else(|| {
                    ConfigLoweringError::Backend(
                        "endpoint config missing encrypted_config content".to_string(),
                    )
                })?;

            let encrypted_blob = self
                .store
                .get(encrypted_hash)
                .await
                .map_err(|error| {
                    ConfigLoweringError::Backend(format!(
                        "encrypted config blob read failed: {error}"
                    ))
                })?
                .ok_or_else(|| {
                    ConfigLoweringError::Backend(
                        "encrypted config content blob not found".to_string(),
                    )
                })?;

            let config_plaintext = sck_decrypt(
                &self.sck,
                encrypted_blob.bytes(),
                tenant.internal().as_uuid(),
                endpoint_id.internal().as_uuid(),
                key_version,
            )
            .map_err(|error| {
                ConfigLoweringError::Backend(format!(
                    "endpoint config decryption failed for '{name}': {error}"
                ))
            })?;

            let config_json: JsonValue = serde_json::from_slice(config_plaintext.as_slice())
                .map_err(|error| {
                    ConfigLoweringError::Backend(format!(
                        "decrypted endpoint config is invalid JSON for '{name}': {error}"
                    ))
                })?;

            let impl_hash = latest
                .content_attrs
                .get("implementation")
                .copied()
                .ok_or_else(|| {
                    ConfigLoweringError::Backend(
                        "endpoint config missing implementation content".to_string(),
                    )
                })?;
            let impl_blob = self
                .store
                .get(impl_hash)
                .await
                .map_err(|error| {
                    ConfigLoweringError::Backend(format!(
                        "implementation blob read failed: {error}"
                    ))
                })?
                .ok_or_else(|| {
                    ConfigLoweringError::Backend(
                        "implementation content blob not found".to_string(),
                    )
                })?;
            let impl_name: String = serde_json::from_slice(impl_blob.bytes()).map_err(|error| {
                ConfigLoweringError::Backend(format!(
                    "stored implementation name is invalid JSON: {error}"
                ))
            })?;

            let connector_payload = json!({
                "realm": realm,
                "impl": impl_name,
                "config": config_json,
            });

            let payload_bytes = serde_json::to_vec(&connector_payload).map_err(|error| {
                ConfigLoweringError::Backend(format!("payload serialization failed: {error}"))
            })?;

            let config_uuid = endpoint_id.internal().as_uuid();

            let encrypted_payload = encrypt_payload(
                &payload_bytes,
                realm_key,
                AeadAadInputs {
                    realm: &realm,
                    tenant: tenant.internal().as_uuid(),
                    inst,
                    step: step_seq,
                    config_uuid,
                    kid,
                },
                &mut OsRng,
            )
            .map_err(|error| {
                ConfigLoweringError::Backend(format!(
                    "payload encryption failed for '{name}': {error}"
                ))
            })?;

            let encrypted_payload_bytes =
                encrypted_payload.into_inner().to_vec().map_err(|error| {
                    ConfigLoweringError::Backend(format!(
                        "COSE_Encrypt0 encoding failed for '{name}': {error}"
                    ))
                })?;
            let payload_hash = Sha256::of(&encrypted_payload_bytes);
            let now = UnixMillis::now();
            let exp = expiry(now, self.token_lifetime_ms)?;

            let claims = ConnectorTokenClaims {
                iss: self.issuer.clone(),
                exp,
                iat: now,
                kid: kid.to_string(),
                realm: realm.clone(),
                tenant: tenant.internal().as_uuid(),
                inst,
                step: step_seq,
                config_uuid,
                payload_hash,
            };

            let token = self.signing_key.mint_token(&claims).map_err(|error| {
                ConfigLoweringError::Backend(format!("token minting failed for '{name}': {error}"))
            })?;
            let token_bytes = token.into_inner().to_vec().map_err(|error| {
                ConfigLoweringError::Backend(format!(
                    "COSE_Sign1 encoding failed for '{name}': {error}"
                ))
            })?;

            let token_hex = hex::encode(token_bytes);
            let payload_hex = hex::encode(encrypted_payload_bytes);

            let connector_host = format!("{realm}.connector.{}", self.connector_domain_suffix);

            let http_endpoint = json!({
                "method": "post",
                "url_template": self.connector_router_url,
                "headers": {
                    "Host": connector_host,
                    "Authorization": format!("Bearer {token_hex}"),
                    "X-Encrypted-Payload": payload_hex,
                    "Content-Type": "application/json"
                },
                "request_body_type": "json",
                "response_body_type": "json",
                "response_max_bytes": 10_485_760,
            });

            endpoints.insert(name.clone(), http_endpoint);
        }

        Ok(json!({ "endpoints": endpoints }))
    }
}

fn expiry(now: UnixMillis, lifetime_ms: u64) -> Result<UnixMillis, ConfigLoweringError> {
    let lifetime = i64::try_from(lifetime_ms).map_err(|error| {
        ConfigLoweringError::InvalidConfig(format!(
            "lowerer token lifetime does not fit in i64 milliseconds: {error}"
        ))
    })?;
    now.as_i64()
        .checked_add(lifetime)
        .map(UnixMillis)
        .ok_or_else(|| {
            ConfigLoweringError::InvalidConfig(
                "lowerer token expiry timestamp overflowed i64 milliseconds".to_string(),
            )
        })
}
