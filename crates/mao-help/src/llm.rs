//! The answer step: the best pieces for a question, handed to a language model with the instruction to answer from
//! them and nothing else. The model is whatever the player has set up behind an address: a program on their own machine
//! that speaks the OpenAI chat protocol (ollama, llama.cpp's server, LM Studio and the like), or Anthropic's API.
//! Nothing is sent anywhere until an endpoint is set, and a key is never kept in the settings: the settings name the
//! environment variable that holds it.

use serde_json::{json, Value};

use crate::fetch::Http;
use crate::index::Hit;

/// Which protocol the endpoint speaks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Api {
    /// `POST <endpoint>` with `{model, messages}`; the answer in `choices[0].message.content`.
    #[default]
    OpenAi,
    /// Anthropic's Messages API: `{model, max_tokens, system, messages}`; the answer in `content[..].text`.
    Anthropic,
}

/// The model's settings (`[help]` in `mao.ini`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    pub api: Api,
    /// The whole address requests go to (`http://localhost:11434/v1/chat/completions`,
    /// `https://api.anthropic.com/v1/messages`). Empty: no model, search only.
    pub endpoint: String,
    pub model: String,
    /// The environment variable that holds the key, when the endpoint wants one.
    pub key_env: String,
    /// The most characters of sources put before the model (a small model's context is small).
    pub context_chars: usize,
}

impl Default for Model {
    fn default() -> Model {
        Model { api: Api::OpenAi, endpoint: String::new(), model: String::new(), key_env: String::new(), context_chars: 6000 }
    }
}

impl Model {
    pub fn configured(&self) -> bool {
        !self.endpoint.trim().is_empty() && !self.model.trim().is_empty()
    }
}

/// What the model is told it is doing.
pub const SYSTEM: &str = "You help a player of the online game Anarchy Online. Answer the player's question using only the numbered sources given with it. If the sources do not hold the answer, say that they do not; do not guess and do not use other knowledge. Be brief and practical. After each statement name the source it comes from by its number in square brackets, like [2].";

/// The question with its sources, as the model gets it; the sources that fitted are the first `n` of `hits`.
pub fn prompt(question: &str, hits: &[Hit], context_chars: usize) -> (String, usize) {
    let mut sources = String::new();
    let mut used = 0;
    for (n, hit) in hits.iter().enumerate() {
        let heading = if hit.heading.is_empty() || hit.heading == hit.title { String::new() } else { format!(" ({})", hit.heading) };
        let entry = format!("[{}] {}{}\n{}\n\n", n + 1, hit.title, heading, hit.text);
        if used > 0 && sources.chars().count() + entry.chars().count() > context_chars {
            break;
        }
        sources += &entry;
        used += 1;
    }
    (format!("Sources:\n\n{sources}Question: {}", question.trim()), used)
}

/// The request for a prompt: the headers and the body.
pub fn request(model: &Model, key: Option<&str>, user: &str) -> (Vec<(String, String)>, String) {
    let key = key.map(str::trim).filter(|k| !k.is_empty());
    match model.api {
        Api::OpenAi => {
            let headers = key.map(|k| vec![("Authorization".to_string(), format!("Bearer {k}"))]).unwrap_or_default();
            let body = json!({
                "model": model.model,
                "messages": [{"role": "system", "content": SYSTEM}, {"role": "user", "content": user}],
                "temperature": 0.2,
                "stream": false,
            });
            (headers, body.to_string())
        }
        Api::Anthropic => {
            let mut headers = vec![("anthropic-version".to_string(), "2023-06-01".to_string())];
            if let Some(k) = key {
                headers.push(("x-api-key".to_string(), k.to_string()));
            }
            let body = json!({
                "model": model.model,
                "max_tokens": 700,
                "system": SYSTEM,
                "messages": [{"role": "user", "content": user}],
            });
            (headers, body.to_string())
        }
    }
}

/// The answer's text out of the endpoint's reply, or what the endpoint said went wrong.
pub fn reply_text(api: Api, body: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(body).map_err(|_| format!("the endpoint did not answer in JSON: {}", body.chars().take(160).collect::<String>()))?;
    if let Some(error) = value.get("error") {
        let message = error.get("message").and_then(Value::as_str).or_else(|| error.as_str()).unwrap_or("an error without a message");
        return Err(format!("the endpoint says: {message}"));
    }
    let text = match api {
        Api::OpenAi => value.pointer("/choices/0/message/content").and_then(Value::as_str).map(str::to_string),
        Api::Anthropic => value.get("content").and_then(Value::as_array).map(|parts| parts.iter().filter_map(|p| p.get("text").and_then(Value::as_str)).collect::<Vec<_>>().join("")),
    };
    text.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).ok_or_else(|| "the endpoint's answer has no text".to_string())
}

/// An answer and how many of the hits it was given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    pub text: String,
    pub sources: usize,
}

/// Asks the model. `key` is the key's value (read from the variable the settings name, by the caller).
pub fn answer(http: &mut dyn Http, model: &Model, key: Option<&str>, question: &str, hits: &[Hit]) -> Result<Answer, String> {
    if !model.configured() {
        return Err("no model is set up (the [help] section of mao.ini)".into());
    }
    if hits.is_empty() {
        return Err("nothing in the index matches the question".into());
    }
    let (user, sources) = prompt(question, hits, model.context_chars);
    let (headers, body) = request(model, key, &user);
    let reply = http.post_json(model.endpoint.trim(), &headers, &body)?;
    let text = reply_text(model.api, &reply.text());
    match text {
        Ok(text) => Ok(Answer { text, sources }),
        Err(e) if reply.ok() => Err(e),
        Err(e) => Err(format!("{e} (status {})", reply.status)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crawl::tests::Fake;

    fn hit(n: usize, text: &str) -> Hit {
        Hit { source: "test".into(), url: format!("u/{n}"), title: format!("Page {n}"), heading: if n == 2 { "Soil".into() } else { String::new() }, text: text.into(), snippet: String::new(), score: 1.0 }
    }

    #[test]
    fn the_sources_are_numbered_and_fit_the_context() {
        let hits = [hit(1, "Cold loam."), hit(2, "Dig twice."), hit(3, &"x".repeat(500))];
        let (text, used) = prompt("  what soil? ", &hits, 80);
        assert_eq!(used, 2);
        assert_eq!(text, "Sources:\n\n[1] Page 1\nCold loam.\n\n[2] Page 2 (Soil)\nDig twice.\n\nQuestion: what soil?");
        // (the first source is given even when it alone is longer than the context)
        assert_eq!(prompt("q", &[hit(3, &"x".repeat(500))], 80).1, 1);
    }

    #[test]
    fn an_openai_endpoint_is_asked_and_read() {
        let model = Model { endpoint: "http://localhost:11434/v1/chat/completions".into(), model: "small".into(), ..Default::default() };
        let mut http = Fake::default().page(&model.endpoint, 200, r#"{"choices":[{"message":{"role":"assistant","content":" Cold loam [1]. "}}]}"#);
        let got = answer(&mut http, &model, None, "what soil?", &[hit(1, "Cold loam.")]).unwrap();
        assert_eq!(got, Answer { text: "Cold loam [1].".into(), sources: 1 });
        let (url, headers, body) = &http.posted[0];
        assert_eq!(url, &model.endpoint);
        assert!(headers.is_empty());
        let sent: Value = serde_json::from_str(body).unwrap();
        assert_eq!(sent["model"], "small");
        assert_eq!(sent["stream"], false);
        assert_eq!(sent["messages"][0]["role"], "system");
        assert!(sent["messages"][1]["content"].as_str().unwrap().ends_with("Question: what soil?"));
        // (a key goes into the header, not the body)
        let (headers, body) = request(&model, Some(" secret "), "q");
        assert_eq!(headers, [("Authorization".to_string(), "Bearer secret".to_string())]);
        assert!(!body.contains("secret"));
    }

    #[test]
    fn an_anthropic_endpoint_is_asked_and_read() {
        let model = Model { api: Api::Anthropic, endpoint: "https://api.anthropic.com/v1/messages".into(), model: "claude-haiku-4-5".into(), key_env: "K".into(), ..Default::default() };
        let (headers, body) = request(&model, Some("secret"), "q");
        assert_eq!(headers, [("anthropic-version".to_string(), "2023-06-01".to_string()), ("x-api-key".to_string(), "secret".to_string())]);
        let sent: Value = serde_json::from_str(&body).unwrap();
        assert_eq!((sent["model"].as_str(), sent["system"].as_str(), sent["messages"][0]["content"].as_str()), (Some("claude-haiku-4-5"), Some(SYSTEM), Some("q")));
        assert!(sent["max_tokens"].as_u64().unwrap() > 0);
        assert_eq!(reply_text(Api::Anthropic, r#"{"content":[{"type":"text","text":"Dig "},{"type":"text","text":"twice [2]."}]}"#).unwrap(), "Dig twice [2].");
    }

    #[test]
    fn what_goes_wrong_is_said() {
        let model = Model { endpoint: "http://localhost:1/x".into(), model: "m".into(), ..Default::default() };
        assert!(answer(&mut Fake::default(), &Model::default(), None, "q", &[hit(1, "t")]).unwrap_err().contains("no model"));
        assert!(answer(&mut Fake::default(), &model, None, "q", &[]).unwrap_err().contains("nothing"));
        let mut http = Fake::default().page(&model.endpoint, 404, r#"{"error":{"message":"model 'm' not found"}}"#);
        assert_eq!(answer(&mut http, &model, None, "q", &[hit(1, "t")]).unwrap_err(), "the endpoint says: model 'm' not found (status 404)");
        let mut http = Fake::default().page(&model.endpoint, 502, "<html>Bad Gateway</html>");
        assert!(answer(&mut http, &model, None, "q", &[hit(1, "t")]).unwrap_err().contains("status 502"));
        assert!(reply_text(Api::OpenAi, r#"{"choices":[]}"#).is_err());
    }
}
