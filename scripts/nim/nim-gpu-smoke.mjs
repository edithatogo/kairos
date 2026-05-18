const baseUrl = process.env.NVIDIA_NIM_BASE_URL || process.env.NIM_BASE_URL || "";
const apiKey = process.env.NVIDIA_NIM_API_KEY || process.env.NIM_API_KEY || "";
const modelHint = process.env.NVIDIA_NIM_MODEL || process.env.NIM_MODEL || "";

function normalizeBaseUrl(value) {
  return value.trim().replace(/\/+$/, "");
}

function toJson(value) {
  return JSON.stringify(value, null, 2);
}

async function fetchJson(url, init = {}) {
  const response = await fetch(url, {
    ...init,
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json",
      ...(init.headers || {})
    }
  });
  const text = await response.text();
  if (!response.ok) {
    throw new Error(`${response.status} ${response.statusText} from ${url}\n${text}`);
  }
  return text ? JSON.parse(text) : {};
}

if (!baseUrl || !apiKey) {
  console.log(
    toJson({
      status: "skipped",
      reason: "NVIDIA_NIM_BASE_URL and NVIDIA_NIM_API_KEY are required for this smoke"
    })
  );
  process.exit(0);
}

const root = normalizeBaseUrl(baseUrl);
const models = await fetchJson(`${root}/v1/models`);
const modelId = modelHint || models?.data?.[0]?.id || models?.data?.[0]?.name || "";

if (!modelId) {
  throw new Error(`No model ID returned by ${root}/v1/models`);
}

const completion = await fetchJson(`${root}/v1/chat/completions`, {
  method: "POST",
  body: JSON.stringify({
    model: modelId,
    messages: [
      {
        role: "user",
        content: "Reply with a single line that confirms the NIM smoke completed."
      }
    ],
    temperature: 0,
    max_tokens: 32
  })
});

const message = completion?.choices?.[0]?.message?.content?.trim() || "";
if (!message) {
  throw new Error("NIM chat completion returned no message content");
}

console.log(
  toJson({
    status: "ok",
    base_url: root,
    model_id: modelId,
    model_count: Array.isArray(models?.data) ? models.data.length : null,
    message
  })
);
