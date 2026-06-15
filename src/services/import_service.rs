use std::sync::LazyLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Shared HTTP client for outbound Anthropic calls. reqwest has no total request
/// timeout by default, so a stalled upstream would hang `/api/import/models` or
/// `/api/import/parse` indefinitely. Built once and reused so the connection pool
/// is shared across requests. `parse_note` can be slow on a long completion, so
/// the total timeout is generous (60s) while the connect timeout stays tight.
static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build Anthropic HTTP client")
});

/// Parsed roll data extracted from freeform note text by the LLM.
#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedRoll {
    pub roll_id: String,
    pub film_stock_guess: Option<String>,
    pub camera_prefix_guess: Option<String>,
    pub lens_guess: Option<String>,
    pub frame_count: Option<i32>,
    pub date_loaded: Option<String>,
    pub date_finished: Option<String>,
    pub notes: Option<String>,
    pub shots: Vec<ParsedShot>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedShot {
    pub frame_number: String,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub date: Option<String>,
    pub focal_length: Option<String>,
    pub location: Option<String>,
    pub notes: Option<String>,
}

// --- Claude API types (private) ---

#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

#[derive(Deserialize)]
struct ApiErrorResponse {
    error: ApiError,
}

#[derive(Deserialize)]
struct ApiError {
    message: String,
}

// --- Models list types ---

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
}

#[derive(Deserialize)]
struct ModelsListResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
    display_name: String,
}

const SYSTEM_PROMPT: &str = r#"You are a structured data extraction assistant for a film photography catalog app. The user will paste freeform notes about a roll of film and its shots. Extract structured JSON from these notes.

Output ONLY valid JSON matching this schema (no markdown fences, no explanation):
{
  "roll_id": "string — the roll identifier (e.g. NFE-1, M67-24, I45-7)",
  "film_stock_guess": "string|null — film stock name (e.g. 'Portra 400', 'Ilford Delta 400')",
  "camera_prefix_guess": "string|null — camera prefix from roll ID (e.g. 'M67' from 'M67-24', 'NFE' from 'NFE-1')",
  "lens_guess": "string|null — default lens for the entire roll if mentioned (e.g. 'Nikkor 50mm f/1.8', '50mm 1.8 E'). Look for phrases like 'All N on [lens]' or a single lens at the roll level.",
  "frame_count": "number|null — total frames if mentioned (e.g. 'x36' = 36)",
  "date_loaded": "string|null — ISO date YYYY-MM-DD if a load/start date is mentioned",
  "date_finished": "string|null — ISO date YYYY-MM-DD if a finish date is mentioned",
  "notes": "string|null — any roll-level notes not captured in other fields",
  "shots": [
    {
      "frame_number": "string — frame number or holder notation (e.g. '1', '0', 'H1', '21-24' for ranges)",
      "aperture": "string|null — aperture value (e.g. 'f/5.6', 'f11')",
      "shutter_speed": "string|null — shutter speed (e.g. '1/125', '1/8', '1s')",
      "date": "string|null — ISO date YYYY-MM-DD",
      "focal_length": "string|null — focal length used (e.g. '50mm', '65mm')",
      "location": "string|null — location or place name",
      "notes": "string|null — any additional notes for this shot"
    }
  ]
}

Rules:
- If a shot says "Same" for any value, propagate the previous shot's value for that field.
- Frame ranges like "21-24" should be a single shot entry with frame_number "21-24".
- Normalize dates: "1/10" in context of year 2017 → "2017-01-10"; "5/16/21" → "2021-05-16"; "2/28/2021" → "2021-02-28". If only month/day with no year context, use null for date.
- "?" means null for that field.
- "H1", "H2" are large format holder notations — use as frame_number.
- Time annotations like "10:40pm" or "7:27pm" should go in shot notes, not in date.
- Extract aperture from "fN" notation: "f11" → "f/11", "f5.6" → "f/5.6".
- If a single lens is mentioned for the entire roll (e.g. "All 36 on 50mm 1.8 E" or a lens on the header line), set "lens_guess" at the roll level. Do NOT duplicate this into per-shot focal_length.
- Per-shot focal_length should only be set when individual shots specify different lenses.
- If all shots use the same lens and it's stated at the roll level, set lens_guess and leave per-shot focal_length null."#;

/// Anthropic API base URL. The `*_at` methods below take this as a parameter so
/// integration tests can point them at a local mock server (the real reqwest /
/// serde path is then exercised, not a reimplementation).
const ANTHROPIC_API_BASE: &str = "https://api.anthropic.com";

pub struct ImportService;

impl ImportService {
    /// Fetch available models from the Anthropic API.
    pub async fn list_models(api_key: &str) -> Result<Vec<ModelInfo>, String> {
        Self::list_models_at(ANTHROPIC_API_BASE, api_key).await
    }

    /// Base-URL-injectable form of [`list_models`]. Production calls the wrapper
    /// above; tests pass a mock-server base URL. (Same seam pattern as the
    /// `spa` / `compression` lib modules.)
    pub async fn list_models_at(base_url: &str, api_key: &str) -> Result<Vec<ModelInfo>, String> {
        let response = HTTP_CLIENT
            .get(format!("{base_url}/v1/models"))
            .query(&[("limit", "1000")])
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .map_err(|e| format!("Failed to reach Anthropic API: {e}"))?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Models API error (HTTP {status}): {body}");

            if let Ok(err_resp) = serde_json::from_str::<ApiErrorResponse>(&body) {
                return Err(format!(
                    "API error ({}): {}",
                    status.as_u16(),
                    err_resp.error.message
                ));
            }
            return Err(format!("Anthropic API returned HTTP {}.", status.as_u16()));
        }

        let list: ModelsListResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse models response: {e}"))?;

        Ok(list
            .data
            .into_iter()
            .map(|entry| ModelInfo {
                id: entry.id,
                display_name: entry.display_name,
            })
            .collect())
    }

    pub async fn parse_note(
        api_key: &str,
        model: &str,
        note_text: &str,
    ) -> Result<ParsedRoll, String> {
        Self::parse_note_at(ANTHROPIC_API_BASE, api_key, model, note_text).await
    }

    /// Base-URL-injectable form of [`parse_note`] (see [`list_models_at`]).
    pub async fn parse_note_at(
        base_url: &str,
        api_key: &str,
        model: &str,
        note_text: &str,
    ) -> Result<ParsedRoll, String> {
        let request = MessagesRequest {
            model: model.to_string(),
            max_tokens: 4096,
            system: SYSTEM_PROMPT.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: note_text.to_string(),
            }],
        };

        let response = HTTP_CLIENT
            .post(format!("{base_url}/v1/messages"))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to reach Claude API: {e}"))?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Claude API error (HTTP {status}): {body}");

            if let Ok(err_resp) = serde_json::from_str::<ApiErrorResponse>(&body) {
                return Err(format!(
                    "Claude API error ({}): {}",
                    status.as_u16(),
                    err_resp.error.message
                ));
            }
            return Err(format!(
                "Claude API returned HTTP {}. Check the application logs for details.",
                status.as_u16()
            ));
        }

        let resp: MessagesResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Claude API response: {e}"))?;

        let text = resp
            .content
            .into_iter()
            .find_map(|block| block.text)
            .ok_or_else(|| "Claude API returned no text content".to_string())?;

        // Strip markdown code fences if present
        let trimmed = text.trim();
        let after_prefix = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        let json_str = after_prefix
            .strip_suffix("```")
            .unwrap_or(after_prefix)
            .trim();

        serde_json::from_str::<ParsedRoll>(json_str).map_err(|e| {
            tracing::error!("Failed to parse LLM output as JSON: {e}\nRaw output: {text}");
            format!("Failed to parse AI response as structured data: {e}")
        })
    }
}
