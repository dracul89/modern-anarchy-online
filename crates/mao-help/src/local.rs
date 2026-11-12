//! The model inside the client: a small language model that runs on the player's own processor, with no server and no
//! account, and only ever sees the question and the pieces of the player's index that were found for it.
//!
//! The model is a quantized GGUF file of the Qwen2 family (Qwen2.5 Instruct in its 1.5B or 0.5B size; Apache-2.0,
//! from the Qwen team's own repositories) and its `tokenizer.json`, kept on the player's machine: [`MODELS`] says
//! which files, where they come from and what they have to hash to. Nothing of a model is in the repository or in a
//! package; the client fetches the two files when the player asks it to (`fetch_model`), and a file that does not
//! hash to what is written here is not used.
//!
//! The running is `candle`'s (Hugging Face's inference library, in Rust: no other program to install). The prompt is
//! the family's chat form (`<|im_start|>role ... <|im_end|>`), the answer is the likeliest token each time (no dice:
//! the same question over the same sources gives the same answer), with a light penalty on repeating, until the
//! model ends its turn or the answer is as long as allowed.

use std::path::{Path, PathBuf};

use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_qwen2::ModelWeights;
use tokenizers::Tokenizer;

/// A model the client knows how to get and run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Known {
    /// What the settings call it.
    pub name: &'static str,
    pub title: &'static str,
    /// Where the weights come from, their size in bytes and their SHA-256.
    pub weights_url: &'static str,
    pub weights_bytes: u64,
    pub weights_sha256: &'static str,
    pub tokenizer_url: &'static str,
    /// The most characters of sources put before it (the smaller model loses the thread sooner).
    pub context_chars: usize,
}

/// The models, the default first. Sizes and hashes as Hugging Face's API gave them on 2026-11-12. (The family's 0.5B
/// size was tried the same day and left out: asked what its source did not say, it made an answer up, where this one
/// says that the sources do not say.)
pub const MODELS: &[Known] = &[
    Known {
        name: "qwen2.5-1.5b",
        title: "Qwen2.5 1.5B Instruct (1.1 GB)",
        weights_url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf",
        weights_bytes: 1_117_320_736,
        weights_sha256: "6a1a2eb6d15622bf3c96857206351ba97e1af16c30d7a74ee38970e434e9407e",
        tokenizer_url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct/resolve/main/tokenizer.json",
        context_chars: 5000,
    },
];

/// A known model by its name; the default for a name that is none's.
pub fn known(name: &str) -> &'static Known {
    MODELS.iter().find(|m| m.name.eq_ignore_ascii_case(name.trim())).unwrap_or(&MODELS[0])
}

impl Known {
    /// The model's folder under the assistant's (`models/<name>`), and its two files there.
    pub fn dir(&self, base: &Path) -> PathBuf {
        base.join("models").join(self.name)
    }

    pub fn weights(&self, base: &Path) -> PathBuf {
        self.dir(base).join("model.gguf")
    }

    pub fn tokenizer(&self, base: &Path) -> PathBuf {
        self.dir(base).join("tokenizer.json")
    }

    /// Whether both files are there, the weights at their full size (their hash is checked when they are fetched).
    pub fn present(&self, base: &Path) -> bool {
        std::fs::metadata(self.weights(base)).is_ok_and(|m| m.len() == self.weights_bytes) && self.tokenizer(base).is_file()
    }
}

/// The chat form of the Qwen2 family: the system's words, the user's, and the assistant's turn left open.
pub fn chat(system: &str, user: &str) -> String {
    format!("<|im_start|>system\n{system}<|im_end|>\n<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n")
}

/// The token that ends a turn.
const END: &str = "<|im_end|>";
/// How hard a token already said is pushed down, and over how many of the last tokens.
const REPEAT_PENALTY: f32 = 1.1;
const REPEAT_WINDOW: usize = 64;

/// A model loaded and ready.
pub struct Local {
    model: ModelWeights,
    tokenizer: Tokenizer,
    end: u32,
    /// The threads the arithmetic runs on: its own few, so that a game drawing its frames keeps the rest.
    pool: rayon::ThreadPool,
}

/// How many threads the model works with on a machine of `cores`: half of them, at least two and at most eight
/// (more gain little for a model this size, and the game wants the others).
pub fn threads_for(cores: usize) -> usize {
    (cores / 2).clamp(2, 8)
}

impl Local {
    /// Reads the weights and the tokenizer.
    pub fn load(weights: &Path, tokenizer: &Path) -> Result<Local, String> {
        let mut file = std::fs::File::open(weights).map_err(|e| format!("{}: {e}", weights.display()))?;
        let content = gguf_file::Content::read(&mut file).map_err(|e| format!("{}: not a model file this client reads ({e})", weights.display()))?;
        let model = ModelWeights::from_gguf(content, &mut file, &Device::Cpu).map_err(|e| format!("{}: {e}", weights.display()))?;
        let tokenizer = Tokenizer::from_file(tokenizer).map_err(|e| format!("{}: {e}", tokenizer.display()))?;
        let end = tokenizer.token_to_id(END).ok_or("the tokenizer has no end-of-turn token")?;
        let threads = threads_for(std::thread::available_parallelism().map_or(4, |n| n.get()));
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).thread_name(|n| format!("help-model-{n}")).build().map_err(|e| e.to_string())?;
        Ok(Local { model, tokenizer, end, pool })
    }

    /// How many tokens a text is.
    pub fn tokens(&self, text: &str) -> usize {
        self.tokenizer.encode(text, false).map_or(0, |e| e.len())
    }

    /// Answers a prompt (as [`chat`] makes it) with at most `limit` tokens. `said` gets the answer so far each time it
    /// grows and may stop it by saying false.
    pub fn generate(&mut self, prompt: &str, limit: usize, said: &mut (dyn FnMut(&str) -> bool + Send)) -> Result<String, String> {
        // (inside the model's own threads: candle's loops run on whichever pool they are called in)
        let Local { model, tokenizer, end, pool } = self;
        pool.install(|| generate_in(model, tokenizer, *end, prompt, limit, said))
    }
}

fn generate_in(model: &mut ModelWeights, tokenizer: &Tokenizer, end: u32, prompt: &str, limit: usize, said: &mut (dyn FnMut(&str) -> bool + Send)) -> Result<String, String> {
    {
        let fail = |e: candle_core::Error| e.to_string();
        let prompt_tokens = tokenizer.encode(prompt, false).map_err(|e| e.to_string())?.get_ids().to_vec();
        if prompt_tokens.is_empty() {
            return Err("an empty prompt".into());
        }
        model.clear_kv_cache();
        let mut pick = LogitsProcessor::from_sampling(0, Sampling::ArgMax);
        // (the whole prompt in one pass, then a token at a time)
        let input = Tensor::new(prompt_tokens.as_slice(), &Device::Cpu).and_then(|t| t.unsqueeze(0)).map_err(fail)?;
        let logits = model.forward(&input, 0).and_then(|l| l.squeeze(0)).map_err(fail)?;
        let mut next = pick.sample(&logits).map_err(fail)?;
        let mut answer: Vec<u32> = Vec::new();
        let mut text = String::new();
        while next != end && answer.len() < limit {
            answer.push(next);
            let now = tokenizer.decode(&answer, true).map_err(|e| e.to_string())?;
            // (a letter of several bytes comes a token at a time: nothing is said until it is whole)
            if !now.ends_with('\u{fffd}') && now != text {
                text = now;
                if !said(&text) {
                    break;
                }
            }
            let input = Tensor::new(&[next], &Device::Cpu).and_then(|t| t.unsqueeze(0)).map_err(fail)?;
            let logits = model.forward(&input, prompt_tokens.len() + answer.len() - 1).and_then(|l| l.squeeze(0)).map_err(fail)?;
            let from = answer.len().saturating_sub(REPEAT_WINDOW);
            let logits = candle_transformers::utils::apply_repeat_penalty(&logits, REPEAT_PENALTY, &answer[from..]).map_err(fail)?;
            next = pick.sample(&logits).map_err(fail)?;
        }
        Ok(tokenizer.decode(&answer, true).map_err(|e| e.to_string())?.trim().to_string())
    }
}

/// Fetches a known model's two files into its folder under `base`: each as `<name>.part` first; the weights are then
/// checked against the size and the hash written in [`MODELS`], and only a file that passes is given its name. A fetch
/// broken off goes on from where it stopped the next time. `stop` ends it.
pub fn fetch_model(curl: &mut crate::fetch::Curl, known: &Known, base: &Path, stop: &std::sync::atomic::AtomicBool) -> Result<(), String> {
    let (weights, tokenizer) = (known.weights(base), known.tokenizer(base));
    if !tokenizer.is_file() {
        let part = tokenizer.with_extension("json.part");
        let _ = std::fs::remove_file(&part);
        curl.download(known.tokenizer_url, &part, stop)?;
        // (a tokenizer is a JSON object of some size; anything else is an error page)
        let text = std::fs::read_to_string(&part).map_err(|e| e.to_string())?;
        if text.len() < 10_000 || !text.trim_start().starts_with('{') {
            let _ = std::fs::remove_file(&part);
            return Err("what came for the tokenizer is not one".into());
        }
        std::fs::rename(&part, &tokenizer).map_err(|e| e.to_string())?;
    }
    if !known.present(base) {
        let part = fetch_part(known, base);
        curl.download(known.weights_url, &part, stop)?;
        let size = std::fs::metadata(&part).map_or(0, |m| m.len());
        if size != known.weights_bytes {
            // (longer than it should be can never become right: begin again the next time)
            if size > known.weights_bytes {
                let _ = std::fs::remove_file(&part);
            }
            return Err(format!("the model's file came as {size} bytes of {}: try again to go on", known.weights_bytes));
        }
        let hash = sha256_of(&part).map_err(|e| e.to_string())?;
        if hash != known.weights_sha256 {
            let _ = std::fs::remove_file(&part);
            return Err("the model's file is not the one expected (its checksum differs): it was thrown away".into());
        }
        std::fs::rename(&part, &weights).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The weights while they are being fetched.
pub fn fetch_part(known: &Known, base: &Path) -> PathBuf {
    known.weights(base).with_extension("gguf.part")
}

/// How much of a model's weights is there: all of them when the model is present, else what the fetch has so far.
pub fn fetched_bytes(known: &Known, base: &Path) -> u64 {
    if known.present(base) { known.weights_bytes } else { std::fs::metadata(fetch_part(known, base)).map_or(0, |m| m.len()) }
}

/// What the model inside the client is told: shorter and plainer than what a large model gets ([`crate::llm::SYSTEM`]),
/// since a small one follows a few plain rules better than many fine ones.
pub const SYSTEM: &str = "You answer a player's questions about the online game Anarchy Online. You know nothing about the game except the numbered sources in the message. You never add a fact that is not written in the sources.";

/// What follows the sources and the question: the rule again, last, where a small model heeds it most.
pub const RULE: &str = "Answer in one to three short sentences, using only what the sources above say. If the sources above do not contain the answer, reply with exactly these words and nothing else: The sources do not say.";

/// The user's part as the small model gets it: the sources and the question ([`crate::llm::prompt`]), then [`RULE`].
pub fn ask(question: &str, hits: &[crate::index::Hit], context_chars: usize) -> (String, usize) {
    let (user, used) = crate::llm::prompt(question, hits, context_chars);
    (format!("{user}\n\n{RULE}"), used)
}

/// How long an answer may be, in tokens.
pub const ANSWER_TOKENS: usize = 220;

/// The model's answer to a question from the pieces found for it (the first of `hits` that fit in `context_chars`).
/// `said` gets the answer as it grows and may stop it.
pub fn answer(model: &mut Local, question: &str, hits: &[crate::index::Hit], context_chars: usize, said: &mut (dyn FnMut(&str) -> bool + Send)) -> Result<crate::llm::Answer, String> {
    if hits.is_empty() {
        return Err("nothing in the index matches the question".into());
    }
    let (user, sources) = ask(question, hits, context_chars);
    let text = model.generate(&chat(SYSTEM, &user), ANSWER_TOKENS, said)?;
    Ok(crate::llm::Answer { text, sources })
}

/// Lower-case hex of a file's SHA-256 (read in pieces: the weights are a gigabyte).
pub fn sha256_of(path: &Path) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chat_form_leaves_the_assistants_turn_open() {
        assert_eq!(chat("Be brief.", "Why?"), "<|im_start|>system\nBe brief.<|im_end|>\n<|im_start|>user\nWhy?<|im_end|>\n<|im_start|>assistant\n");
    }

    #[test]
    fn a_model_is_known_by_its_name_and_has_its_place() {
        assert_eq!(known(" QWEN2.5-1.5B ").name, "qwen2.5-1.5b");
        // (a name that is none's: the default, the first)
        assert_eq!(known("something else").name, MODELS[0].name);
        assert_eq!([1, 4, 8, 12, 32, 64].map(threads_for), [2, 2, 4, 6, 8, 8]);
        let base = Path::new("help");
        assert_eq!(MODELS[0].weights(base), Path::new("help").join("models").join("qwen2.5-1.5b").join("model.gguf"));
        assert!(!MODELS[0].present(Path::new("no such folder")));
        // (what is fetched is the Qwen team's own, over a safe connection, and its hash is written down)
        for m in MODELS {
            assert!(m.weights_url.starts_with("https://huggingface.co/Qwen/") && m.tokenizer_url.starts_with("https://huggingface.co/Qwen/"));
            assert_eq!(m.weights_sha256.len(), 64);
        }
    }

    #[test]
    fn a_file_is_hashed() {
        let dir = std::env::temp_dir().join(format!("mao-help-hash-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("abc.txt");
        std::fs::write(&file, b"abc").unwrap();
        assert_eq!(sha256_of(&file).unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    /// `MAO_HELP_MODEL=<folder with model.gguf and tokenizer.json>`: the model answers from a source and nothing else.
    #[test]
    fn the_model_answers_from_its_sources() {
        let Some(dir) = std::env::var_os("MAO_HELP_MODEL").map(PathBuf::from) else { return };
        let mut model = Local::load(&dir.join("model.gguf"), &dir.join("tokenizer.json")).unwrap();
        let source = "Sources:\n\n[1] The Blue Lantern\nThe Blue Lantern inn opens at seven in the evening and closes at two in the morning. Its cook is called Mirela.\n\n";
        let user = format!("{source}Question: When does the Blue Lantern open?\n\n{RULE}");
        let user = user.as_str();
        let mut pieces = 0;
        let answer = model.generate(&chat(SYSTEM, user), 60, &mut |_| {
            pieces += 1;
            true
        }).unwrap();
        assert!(answer.to_lowercase().contains("seven") || answer.contains('7'), "{answer}");
        assert!(pieces > 0);
        // (the same again: the same answer, the earlier question forgotten)
        assert_eq!(model.generate(&chat(SYSTEM, user), 60, &mut |_| true).unwrap(), answer);
        // (what the sources do not hold is not made up)
        let other = format!("{source}Question: What does a room at the Blue Lantern cost?\n\n{RULE}");
        let unknown = model.generate(&chat(SYSTEM, &other), 60, &mut |_| true).unwrap();
        assert!(unknown.to_lowercase().contains("do not say") || unknown.to_lowercase().contains("not mention") || unknown.to_lowercase().contains("no information"), "{unknown}");
    }
}
