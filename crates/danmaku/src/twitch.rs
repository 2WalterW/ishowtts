use std::collections::HashMap;

use anyhow::{anyhow, Result};
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Map as JsonMap, Value as JsonValue};

use crate::message::{NormalizedMessage, Platform, Priority};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TwitchChatMessage {
    pub channel: String,
    pub username: String,
    pub user_id: Option<String>,
    pub message: String,
    pub bits: Option<u32>,
    pub badges: Vec<String>,
    pub raw_tags: HashMap<String, String>,
}

impl TwitchChatMessage {
    pub fn to_normalized(&self) -> NormalizedMessage {
        let mut priority = Priority::Normal;
        if let Some(bits) = self.bits {
            if bits > 0 {
                priority = Priority::Paid;
            }
        }
        if self
            .badges
            .iter()
            .any(|badge| badge.starts_with("moderator") || badge.starts_with("broadcaster"))
        {
            priority = Priority::Moderator;
        }
        let metadata = build_metadata(self);
        NormalizedMessage::new_text(
            Platform::Twitch,
            self.channel.clone(),
            self.user_id.clone(),
            self.username.clone(),
            priority,
            self.message.clone(),
            metadata,
        )
    }
}

fn build_metadata(message: &TwitchChatMessage) -> JsonValue {
    let mut meta = JsonMap::new();
    meta.insert("badges".into(), json!(message.badges));
    if let Some(bits) = message.bits {
        meta.insert("bits".into(), json!(bits));
    }

    if let Some(color) = message
        .raw_tags
        .get("color")
        .filter(|value| !value.is_empty())
    {
        meta.insert("color".into(), json!(color));
    }

    meta.insert(
        "emotes".into(),
        JsonValue::Array(parse_emotes(message.raw_tags.get("emotes"))),
    );

    if let Some(reply) = parse_reply(&message.raw_tags) {
        meta.insert("reply".into(), reply);
    }

    meta.insert(
        "first_msg".into(),
        JsonValue::Bool(tag_as_bool(&message.raw_tags, "first-msg")),
    );
    meta.insert(
        "returning_chatter".into(),
        JsonValue::Bool(tag_as_bool(&message.raw_tags, "returning-chatter")),
    );
    meta.insert(
        "subscriber".into(),
        JsonValue::Bool(tag_as_bool(&message.raw_tags, "subscriber")),
    );

    if let Some(user_type) = message
        .raw_tags
        .get("user-type")
        .filter(|value| !value.is_empty())
    {
        meta.insert("user_type".into(), json!(user_type));
    }

    if let Some(display) = message
        .raw_tags
        .get("display-name")
        .filter(|value| !value.is_empty())
    {
        meta.insert("display_name".into(), json!(display));
    }

    if let Some(login) = message
        .raw_tags
        .get("user-login")
        .filter(|value| !value.is_empty())
    {
        meta.insert("user_login".into(), json!(login));
    }

    if let Some(msg_id) = message.raw_tags.get("id").filter(|value| !value.is_empty()) {
        meta.insert("message_id".into(), json!(msg_id));
    }

    if let Some(ts) = message
        .raw_tags
        .get("tmi-sent-ts")
        .and_then(|value| value.parse::<i64>().ok())
    {
        meta.insert("timestamp_ms".into(), json!(ts));
    }

    if let Some(room_id) = message
        .raw_tags
        .get("room-id")
        .filter(|value| !value.is_empty())
    {
        meta.insert("room_id".into(), json!(room_id));
    }

    if let Some(user_id) = message
        .raw_tags
        .get("user-id")
        .filter(|value| !value.is_empty())
    {
        meta.insert("user_id".into(), json!(user_id));
    }

    meta.insert(
        "raw_tags".into(),
        serde_json::to_value(&message.raw_tags).unwrap_or(JsonValue::Null),
    );

    JsonValue::Object(meta)
}

fn parse_emotes(tag: Option<&String>) -> Vec<JsonValue> {
    let Some(raw) = tag else {
        return Vec::new();
    };

    raw.split('/')
        .filter_map(|spec| {
            if spec.is_empty() {
                return None;
            }
            let mut parts = spec.split(':');
            let id = parts.next().unwrap_or_default();
            let Some(indices_part) = parts.next() else {
                return None;
            };
            let positions: Vec<JsonValue> = indices_part
                .split(',')
                .filter_map(|range| {
                    let (start_str, end_str) = range.split_once('-')?;
                    let start = start_str.parse::<usize>().ok()?;
                    let end = end_str.parse::<usize>().ok()?;
                    Some(json!({ "start": start, "end": end }))
                })
                .collect();
            Some(json!({
                "id": id,
                "positions": positions,
            }))
        })
        .collect()
}

fn parse_reply(tags: &HashMap<String, String>) -> Option<JsonValue> {
    let parent_id = tags.get("reply-parent-msg-id")?;
    let mut reply = JsonMap::new();
    reply.insert("parent_msg_id".into(), json!(parent_id));

    if let Some(display_name) = tags.get("reply-parent-display-name") {
        reply.insert("parent_display_name".into(), json!(display_name));
    }
    if let Some(body) = tags.get("reply-parent-msg-body") {
        reply.insert("parent_message".into(), json!(body));
    }
    if let Some(user_id) = tags.get("reply-parent-user-id") {
        reply.insert("parent_user_id".into(), json!(user_id));
    }
    if let Some(user_login) = tags.get("reply-parent-user-login") {
        reply.insert("parent_user_login".into(), json!(user_login));
    }
    if let Some(thread_parent) = tags.get("reply-thread-parent-msg-id") {
        reply.insert("thread_parent_msg_id".into(), json!(thread_parent));
    }
    if let Some(thread_parent_user) = tags.get("reply-thread-parent-user-id") {
        reply.insert("thread_parent_user_id".into(), json!(thread_parent_user));
    }

    Some(JsonValue::Object(reply))
}

fn tag_as_bool(tags: &HashMap<String, String>, key: &str) -> bool {
    matches!(tags.get(key).map(String::as_str), Some("1"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawIrcMessage {
    tags: HashMap<String, String>,
    prefix: Option<String>,
    command: String,
    params: Vec<String>,
}

pub(crate) fn parse_irc_message(line: &str) -> Result<RawIrcMessage> {
    let mut rest = line.trim_end_matches('\r').trim_end_matches('\n');
    let mut tags = HashMap::new();
    if rest.starts_with('@') {
        if let Some(idx) = rest.find(' ') {
            let (raw_tags, remainder) = rest.split_at(idx);
            rest = remainder.trim_start();
            for tag in raw_tags.trim_start_matches('@').split(';') {
                let mut parts = tag.splitn(2, '=');
                let key = parts.next().unwrap_or_default();
                let value = parts.next().unwrap_or("");
                tags.insert(key.to_string(), value.to_string());
            }
        } else {
            return Err(anyhow!("invalid IRC tags segment"));
        }
    }

    let mut prefix = None;
    if rest.starts_with(':') {
        if let Some(idx) = rest.find(' ') {
            prefix = Some(rest[1..idx].to_string());
            rest = rest[idx + 1..].trim_start();
        } else {
            return Err(anyhow!("invalid IRC prefix segment"));
        }
    }

    let mut parts = rest.splitn(2, ' ');
    let command = parts
        .next()
        .ok_or_else(|| anyhow!("missing IRC command"))?
        .to_string();
    let params_part = parts.next().unwrap_or("");

    let mut params = Vec::new();
    let mut iter = params_part.split(' ');
    while let Some(part) = iter.next() {
        if part.is_empty() {
            continue;
        }
        if part.starts_with(':') {
            let mut trailing = part.trim_start_matches(':').to_string();
            let rest_trailing: Vec<&str> = iter.collect();
            if !rest_trailing.is_empty() {
                if !trailing.is_empty() {
                    trailing.push(' ');
                }
                trailing.push_str(&rest_trailing.join(" "));
            }
            params.push(trailing);
            break;
        } else {
            params.push(part.to_string());
        }
    }

    Ok(RawIrcMessage {
        tags,
        prefix,
        command,
        params,
    })
}

pub fn parse_privmsg(line: &str) -> Result<Option<TwitchChatMessage>> {
    let msg = parse_irc_message(line)?;
    if msg.command != "PRIVMSG" {
        return Ok(None);
    }

    if msg.params.len() < 2 {
        return Err(anyhow!("PRIVMSG missing params"));
    }

    let channel = msg.params[0].trim_start_matches('#').to_string();
    let message = msg.params[1].clone();
    let display_name = msg
        .tags
        .get("display-name")
        .cloned()
        .or_else(|| {
            msg.prefix
                .as_ref()
                .map(|p| p.split('!').next().unwrap_or(p).to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());
    let user_id = msg.tags.get("user-id").cloned();
    let bits = msg
        .tags
        .get("bits")
        .and_then(|bits| bits.parse::<u32>().ok());
    let badges = msg
        .tags
        .get("badges")
        .map(|badges| badges.split(',').map(|s| s.to_string()).collect())
        .unwrap_or_else(Vec::new);

    Ok(Some(TwitchChatMessage {
        channel,
        username: display_name,
        user_id,
        message,
        bits,
        badges,
        raw_tags: msg.tags,
    }))
}

pub fn parse_clearmsg(line: &str) -> Result<Option<(String, String)>> {
    let msg = parse_irc_message(line)?;
    if msg.command != "CLEARMSG" {
        return Ok(None);
    }
    if msg.params.len() < 2 {
        return Err(anyhow!("CLEARMSG missing params"));
    }
    let channel = msg.params[0].trim_start_matches('#').to_string();
    let target = msg.params[1].clone();
    Ok(Some((channel, target)))
}

lazy_static::lazy_static! {
    static ref PING_RE: Regex = Regex::new(r"^PING :?(?P<token>.+)").unwrap();
}

pub fn parse_ping(line: &str) -> Option<String> {
    PING_RE
        .captures(line.trim())
        .and_then(|caps| caps.name("token").map(|m| m.as_str().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_privmsg_basic() {
        let line = "@badge-info=;badges=moderator/1;color=#00FF7F;display-name=ModUser;emotes=;flags=;id=abcd-1234-ef;mod=1;room-id=123;subscriber=0;tmi-sent-ts=1660000000000;turbo=0;user-id=42;user-type=mod :moduser!moduser@moduser.tmi.twitch.tv PRIVMSG #channel :Hello World";
        let msg = parse_privmsg(line).unwrap().unwrap();
        assert_eq!(msg.channel, "channel");
        assert_eq!(msg.username, "ModUser");
        assert_eq!(msg.user_id.clone().unwrap(), "42");
        assert_eq!(msg.message, "Hello World");
        let normalized = msg.to_normalized();
        assert_eq!(normalized.content.as_text().unwrap(), "Hello World");
        matches!(normalized.priority, Priority::Moderator);
        let color = normalized
            .metadata
            .get("color")
            .and_then(|value| value.as_str())
            .unwrap();
        assert_eq!(color, "#00FF7F");
    }

    #[test]
    fn parse_ping_token() {
        assert_eq!(
            parse_ping("PING :tmi.twitch.tv"),
            Some("tmi.twitch.tv".into())
        );
    }
}
