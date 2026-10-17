use crate::domain::AppResult;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
pub const API_BASE: &str = "https://api.deepseek.com";
pub const RETRY_DELAYS: [u64; 2] = [1, 3];
pub const MAX_RESPONSE: usize = 2 * 1024 * 1024;
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cached_tokens: Option<u64>,
    pub cache_miss_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
}
#[derive(Debug)]
pub struct ChatReply {
    pub error: Option<String>,
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
}
pub struct DeepSeekClient {
    http: reqwest::Client,
    base: String,
    delays: [Duration; 2],
}
fn status_error(status: u16) -> String {
    match status {
        401 => "DeepSeek API密钥无效或已失效，请更换密钥".into(),
        402 => "DeepSeek账户余额不足，请检查账户账单".into(),
        429 => "DeepSeek请求过于频繁，请稍后重试（HTTP 429）".into(),
        500 | 502 | 503 | 504 => format!("DeepSeek服务暂时不可用，请稍后重试（HTTP {status}）"),
        300..=399 => "DeepSeek返回重定向，已阻止转发密钥".into(),
        _ => format!("DeepSeek请求失败（HTTP {status}），请检查设置后重试"),
    }
}
fn network_error(e: reqwest::Error) -> String {
    if e.is_timeout() {
        "DeepSeek请求超时，请检查网络后重试".into()
    } else if e.is_connect() {
        "无法连接DeepSeek，请检查网络或系统代理".into()
    } else {
        "DeepSeek网络请求失败，请稍后重试".into()
    }
}
impl DeepSeekClient {
    pub fn new() -> AppResult<Self> {
        Ok(Self {
            http: Self::builder(Duration::from_secs(120))
                .https_only(true)
                .build()
                .map_err(|_| "无法初始化安全AI网络连接")?,
            base: API_BASE.into(),
            delays: RETRY_DELAYS.map(Duration::from_secs),
        })
    }
    fn builder(timeout: Duration) -> reqwest::ClientBuilder {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("Yingliu/", env!("CARGO_PKG_VERSION")))
    }
    async fn request(&self, key: &[u8], path: &str, body: Option<&Value>) -> AppResult<Value> {
        let mut auth =
            reqwest::header::HeaderValue::from_bytes(&[b"Bearer ".as_slice(), key].concat())
                .map_err(|_| "API密钥格式无效")?;
        auth.set_sensitive(true);
        if body.is_some_and(|b| serde_json::to_vec(b).map_or(true, |v| v.len() > 18 * 1024 * 1024))
        {
            return Err("AI请求内容过大，请减少本次内容".into());
        }
        for attempt in 0..=2 {
            let url = format!("{}{path}", self.base);
            let req = if let Some(b) = body {
                self.http.post(&url).json(b)
            } else {
                self.http.get(&url)
            };
            let mut response = req
                .header(reqwest::header::AUTHORIZATION, auth.clone())
                .send()
                .await
                .map_err(network_error)?;
            let status = response.status().as_u16();
            if [429, 500, 502, 503, 504].contains(&status) && attempt < 2 {
                drop(response);
                tokio::time::sleep(self.delays[attempt]).await;
                continue;
            }
            if !(200..300).contains(&status) {
                return Err(status_error(status));
            }
            if response
                .content_length()
                .is_some_and(|n| n > MAX_RESPONSE as u64)
            {
                return Err("DeepSeek响应超过安全大小限制".into());
            }
            let mut data = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(network_error)? {
                if data.len() + chunk.len() > MAX_RESPONSE {
                    return Err("DeepSeek响应超过安全大小限制".into());
                }
                data.extend_from_slice(&chunk);
            }
            return serde_json::from_slice(&data)
                .map_err(|_| "DeepSeek返回无效或截断的JSON，请重试".into());
        }
        Err("DeepSeek请求未完成，请重试".into())
    }
    pub async fn test_connection(&self, key: &[u8], model: &str) -> AppResult<String> {
        let value = self.request(key, "/models", None).await?;
        let available = value
            .get("data")
            .and_then(Value::as_array)
            .is_some_and(|models| {
                models
                    .iter()
                    .any(|m| m.get("id").and_then(Value::as_str) == Some(model))
            });
        if !available {
            return Err("连接成功，但此账户未提供所选模型，请检查模型设置".into());
        }
        Ok("连接成功，所选模型可用；未发送视频内容".into())
    }
    pub async fn chat(
        &self,
        key: &[u8],
        model: &str,
        messages: Vec<Value>,
    ) -> AppResult<ChatReply> {
        let v=self.request(key,"/chat/completions",Some(&json!({"model":model,"messages":messages,"response_format":{"type":"json_object"},"max_tokens":8192,"stream":false,"thinking":{"type":"disabled"}}))).await?;
        let text = v
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str);
        let truncated = v
            .pointer("/choices/0/finish_reason")
            .and_then(Value::as_str)
            == Some("length");
        let error = if truncated {
            Some("DeepSeek输出被截断，请缩小分析内容后重试".into())
        } else if text.is_none_or(|s| s.is_empty() || s.len() > 1024 * 1024) {
            Some("DeepSeek未返回完整分析内容，请重试".into())
        } else {
            None
        };
        let content = if error.is_none() {
            text.unwrap().to_owned()
        } else {
            String::new()
        };
        let usage = TokenUsage {
            cache_miss_tokens: v
                .pointer("/usage/prompt_cache_miss_tokens")
                .and_then(Value::as_u64),
            reasoning_tokens: v
                .pointer("/usage/completion_tokens_details/reasoning_tokens")
                .and_then(Value::as_u64),
            input_tokens: v.pointer("/usage/prompt_tokens").and_then(Value::as_u64),
            output_tokens: v
                .pointer("/usage/completion_tokens")
                .and_then(Value::as_u64),
            cached_tokens: v
                .pointer("/usage/prompt_cache_hit_tokens")
                .and_then(Value::as_u64),
        };
        if [
            usage.input_tokens,
            usage.output_tokens,
            usage.cached_tokens,
            usage.cache_miss_tokens,
            usage.reasoning_tokens,
        ]
        .into_iter()
        .flatten()
        .any(|n| n > i64::MAX as u64)
        {
            return Err("DeepSeek用量数据无效".into());
        }
        Ok(ChatReply {
            error,
            content,
            model: v
                .get("model")
                .and_then(Value::as_str)
                .filter(|s| {
                    s.len() <= 128
                        && s.starts_with("deepseek-")
                        && s.bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                })
                .unwrap_or(model)
                .into(),
            usage,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };
    fn server(
        statuses: Vec<u16>,
        payload: String,
        wait: Duration,
    ) -> (
        DeepSeekClient,
        Arc<AtomicUsize>,
        std::thread::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let n = count.clone();
        let handle = std::thread::spawn(move || {
            for status in statuses {
                let (mut s, _) = listener.accept().unwrap();
                s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                let mut buf = [0; 65536];
                let _ = s.read(&mut buf);
                n.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(wait);
                let response=format!("HTTP/1.1 {status} Mock\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",payload.len());
                let _ = s.write_all(response.as_bytes());
            }
        });
        (
            DeepSeekClient {
                http: DeepSeekClient::builder(Duration::from_millis(100))
                    .no_proxy()
                    .build()
                    .unwrap(),
                base: format!("http://{addr}"),
                delays: [Duration::ZERO; 2],
            },
            count,
            handle,
        )
    }
    #[test]
    fn mocked_http_auth_balance_rate_retry_and_sanitization() {
        tauri::async_runtime::block_on(async {
            for status in [401, 402, 429, 500, 503] {
                let tries = if [429, 500, 503].contains(&status) {
                    3
                } else {
                    1
                };
                let (c, n, t) = server(
                    vec![status; tries],
                    "{\"error\":\"sk-private-test-value\"}".into(),
                    Duration::ZERO,
                );
                let error = c
                    .test_connection(b"sk-private-test-value", "deepseek-flash")
                    .await
                    .unwrap_err();
                assert!(!error.contains("sk-private"));
                assert!(!error.contains("Authorization"));
                assert!(error.contains(&status.to_string()) || status == 401 || status == 402);
                t.join().unwrap();
                assert_eq!(n.load(Ordering::SeqCst), tries);
            }
            let (c, n, t) = server(
                vec![503, 429, 200],
                json!({"data":[{"id":"deepseek-flash"}]}).to_string(),
                Duration::ZERO,
            );
            assert!(c
                .test_connection(b"sk-private-test-value", "deepseek-flash")
                .await
                .is_ok());
            t.join().unwrap();
            assert_eq!(n.load(Ordering::SeqCst), 3);
            assert_eq!(RETRY_DELAYS, [1, 3]);
        });
    }
    #[test]
    fn mocked_http_invalid_json_timeout_disconnect_redirect_and_usage() {
        tauri::async_runtime::block_on(async {
            for (status, payload, wait, expected) in [
                (200, "{broken", 0, "JSON"),
                (200, "{}", 200, "超时"),
                (302, "{}", 0, "重定向"),
            ] {
                let (c, _, t) = server(vec![status], payload.into(), Duration::from_millis(wait));
                let error = c
                    .test_connection(b"sk-private-test-value", "deepseek-flash")
                    .await
                    .unwrap_err();
                assert!(error.contains(expected), "{error}");
                t.join().unwrap();
            }
            let(c,_,t)=server(vec![200],json!({"model":"deepseek-flash","choices":[{"message":{"content":"{\"answer\":\"未知\"}"},"finish_reason":"stop"}],"usage":{"prompt_tokens":123,"completion_tokens":45,"prompt_cache_hit_tokens":20}}).to_string(),Duration::ZERO);
            let result = c
                .chat(b"sk-private-test-value", "deepseek-flash", vec![])
                .await
                .unwrap();
            t.join().unwrap();
            assert_eq!(result.usage.input_tokens, Some(123));
            assert_eq!(result.usage.cached_tokens, Some(20));
            let c = DeepSeekClient {
                http: DeepSeekClient::builder(Duration::from_secs(5))
                    .no_proxy()
                    .build()
                    .unwrap(),
                base: "http://127.0.0.1:1".into(),
                delays: [Duration::ZERO; 2],
            };
            assert!(c
                .test_connection(b"sk-private-test-value", "deepseek-flash")
                .await
                .unwrap_err()
                .contains("连接"));
        });
    }
}

impl ChatReply {
    pub fn checked_content(&self) -> AppResult<&str> {
        if let Some(error) = &self.error {
            Err(error.clone())
        } else {
            Ok(&self.content)
        }
    }
}
#[cfg(test)]
impl DeepSeekClient {
    pub(crate) fn fixture(base: String) -> Self {
        assert!(base.starts_with("http://127.0.0.1:"));
        Self {
            http: Self::builder(Duration::from_secs(5))
                .no_proxy()
                .build()
                .unwrap(),
            base,
            delays: [Duration::ZERO; 2],
        }
    }
}

#[cfg(test)]
mod response_limit_tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    #[test]
    fn truncated_outputs_preserve_usage_and_oversized_responses_are_rejected() {
        tauri::async_runtime::block_on(async {
            for oversized in [false, true] {
                let listener = TcpListener::bind("127.0.0.1:0").unwrap();
                let client =
                    DeepSeekClient::fixture(format!("http://{}", listener.local_addr().unwrap()));
                let thread = std::thread::spawn(move || {
                    let (mut stream, _) = listener.accept().unwrap();
                    let mut buffer = [0; 65536];
                    let _ = stream.read(&mut buffer);
                    let payload = if oversized {
                        "X".repeat(MAX_RESPONSE + 1)
                    } else {
                        json!({"model":"deepseek-flash","choices":[{"message":{"content":"{partial"},"finish_reason":"length"}],"usage":{"prompt_tokens":88,"completion_tokens":20}}).to_string()
                    };
                    let _=stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",payload.len()).as_bytes());
                });
                let result = client
                    .chat(b"sk-fake-private-value", "deepseek-flash", vec![])
                    .await;
                if oversized {
                    assert!(result.unwrap_err().contains("超过"));
                } else {
                    let reply = result.unwrap();
                    assert_eq!(reply.usage.input_tokens, Some(88));
                    assert!(reply.checked_content().unwrap_err().contains("截断"));
                }
                thread.join().unwrap();
            }
        });
    }
}
