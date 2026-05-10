//! Background tokio task that runs embedding jobs for datasets.

use std::{sync::Arc, time::Duration};

use philharmonic::api::{ApiStore, EmbedDatasetCaps, EmbedDatasetDispatcher};
use philharmonic::policy::{
    CorpusItem, EmbeddingDataset, EmbeddingDatasetStatus, Sck, SourceItem, Tenant,
    TenantEndpointConfig, decode_source_items, encode_corpus, sck_decrypt,
};
use philharmonic::store::{
    ContentStoreExt, EntityRefValue, EntityStoreExt, RevisionInput, RevisionRow,
};
use philharmonic::types::{
    CanonicalJson, ContentHash, ContentValue, EntityId, Identity, JsonValue, ScalarValue, Sha256,
    Uuid,
};
use philharmonic::workflow::{ConfigLowerer, SubjectContext, SubjectKind, WorkflowInstance};
use serde_json::json;

use crate::executor::MechanicsWorkerExecutor;

const EMBED_SCRIPT_SRC: &str = include_str!("embed_script.js");
const EMBED_JOB_TIMEOUT: Duration = Duration::from_secs(1_800);
const DEFAULT_MAX_BATCH_SIZE: usize = 8;

#[derive(Clone)]
pub(crate) struct EmbedJobDispatcher {
    store: Arc<dyn ApiStore>,
    lowerer: Arc<dyn ConfigLowerer>,
    executor: Arc<MechanicsWorkerExecutor>,
    sck: Arc<Sck>,
    caps: EmbedDatasetCaps,
}

impl EmbedJobDispatcher {
    pub(crate) fn new(
        store: Arc<dyn ApiStore>,
        lowerer: Arc<dyn ConfigLowerer>,
        executor: Arc<MechanicsWorkerExecutor>,
        sck: Arc<Sck>,
        caps: EmbedDatasetCaps,
    ) -> Self {
        Self {
            store,
            lowerer,
            executor,
            sck,
            caps,
        }
    }
}

impl EmbedDatasetDispatcher for EmbedJobDispatcher {
    fn dispatch_embed_dataset(
        &self,
        dataset_id: EntityId<EmbeddingDataset>,
        tenant_id: EntityId<Tenant>,
    ) {
        let job = self.clone();
        // Design 16 v1 intentionally uses an in-process tokio task. If the
        // API server restarts mid-embed, the task is lost and the dataset is
        // left in `status=Embedding`; recovery is an admin resubmission.
        tokio::spawn(async move {
            if let Err(error) = job.run(dataset_id, tenant_id).await {
                tracing::warn!(
                    tenant = %tenant_id.internal().as_uuid(),
                    dataset_id = %dataset_id.public().as_uuid(),
                    error = %error,
                    "embed-job dispatch failed"
                );
            }
        });
    }
}

impl EmbedJobDispatcher {
    async fn run(
        &self,
        dataset_id: EntityId<EmbeddingDataset>,
        tenant_id: EntityId<Tenant>,
    ) -> Result<(), String> {
        let mut latest = latest_revision(&self.store, dataset_id).await?;

        // Security pre-review 2026-05-10 Finding 2 (defence-in-depth):
        // verify the dataset revision's stored tenant matches the
        // dispatcher-supplied `tenant_id` before any crypto-sensitive
        // operation (SCK decrypt of the endpoint config, lowerer call,
        // connector-token AAD construction). HTTP routes already check
        // tenant before dispatch, but this module crosses the SCK +
        // lowerer boundary and shouldn't take the caller's word for it
        // if the substrate disagrees.
        let stored_tenant = required_entity_ref(&latest, "tenant")?;
        if stored_tenant.target_entity_id != tenant_id.internal().as_uuid() {
            return Err(format!(
                "embed-job tenant mismatch: dataset {} stored tenant is {} \
                 but dispatcher was called with tenant {}",
                dataset_id.public().as_uuid(),
                stored_tenant.target_entity_id,
                tenant_id.internal().as_uuid(),
            ));
        }

        if status_from_revision(&latest)? == EmbeddingDatasetStatus::Created {
            self.append_status_revision(
                dataset_id,
                &latest,
                EmbeddingDatasetStatus::Embedding,
                None,
            )
            .await?;
            latest = latest_revision(&self.store, dataset_id).await?;
        }

        let source_items = load_source_items(&self.store, &latest).await?;
        let embed_endpoint_public = embed_endpoint_id(&self.store, &latest).await?;

        // Security pre-review 2026-05-10 Finding 3: revalidate the
        // endpoint at job-execution time. Routes check tenant + retire
        // + `implementation == "embed"` at create/update, but a queued
        // job can race retirement or implementation changes. Validate
        // before SCK decrypt + lowerer call.
        let (embed_endpoint_id, endpoint_revision) =
            validate_endpoint(&self.store, embed_endpoint_public, tenant_id).await?;
        let max_batch_size = self
            .read_max_batch_size(&endpoint_revision, &latest, tenant_id, embed_endpoint_id)
            .await?;

        let ephemeral_inst: EntityId<WorkflowInstance> = Identity {
            internal: Uuid::now_v7(),
            public: Uuid::new_v4(),
        }
        .typed()
        .expect("freshly-minted v7+v4 pair satisfies Identity::typed");
        let ephemeral_step: u64 = 0;

        // The lowerer reads only `subject.tenant_id`; embed jobs produce no
        // StepRecord. A proper `SubjectKind::System` is deferred to a
        // follow-up review, so this uses a stable principal-shaped system id.
        let subject = SubjectContext {
            kind: SubjectKind::Principal,
            id: format!("system:embed-job:{}", dataset_id.public().as_uuid()),
            tenant_id,
            authority_id: None,
            claims: JsonValue::Null,
        };
        let abstract_config = json!({ "embed": embed_endpoint_public.to_string() });
        let lowered = self
            .lowerer
            .lower(&abstract_config, ephemeral_inst, ephemeral_step, &subject)
            .await
            .map_err(|error| format!("embed endpoint lowering failed: {error}"))?;

        tracing::info!(
            tenant = %tenant_id.internal().as_uuid(),
            dataset_id = %dataset_id.public().as_uuid(),
            revision_id = latest.revision_seq,
            embed_endpoint_id = %embed_endpoint_public,
            config_uuid = %embed_endpoint_id.internal().as_uuid(),
            synthetic_inst = %ephemeral_inst.internal().as_uuid(),
            "embed-job lowerer dispatch"
        );

        let arg = serde_json::to_value(json!({
            "items": source_items,
            "max_batch_size": max_batch_size,
        }))
        .map_err(|error| format!("embed job argument serialization failed: {error}"))?;

        let result = self
            .executor
            .execute_with_run_timeout(EMBED_SCRIPT_SRC, &arg, &lowered, EMBED_JOB_TIMEOUT)
            .await;
        match result {
            Ok(value) => match parse_corpus_output(&value, &self.caps) {
                Ok(corpus) => {
                    self.append_status_revision(
                        dataset_id,
                        &latest,
                        EmbeddingDatasetStatus::Ready,
                        Some(corpus),
                    )
                    .await
                }
                Err(error) => {
                    self.append_status_revision(
                        dataset_id,
                        &latest,
                        EmbeddingDatasetStatus::Failed,
                        None,
                    )
                    .await?;
                    Err(error)
                }
            },
            Err(error) => {
                self.append_status_revision(
                    dataset_id,
                    &latest,
                    EmbeddingDatasetStatus::Failed,
                    None,
                )
                .await?;
                Err(format!("mechanics embed job failed: {error}"))
            }
        }
    }

    async fn append_status_revision(
        &self,
        dataset_id: EntityId<EmbeddingDataset>,
        previous: &RevisionRow,
        status: EmbeddingDatasetStatus,
        corpus: Option<Vec<CorpusItem>>,
    ) -> Result<(), String> {
        let next_seq = previous
            .revision_seq
            .checked_add(1)
            .ok_or_else(|| "embedding dataset revision sequence overflow".to_string())?;
        let mut revision = RevisionInput::new()
            .with_content(
                "display_name",
                required_content_hash(previous, "display_name")?,
            )
            .with_content(
                "source_items",
                required_content_hash(previous, "source_items")?,
            )
            .with_content(
                "embed_endpoint_id",
                required_content_hash(previous, "embed_endpoint_id")?,
            )
            .with_entity("tenant", required_entity_ref(previous, "tenant")?)
            .with_scalar("status", ScalarValue::I64(status.as_i64()))
            .with_scalar(
                "is_retired",
                ScalarValue::Bool(bool_scalar(previous, "is_retired")?),
            )
            .with_scalar(
                "item_count",
                ScalarValue::I64(i64_scalar(previous, "item_count")?),
            );

        if let Some(corpus) = corpus {
            let encoded = encode_corpus(&corpus)
                .map_err(|error| format!("corpus encoding failed: {error}"))?;
            // Security pre-review 2026-05-10 Finding 1: enforce the
            // post-encode corpus blob cap before the storage write so
            // an unbounded mechanics / upstream-provider response
            // cannot induce arbitrary content-store growth.
            if encoded.len() > self.caps.max_corpus_blob_bytes {
                return Err(format!(
                    "encoded corpus blob is {} bytes, exceeds cap of {} bytes",
                    encoded.len(),
                    self.caps.max_corpus_blob_bytes,
                ));
            }
            revision = revision.with_content("corpus", put_bytes(&self.store, &encoded).await?);
        } else if let Some(hash) = optional_content_hash(previous, "corpus") {
            revision = revision.with_content("corpus", hash);
        }

        self.store
            .append_revision_typed::<EmbeddingDataset>(dataset_id, next_seq, &revision)
            .await
            .map_err(|error| format!("embedding dataset revision append failed: {error}"))
    }

    async fn read_max_batch_size(
        &self,
        endpoint_revision: &RevisionRow,
        dataset_revision: &RevisionRow,
        tenant_id: EntityId<Tenant>,
        endpoint_id: EntityId<TenantEndpointConfig>,
    ) -> Result<usize, String> {
        let encrypted_hash = required_content_hash(endpoint_revision, "encrypted_config")?;
        let encrypted = self
            .store
            .get(encrypted_hash)
            .await
            .map_err(|error| format!("encrypted endpoint config read failed: {error}"))?
            .ok_or_else(|| "encrypted endpoint config blob missing".to_string())?;
        let key_version = i64_scalar(endpoint_revision, "key_version")?;
        let plaintext = sck_decrypt(
            &self.sck,
            encrypted.bytes(),
            tenant_id.internal().as_uuid(),
            endpoint_id.internal().as_uuid(),
            key_version,
        )
        .map_err(|error| format!("endpoint config decryption failed: {error}"))?;
        let config: JsonValue = serde_json::from_slice(plaintext.as_slice())
            .map_err(|error| format!("decrypted endpoint config is invalid JSON: {error}"))?;
        let value = config
            .get("max_batch_size")
            .and_then(JsonValue::as_u64)
            .unwrap_or(DEFAULT_MAX_BATCH_SIZE as u64);
        usize::try_from(value)
            .map_err(|error| format!("max_batch_size conversion failed: {error}"))
            .and_then(|size| {
                if size == 0 {
                    Err("max_batch_size must be greater than zero".to_string())
                } else {
                    Ok(size)
                }
            })
            .map_err(|error| {
                format!(
                    "invalid max_batch_size for dataset revision {}: {error}",
                    dataset_revision.revision_seq
                )
            })
    }
}

async fn latest_revision(
    store: &Arc<dyn ApiStore>,
    dataset_id: EntityId<EmbeddingDataset>,
) -> Result<RevisionRow, String> {
    store
        .get_latest_revision_typed::<EmbeddingDataset>(dataset_id)
        .await
        .map_err(|error| format!("embedding dataset revision lookup failed: {error}"))?
        .ok_or_else(|| "embedding dataset revision missing".to_string())
}

async fn latest_endpoint_revision(
    store: &Arc<dyn ApiStore>,
    endpoint_id: EntityId<TenantEndpointConfig>,
) -> Result<RevisionRow, String> {
    store
        .get_latest_revision_typed::<TenantEndpointConfig>(endpoint_id)
        .await
        .map_err(|error| format!("endpoint config revision lookup failed: {error}"))?
        .ok_or_else(|| "endpoint config revision missing".to_string())
}

async fn resolve_endpoint(
    store: &Arc<dyn ApiStore>,
    public: Uuid,
) -> Result<EntityId<TenantEndpointConfig>, String> {
    let identity = store
        .resolve_public(public)
        .await
        .map_err(|error| format!("endpoint config identity lookup failed: {error}"))?
        .ok_or_else(|| "endpoint config identity missing".to_string())?;
    identity
        .typed::<TenantEndpointConfig>()
        .map_err(|error| format!("endpoint config identity type mismatch: {error}"))
}

/// Resolve and validate an embed endpoint at job-execution time.
///
/// Per security pre-review 2026-05-10 Finding 3: an embed-job dispatched
/// some time after a dataset write can race the endpoint's retirement
/// or implementation-change. Routes validate at create/update time, but
/// the dispatcher must re-check before SCK decrypt + lowerer call.
///
/// Returns the resolved endpoint id and the validated revision so callers
/// don't need to re-fetch the revision for `read_max_batch_size`.
async fn validate_endpoint(
    store: &Arc<dyn ApiStore>,
    public: Uuid,
    tenant_id: EntityId<Tenant>,
) -> Result<(EntityId<TenantEndpointConfig>, RevisionRow), String> {
    let endpoint_id = resolve_endpoint(store, public).await?;
    let revision = latest_endpoint_revision(store, endpoint_id).await?;

    let endpoint_tenant = required_entity_ref(&revision, "tenant")?;
    if endpoint_tenant.target_entity_id != tenant_id.internal().as_uuid() {
        return Err(format!(
            "embed endpoint {public} belongs to a different tenant than the dataset"
        ));
    }
    if bool_scalar(&revision, "is_retired")? {
        return Err(format!("embed endpoint {public} is retired"));
    }
    let implementation_hash = required_content_hash(&revision, "implementation")?;
    let implementation_value = load_json(store, implementation_hash).await?;
    let implementation = implementation_value.as_str().ok_or_else(|| {
        format!("embed endpoint {public} implementation slot must be a JSON string")
    })?;
    if implementation != "embed" {
        return Err(format!(
            "embed endpoint {public} has implementation '{implementation}', expected 'embed'"
        ));
    }

    Ok((endpoint_id, revision))
}

async fn load_source_items(
    store: &Arc<dyn ApiStore>,
    revision: &RevisionRow,
) -> Result<Vec<SourceItem>, String> {
    let bytes = load_bytes(store, required_content_hash(revision, "source_items")?).await?;
    decode_source_items(bytes.bytes())
        .map_err(|error| format!("stored source items are invalid: {error}"))
}

async fn embed_endpoint_id(
    store: &Arc<dyn ApiStore>,
    revision: &RevisionRow,
) -> Result<Uuid, String> {
    match load_json(store, required_content_hash(revision, "embed_endpoint_id")?).await? {
        JsonValue::String(value) => value
            .parse()
            .map_err(|error| format!("stored endpoint UUID is invalid: {error}")),
        _ => Err("stored endpoint UUID has invalid JSON type".to_string()),
    }
}

async fn put_bytes(store: &Arc<dyn ApiStore>, bytes: &[u8]) -> Result<Sha256, String> {
    let content = ContentValue::new(bytes.to_vec());
    let hash = content.digest();
    store
        .put(&content)
        .await
        .map_err(|error| format!("content write failed: {error}"))?;
    Ok(hash)
}

async fn load_bytes(store: &Arc<dyn ApiStore>, hash: Sha256) -> Result<ContentValue, String> {
    store
        .get(hash)
        .await
        .map_err(|error| format!("content read failed: {error}"))?
        .ok_or_else(|| "content blob missing".to_string())
}

async fn load_json(store: &Arc<dyn ApiStore>, hash: Sha256) -> Result<JsonValue, String> {
    let typed_hash = ContentHash::<CanonicalJson>::from_digest_unchecked(hash);
    let canonical = store
        .get_typed::<CanonicalJson>(typed_hash)
        .await
        .map_err(|error| format!("JSON content read failed: {error}"))?
        .ok_or_else(|| "JSON content blob missing".to_string())?;
    canonical
        .to_deserializable()
        .map_err(|error| format!("stored JSON content is invalid: {error}"))
}

fn parse_corpus_output(
    value: &JsonValue,
    caps: &EmbedDatasetCaps,
) -> Result<Vec<CorpusItem>, String> {
    let output = value
        .get("output")
        .ok_or_else(|| "mechanics result missing output".to_string())?;
    let items = output
        .as_array()
        .ok_or_else(|| "mechanics output must be an array".to_string())?;
    // Security pre-review 2026-05-10 Finding 1: cap parse-time
    // allocation. Reject before allocating the corpus Vec.
    if items.len() > caps.max_corpus_items {
        return Err(format!(
            "mechanics output has {} items, exceeds corpus cap of {}",
            items.len(),
            caps.max_corpus_items,
        ));
    }
    let mut corpus = Vec::with_capacity(items.len());
    for item in items {
        let object = item
            .as_object()
            .ok_or_else(|| "corpus item must be an object".to_string())?;
        let id = object
            .get("id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| "corpus item id must be a string".to_string())?
            .to_string();
        let vector_values = object
            .get("vector")
            .and_then(JsonValue::as_array)
            .ok_or_else(|| format!("corpus item {id} vector must be an array"))?;
        // Security pre-review 2026-05-10 Finding 1: cap per-vector
        // allocation. Reject before allocating the f32 Vec.
        if vector_values.len() > caps.max_corpus_vector_dimension {
            return Err(format!(
                "corpus item {id} vector has {} dimensions, exceeds cap of {}",
                vector_values.len(),
                caps.max_corpus_vector_dimension,
            ));
        }
        let mut vector = Vec::with_capacity(vector_values.len());
        for value in vector_values {
            let number = value
                .as_f64()
                .ok_or_else(|| format!("corpus item {id} vector entry must be a number"))?;
            if !number.is_finite() || number < f64::from(f32::MIN) || number > f64::from(f32::MAX) {
                return Err(format!("corpus item {id} vector entry is not finite f32"));
            }
            vector.push(number as f32);
        }
        let payload = object.get("payload").cloned();
        corpus.push(CorpusItem {
            id,
            vector,
            payload,
        });
    }
    Ok(corpus)
}

fn required_content_hash(revision: &RevisionRow, attr: &'static str) -> Result<Sha256, String> {
    revision
        .content_attrs
        .get(attr)
        .copied()
        .ok_or_else(|| format!("missing content attribute {attr}"))
}

fn optional_content_hash(revision: &RevisionRow, attr: &'static str) -> Option<Sha256> {
    revision.content_attrs.get(attr).copied()
}

fn required_entity_ref(
    revision: &RevisionRow,
    attr: &'static str,
) -> Result<EntityRefValue, String> {
    revision
        .entity_attrs
        .get(attr)
        .copied()
        .ok_or_else(|| format!("missing entity attribute {attr}"))
}

fn bool_scalar(revision: &RevisionRow, attr: &'static str) -> Result<bool, String> {
    match revision.scalar_attrs.get(attr) {
        Some(ScalarValue::Bool(value)) => Ok(*value),
        Some(ScalarValue::I64(_)) => Err(format!("invalid scalar type for {attr}: expected bool")),
        None => Err(format!("missing scalar attribute {attr}")),
    }
}

fn i64_scalar(revision: &RevisionRow, attr: &'static str) -> Result<i64, String> {
    match revision.scalar_attrs.get(attr) {
        Some(ScalarValue::I64(value)) => Ok(*value),
        Some(ScalarValue::Bool(_)) => Err(format!("invalid scalar type for {attr}: expected i64")),
        None => Err(format!("missing scalar attribute {attr}")),
    }
}

fn status_from_revision(revision: &RevisionRow) -> Result<EmbeddingDatasetStatus, String> {
    let value = i64_scalar(revision, "status")?;
    EmbeddingDatasetStatus::try_from(value)
        .map_err(|_| format!("invalid embedding dataset status {value}"))
}
