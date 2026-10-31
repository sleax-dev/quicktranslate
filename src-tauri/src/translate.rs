use reqwest::blocking::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
struct MyMemoryResponse {
    #[serde(rename = "responseData")]
    response_data: ResponseData,
    #[serde(rename = "responseStatus")]
    response_status: i32,
    #[serde(rename = "responseDetails", default)]
    response_details: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseData {
    #[serde(rename = "translatedText")]
    translated_text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeepLResponse {
    translations: Vec<DeepLTranslation>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeepLTranslation {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterMessage,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenRouterMessage {
    content: String,
}

static CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

pub fn translate_text(
    text: &str,
    source_lang: &str,
    target_lang: &str,
    api_provider: &str,
    api_key: &str,
    translation_type: &str,
) -> Result<String, String> {
    if text.trim().is_empty() {
        return Err("Çevrilecek metin boş".to_string());
    }

    match api_provider {
        "deepL" => translate_deepl(text, source_lang, target_lang, api_key, translation_type),
        "openrouter" => translate_openrouter(text, source_lang, target_lang, api_key, translation_type),
        _ => translate_mymemory(text, source_lang, target_lang),
    }
}

/// Sends the request and returns the body, or an error with the status and body.
fn send(request: RequestBuilder, timeout_secs: u64, provider: &str) -> Result<String, String> {
    let response = request
        .timeout(Duration::from_secs(timeout_secs))
        .send()
        .map_err(|e| format!("{provider} istek hatası: {e}"))?;
    let status = response.status();
    let body = response.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!("{provider} API hatası ({status}): {body}"));
    }
    Ok(body)
}

fn translate_mymemory(text: &str, source_lang: &str, target_lang: &str) -> Result<String, String> {
    let source = if source_lang == "auto" || source_lang.is_empty() { "autodetect" } else { source_lang };
    let lang_pair = format!("{source}|{target_lang}");
    let request = CLIENT
        .get("https://api.mymemory.translated.net/get")
        .query(&[("q", text), ("langpair", lang_pair.as_str())]);
    let body = send(request, 15, "MyMemory")?;

    let result: MyMemoryResponse =
        serde_json::from_str(&body).map_err(|e| format!("Geçersiz API yanıtı: {e} - {body}"))?;
    if result.response_status != 200 {
        return Err(format!("Çeviri API hatası ({}): {:?}", result.response_status, result.response_details));
    }
    Ok(result.response_data.translated_text)
}

fn translate_deepl(
    text: &str,
    source_lang: &str,
    target_lang: &str,
    api_key: &str,
    translation_type: &str,
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("DeepL API anahtarı gerekli".to_string());
    }

    // Free-plan keys end in ":fx" and use their own host.
    let url = if api_key.ends_with(":fx") {
        "https://api-free.deepl.com/v2/translate"
    } else {
        "https://api.deepl.com/v2/translate"
    };
    // DeepL wants a regional variant for English and Portuguese targets.
    let target = match target_lang {
        "en" => "EN-US".to_string(),
        "pt" => "PT-BR".to_string(),
        other => other.to_uppercase(),
    };
    let formality = if translation_type == "formal" { "prefer_more" } else { "prefer_less" };

    let mut params = vec![("text", text.to_string()), ("target_lang", target), ("formality", formality.to_string())];
    // Leaving source_lang out lets DeepL detect it; "auto" is not a value it accepts.
    if source_lang != "auto" && !source_lang.is_empty() {
        params.push(("source_lang", source_lang.to_uppercase()));
    }

    let request = CLIENT.post(url).header("Authorization", format!("DeepL-Auth-Key {api_key}")).form(&params);
    let body = send(request, 20, "DeepL")?;

    let result: DeepLResponse =
        serde_json::from_str(&body).map_err(|e| format!("DeepL JSON parse hatası: {e} - Body: {body}"))?;
    result.translations.into_iter().next().map(|t| t.text).ok_or_else(|| "DeepL yanıtı boş".to_string())
}

fn translate_openrouter(
    text: &str,
    source_lang: &str,
    target_lang: &str,
    api_key: &str,
    translation_type: &str,
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("OpenRouter API anahtarı gerekli".to_string());
    }

    let style_instruction = if translation_type == "formal" {
        "Use formal, professional language suitable for business correspondence."
    } else {
        "Use casual, conversational language suitable for everyday communication."
    };
    let prompt = format!(
        "Translate the following text from {} to {}. {}\nReply with the translation only: no quotes, notes or explanations.\n\nText to translate:\n{}",
        lang_code_to_name(source_lang),
        lang_code_to_name(target_lang),
        style_instruction,
        text
    );
    let request_body = serde_json::json!({
        // OpenRouter picks a suitable current model, so this never points at a retired one.
        "model": "openrouter/auto",
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 2000
    });

    let request = CLIENT
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&request_body);
    let body = send(request, 30, "OpenRouter")?;

    let result: OpenRouterResponse =
        serde_json::from_str(&body).map_err(|e| format!("OpenRouter JSON parse hatası: {e} - Body: {body}"))?;
    result
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content.trim().to_string())
        .ok_or_else(|| "OpenRouter yanıtı boş".to_string())
}

fn lang_code_to_name(code: &str) -> &str {
    match code {
        "auto" | "autodetect" => "the detected language",
        "tr" => "Turkish",
        "en" => "English",
        "de" => "German",
        "fr" => "French",
        "es" => "Spanish",
        "it" => "Italian",
        "pt" => "Portuguese",
        "ru" => "Russian",
        "ar" => "Arabic",
        "ja" => "Japanese",
        "ko" => "Korean",
        "zh" => "Chinese",
        "hi" => "Hindi",
        "nl" => "Dutch",
        "pl" => "Polish",
        "sv" => "Swedish",
        "da" => "Danish",
        "no" => "Norwegian",
        "fi" => "Finnish",
        "el" => "Greek",
        "he" => "Hebrew",
        "th" => "Thai",
        "vi" => "Vietnamese",
        "id" => "Indonesian",
        "ms" => "Malay",
        _ => code,
    }
}
