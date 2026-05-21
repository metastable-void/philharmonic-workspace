export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonObject | JsonValue[];

export interface JsonObject {
  [key: string]: JsonValue;
}

export interface ChatConfigResponse {
  api_url: string;
  notify_instance_uuid: string;
}

export interface VersionResponse {
  version: string;
  git_commit_sha: string | null;
  virtualization: string;
}

export interface BrandingResponse {
  name: string;
  monogram: string;
}

export interface PaginatedResponse<T> {
  items: T[];
  next_cursor: string | null;
}

export interface StepRecord {
  step_record_id: string;
  step_seq: number;
  outcome: string;
  created_at: number;
  input: JsonValue;
  output: JsonValue | null;
  error: JsonValue | null;
  subject: JsonValue;
}

export interface ExecuteInstanceResponse {
  output: JsonValue;
  context: JsonValue;
  status: string;
  step_seq: number;
}

export interface MintEphemeralResponse {
  ephemeral_token: string;
  instance_id: string;
}

export interface ChatMessage {
  role: string;
  content: string;
  name?: string;
  date?: number;
  [extra: string]: JsonValue | string | undefined;
}

export class ApiRequestError extends Error {
  readonly status: number;
  readonly body: unknown;

  constructor(status: number, body: unknown) {
    super(errorMessage(status, body));
    this.name = "ApiRequestError";
    this.status = status;
    this.body = body;
  }
}

let runtimeConfig: ChatConfigResponse | null = null;

export function setRuntimeConfig(config: ChatConfigResponse): void {
  runtimeConfig = config;
}

export function currentConfig(): ChatConfigResponse | null {
  return runtimeConfig;
}

export async function fetchChatConfig(): Promise<ChatConfigResponse> {
  return localCall<ChatConfigResponse>("/config");
}

export async function fetchVersion(): Promise<VersionResponse> {
  return localCall<VersionResponse>("/version");
}

export async function signIn(agentToken: string): Promise<void> {
  await localCall<null>("/sign-in", {
    method: "POST",
    body: JSON.stringify({ agent_token: agentToken }),
  });
}

export async function mintEphemeral(agentToken: string): Promise<MintEphemeralResponse> {
  return localCall<MintEphemeralResponse>("/mint-ephemeral", { method: "POST" }, agentToken);
}

export async function fetchBranding(token: string): Promise<BrandingResponse> {
  return apiCall<BrandingResponse>("_meta/branding", token);
}

export async function fetchLatestStep(
  instanceId: string,
  token: string,
): Promise<StepRecord | null> {
  const response = await apiCall<PaginatedResponse<StepRecord>>(
    `workflows/instances/${instanceId}/steps?limit=1`,
    token,
  );
  return response.items[0] ?? null;
}

export async function executeInstance(
  instanceId: string,
  token: string,
  input: JsonValue,
): Promise<ExecuteInstanceResponse> {
  return apiCall<ExecuteInstanceResponse>(`workflows/instances/${instanceId}/execute`, token, {
    method: "POST",
    body: JSON.stringify({ input }),
  });
}

export async function apiCall<T>(
  path: string,
  token: string,
  options: RequestInit = {},
): Promise<T> {
  if (runtimeConfig === null) {
    throw new Error("chat config has not loaded");
  }
  const normalized = path.replace(/^\/+/, "");
  const apiPath = normalized.startsWith("v1/") ? normalized : `v1/${normalized}`;
  const url = `${runtimeConfig.api_url.replace(/\/+$/, "")}/${apiPath}`;
  return fetchJson<T>(url, requestOptions(options, token));
}

async function localCall<T>(path: string, options: RequestInit = {}, token = ""): Promise<T> {
  return fetchJson<T>(path, requestOptions(options, token));
}

function requestOptions(options: RequestInit, token = ""): RequestInit {
  const headers = new Headers(options.headers);
  const method = options.method?.toUpperCase();
  if (token.length > 0) {
    headers.set("Authorization", `Bearer ${token}`);
  }
  if ((method === "POST" || method === "PATCH") && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }
  return { ...options, headers };
}

async function fetchJson<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, options);
  const body = await responseBody(response);
  if (!response.ok) {
    throw new ApiRequestError(response.status, body);
  }
  return body as T;
}

async function responseBody(response: Response): Promise<unknown> {
  if (response.status === 204) {
    return null;
  }
  const text = await response.text();
  if (text.length === 0) {
    return null;
  }
  try {
    return JSON.parse(text) as unknown;
  } catch {
    return text;
  }
}

function errorMessage(status: number, body: unknown): string {
  if (isRecord(body)) {
    const error = body.error;
    if (isRecord(error) && typeof error.message === "string") {
      return `${status}: ${error.message}`;
    }
  }
  if (typeof body === "string" && body.length > 0) {
    return `${status}: ${body}`;
  }
  return `request failed with status ${status}`;
}

export function parseMessages(output: JsonValue | null): ChatMessage[] {
  const raw = messageArray(output);
  const messages: ChatMessage[] = [];
  for (const item of raw) {
    if (!isRecord(item)) {
      continue;
    }
    const { role, content, name, date } = item;
    if (typeof role !== "string" || typeof content !== "string") {
      continue;
    }
    messages.push({
      ...jsonRecord(item),
      role,
      content,
      name: typeof name === "string" ? name : undefined,
      date: typeof date === "number" ? date : undefined,
    });
  }
  return messages;
}

export function notifyInstances(output: JsonValue | null): string[] {
  if (!isRecord(output) || !Array.isArray(output.instances)) {
    return [];
  }
  return output.instances.filter((value): value is string => typeof value === "string");
}

function messageArray(output: JsonValue | null): JsonValue[] {
  if (Array.isArray(output)) {
    return output;
  }
  if (isRecord(output) && Array.isArray(output.messages)) {
    return output.messages;
  }
  return [];
}

function jsonRecord(value: Record<string, unknown>): Record<string, JsonValue> {
  const result: Record<string, JsonValue> = {};
  for (const [key, entry] of Object.entries(value)) {
    if (isJsonValue(entry)) {
      result[key] = entry;
    }
  }
  return result;
}

function isJsonValue(value: unknown): value is JsonValue {
  if (
    value === null ||
    typeof value === "string" ||
    typeof value === "number" ||
    typeof value === "boolean"
  ) {
    return true;
  }
  if (Array.isArray(value)) {
    return value.every(isJsonValue);
  }
  if (isRecord(value)) {
    return Object.values(value).every(isJsonValue);
  }
  return false;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
