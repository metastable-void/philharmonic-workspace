use std::collections::HashMap;

use async_trait::async_trait;
use coset::CborSerializable;
use philharmonic::connector_client::{AeadAadInputs, LowererSigningKey, encrypt_payload};
use philharmonic::connector_common::{ConnectorTokenClaims, RealmPublicKey};
use philharmonic::types::{EntityId, JsonValue, Sha256, UnixMillis, Uuid};
use philharmonic::workflow::{
    ConfigLowerer, ConfigLoweringError, SubjectContext, WorkflowInstance,
};
use rand_core::OsRng;
use serde_json::json;

pub struct ConnectorConfigLowerer {
    signing_key: LowererSigningKey,
    realm_keys: HashMap<String, RealmPublicKey>,
    issuer: String,
    token_lifetime_ms: u64,
}

impl ConnectorConfigLowerer {
    pub fn new(
        signing_key: LowererSigningKey,
        realm_keys: HashMap<String, RealmPublicKey>,
        issuer: String,
        token_lifetime_ms: u64,
    ) -> Self {
        Self {
            signing_key,
            realm_keys,
            issuer,
            token_lifetime_ms,
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
        let object = abstract_config.as_object().ok_or_else(|| {
            ConfigLoweringError::InvalidConfig("abstract config must be a JSON object".to_string())
        })?;

        let realm = required_string(object, "realm")?.to_owned();
        let config_uuid = parse_uuid(required_string(object, "config_uuid")?, "config_uuid")?;
        let implementation = required_string(object, "impl")?.to_owned();
        let config = object
            .get("config")
            .filter(|value| value.is_object())
            .cloned()
            .ok_or_else(|| {
                ConfigLoweringError::InvalidConfig(
                    "abstract config field 'config' must be a JSON object".to_string(),
                )
            })?;

        let realm_key = self.realm_keys.get(&realm).ok_or_else(|| {
            ConfigLoweringError::InvalidConfig(format!(
                "no realm public key configured for realm '{realm}'"
            ))
        })?;

        let tenant = subject.tenant_id.internal().as_uuid();
        let inst = instance_id.internal().as_uuid();
        let kid = self.signing_key.kid();

        let plaintext = serde_json::to_vec(&json!({
            "realm": realm,
            "impl": implementation,
            "config": config,
        }))
        .map_err(|error| {
            ConfigLoweringError::Backend(format!("payload serialization failed: {error}"))
        })?;

        let encrypted_payload = encrypt_payload(
            &plaintext,
            realm_key,
            AeadAadInputs {
                realm: &realm,
                tenant,
                inst,
                step: step_seq,
                config_uuid,
                kid,
            },
            &mut OsRng,
        )
        .map_err(|error| {
            ConfigLoweringError::Backend(format!("payload encryption failed: {error}"))
        })?;

        let encrypted_payload_bytes = encrypted_payload.into_inner().to_vec().map_err(|error| {
            ConfigLoweringError::Backend(format!("COSE_Encrypt0 encoding failed: {error}"))
        })?;
        let payload_hash = Sha256::of(&encrypted_payload_bytes);
        let now = UnixMillis::now();
        let exp = expiry(now, self.token_lifetime_ms)?;

        let claims = ConnectorTokenClaims {
            iss: self.issuer.clone(),
            exp,
            iat: now,
            kid: kid.to_string(),
            realm,
            tenant,
            inst,
            step: step_seq,
            config_uuid,
            payload_hash,
        };

        let token = self.signing_key.mint_token(&claims).map_err(|error| {
            ConfigLoweringError::Backend(format!("token minting failed: {error}"))
        })?;
        let token_bytes = token.into_inner().to_vec().map_err(|error| {
            ConfigLoweringError::Backend(format!("COSE_Sign1 encoding failed: {error}"))
        })?;

        Ok(json!({
            "token": hex::encode(token_bytes),
            "encrypted_payload": hex::encode(encrypted_payload_bytes),
        }))
    }
}

fn required_string<'a>(
    object: &'a serde_json::Map<String, JsonValue>,
    field: &'static str,
) -> Result<&'a str, ConfigLoweringError> {
    object
        .get(field)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| {
            ConfigLoweringError::InvalidConfig(format!(
                "abstract config field '{field}' must be a string"
            ))
        })
}

fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, ConfigLoweringError> {
    Uuid::parse_str(value).map_err(|error| {
        ConfigLoweringError::InvalidConfig(format!(
            "abstract config field '{field}' must be a UUID string: {error}"
        ))
    })
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
