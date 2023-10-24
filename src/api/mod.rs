use crate::util;
use anyhow::Result;
use curl::easy::{Easy, List, WriteError};
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{cell::RefCell, collections::HashMap, sync::OnceLock};
use uuid::Uuid;

mod arkose;
mod crypto;
mod sse;

pub static COOKIE: OnceLock<String> = OnceLock::new();
pub static TOKEN: OnceLock<String> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct PublicKey {
    #[serde(rename = "token")]
    arkose_token: String,
}

pub fn public_key() -> Result<PublicKey> {
    #[rustfmt::skip]
    const URL: &str = "https://tcr9i.chat.openai.com/fc/gt2/public_key/35536E1E-65B4-4D96-9D97-6ADB7EFF8147";

    Ok(http()
        .post(URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(arkose::arkose()?)
        .send()?
        .json::<PublicKey>()?)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Conversations {
    pub items: Vec<Item>,
    total: u32,
    limit: u32,
    offset: u32,
    has_missing_conversations: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub title: String,
}

pub fn conversations() -> Result<Conversations> {
    const URL: &str = "https://chat.openai.com/backend-api/conversations";

    let url = format!("{URL}?offset={}&limit={}&order=updated", 0, 50);
    request::<Conversations>(&url, None, None)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationInfo {
    conversation_id: String,
    title: String,
    pub current_node: String,
    mapping: HashMap<String, Node>,
    moderation_results: Vec<String>,
    create_time: f64,
    update_time: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Node {
    id: String,
    message: Option<Message>,
    parent: Option<String>,
    children: Option<Vec<String>>,
}

pub fn conversation_info(conversation_id: &str) -> Result<ConversationInfo> {
    const URL: &str = "https://chat.openai.com/backend-api/conversation";

    let url = format!("{URL}/{conversation_id}");
    request::<ConversationInfo>(&url, None, None)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Conversation {
    conversation_id: String,
    error: Option<String>,
    message: Option<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    id: String,
    metadata: Metadata,
    weight: f64,
    status: String,
    content: Content,
    author: Author,
    recipient: String,
    create_time: Option<f64>,
    update_time: Option<f64>,
    end_turn: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Author {
    metadata: HashMap<String, String>,
    name: Option<String>,
    role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Content {
    content_type: String,
    #[serde(default = "Default::default")]
    parts: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    parent_id: Option<String>,
    message_type: Option<String>,
    finish_details: Option<FinishDetails>,
    is_complete: Option<bool>,
    model_slug: Option<String>,
    timestamp_: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FinishDetails {
    #[serde(default = "Default::default")]
    stop_tokens: Vec<i64>,
    #[serde(rename = "type")]
    type_field: String,
}

pub fn conversation(conversation_id: &str, message: &str) -> Result<()> {
    const URL: &str = "https://chat.openai.com/backend-api/conversation";

    let (cookie, token) = match (|| Some((COOKIE.get()?, TOKEN.get()?)))() {
        Some(v) => v,
        None => {
            panic!("Either cookie or token is missing");
        }
    };

    let mut list = List::new();
    list.append("Host: chat.openai.com")?;
    list.append(&format!("Authorization: Bearer {}", token))?;
    list.append("Connection: keep-alive")?;
    list.append(&format!("Cookie: {}", cookie))?;
    list.append("Referer: https://chat.openai.com/")?;
    list.append("Origin: https://chat.openai.com")?;
    list.append("Content-Type: application/json")?;

    let conversation_info = conversation_info(conversation_id)?;

    let json = json!({
        "action": "next",
        "arkose_token": public_key()?.arkose_token,
        "conversation_id": conversation_id,
        "force_paragen": false,
        "history_and_training_disabled": false,
        "messages": [
            {
                "author": {
                    "role": "user"
                },
                "content": {
                    "content_type": "text",
                    "parts": [ message ]
                },
                "id": Uuid::new_v4().to_string(),
                "metadata": {}
            }
        ],
        "model": "gpt-4",
        "parent_message_id": conversation_info.current_node,
        "suggestions": [],
        "timezone_offset_min": -480
    });
    let json_str = json.to_string();

    let mut last_part = String::new();
    let buffer = RefCell::new(String::new());

    let write = move |buf: &[u8]| {
        let mut buffer = buffer.borrow_mut();
        let body = String::from_utf8_lossy(buf);

        for event in sse::parse_sse_chunk(&body, &mut buffer) {
            if let Some(data) = event.data {
                let value = match serde_json::from_str::<Conversation>(&data) {
                    Ok(c) => c,
                    Err(e) => {
                        error!("{e}");
                        continue;
                    }
                };

                if let Some(message) = value.message {
                    for part in message.content.parts {
                        print!("{}", part.replace(&last_part, ""));

                        last_part.clear();
                        last_part.push_str(&part);
                    }
                }
            }
        }

        Ok(buf.len())
    };

    let mut easy = Easy::new();
    easy.post(true)?;
    easy.url(URL)?;
    easy.http_headers(list)?;

    let buf = json_str.as_bytes();
    easy.post_field_size(buf.len() as u64)?;
    easy.post_fields_copy(buf)?;

    let mut transfer = easy.transfer();
    transfer.write_function(write)?;
    transfer.perform()?;

    Ok(())
}

fn http() -> &'static reqwest::blocking::Client {
    static HTTP: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    HTTP.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .cookie_store(true)
            .build()
            .unwrap()
    })
}

type Write<'data> = Box<dyn FnMut(&[u8]) -> Result<usize, WriteError> + 'data>;

fn request<T>(url: &str, body: Option<&str>, write: Option<Write>) -> Result<T>
where
    T: Serialize + for<'de> Deserialize<'de> + 'static,
{
    let (cookie, token) = match (|| Some((COOKIE.get()?, TOKEN.get()?)))() {
        Some(v) => v,
        None => {
            panic!("Either cookie or token is missing");
        }
    };

    let mut list = List::new();
    list.append("Host: chat.openai.com")?;
    list.append(&format!("Authorization: Bearer {}", token))?;
    list.append("Connection: keep-alive")?;
    list.append(&format!("Cookie: {}", cookie))?;
    list.append("Referer: https://chat.openai.com/")?;
    list.append("Origin: https://chat.openai.com")?;

    if body.is_some() {
        list.append("Content-Type: application/json")?;
    }

    let buffer = RefCell::new(Vec::new());
    let write = write.unwrap_or(Box::new(|buf: &[u8]| {
        buffer.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }));

    let mut easy = Easy::new();
    easy.url(url)?;
    easy.http_headers(list)?;

    if let Some(body) = body {
        easy.post(true)?;

        let buf = body.as_bytes();
        easy.post_fields_copy(buf)?;
        easy.post_field_size(buf.len() as u64)?;
    }

    let mut transfer = easy.transfer();
    transfer.write_function(write)?;
    transfer.perform()?;

    if util::is_unit::<T>() {
        Ok(serde_json::from_str::<T>("null")?)
    } else {
        let buffer = buffer.borrow();
        let body = String::from_utf8_lossy(&buffer);
        Ok(serde_json::from_str::<T>(&body)?)
    }
}
