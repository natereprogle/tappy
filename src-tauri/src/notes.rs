use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

static TAG_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?s)\[\s*\[\s*#(.*?)#\s*]\s*]").expect("valid tag regex")
});

static TIMESTAMP_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\d{1,2}:\d{2}(?::\d{2})?)\s+(.+)$").expect("valid timestamp regex")
});

static TIMESTAMP_WITHOUT_SPACE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d{1,2}:\d{2}(?::\d{2})?\S+").expect("valid invalid timestamp regex")
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimedTag {
    pub seconds: u64,
    pub keyword: String,
    pub raw_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedNotes {
    pub regular_tags: Vec<String>,
    pub timed_tags: Vec<TimedTag>,
    pub invalid_tags: Vec<String>,
}

impl ParsedNotes {
    pub fn first_regular_tag(&self) -> Option<&str> {
        self.regular_tags.first().map(String::as_str)
    }

    pub fn has_video_tags(&self) -> bool {
        !self.timed_tags.is_empty()
    }
}

pub fn parse_notes(notes: &str) -> ParsedNotes {
    let mut regular_tags = Vec::new();
    let mut timed_tags = Vec::new();
    let mut invalid_tags = Vec::new();

    for capture in TAG_RE.captures_iter(notes) {
        let raw_body = capture.get(1).map(|m| m.as_str()).unwrap_or_default();

        let body = raw_body.trim();

        if body.is_empty() {
            continue;
        }

        if let Some(timestamp_capture) = TIMESTAMP_RE.captures(body) {
            let raw_time = timestamp_capture
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            let raw_keyword = timestamp_capture
                .get(2)
                .map(|m| m.as_str())
                .unwrap_or_default();

            let keyword = normalize_keyword(raw_keyword);

            match parse_timecode(&raw_time) {
                Some(seconds) if !keyword.is_empty() => {
                    timed_tags.push(TimedTag {
                        seconds,
                        keyword,
                        raw_time,
                    });
                }
                _ => invalid_tags.push(body.to_string()),
            }

            continue;
        }

        if TIMESTAMP_WITHOUT_SPACE_RE.is_match(body) {
            invalid_tags.push(body.to_string());
            continue;
        }

        let keyword = normalize_keyword(body);

        if !keyword.is_empty() {
            regular_tags.push(keyword);
        }
    }

    timed_tags.sort_by_key(|tag| tag.seconds);

    ParsedNotes {
        regular_tags,
        timed_tags,
        invalid_tags,
    }
}

pub fn normalize_keyword(value: &str) -> String {
    value
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub fn parse_timecode(value: &str) -> Option<u64> {
    let parts = value
        .split(':')
        .map(|part| part.parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()?;

    match parts.as_slice() {
        // mm:ss
        [minutes, seconds] if *seconds < 60 => Some(minutes * 60 + seconds),

        // h:mm:ss
        [hours, minutes, seconds] if *minutes < 60 && *seconds < 60 => {
            Some(hours * 3600 + minutes * 60 + seconds)
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_regular_tag() {
        let parsed = parse_notes("[[# sermon #]]");

        assert_eq!(parsed.regular_tags, vec!["sermon"]);
        assert!(parsed.timed_tags.is_empty());
        assert!(parsed.invalid_tags.is_empty());
    }

    #[test]
    fn parses_whitespace_around_brackets() {
        let parsed = parse_notes("[[   #   sermon notes   #   ]]");

        assert_eq!(parsed.regular_tags, vec!["sermon notes"]);
    }

    #[test]
    fn parses_video_tags_sorted() {
        let parsed = parse_notes(
            r#"
            [[# 0:30 giving #]]
            [[# 0:00 sermon #]]
            [[# 1:05 connect card #]]
            "#,
        );

        assert_eq!(
            parsed.timed_tags,
            vec![
                TimedTag {
                    seconds: 0,
                    keyword: "sermon".to_string(),
                    raw_time: "0:00".to_string(),
                },
                TimedTag {
                    seconds: 30,
                    keyword: "giving".to_string(),
                    raw_time: "0:30".to_string(),
                },
                TimedTag {
                    seconds: 65,
                    keyword: "connect card".to_string(),
                    raw_time: "1:05".to_string(),
                },
            ]
        );
    }

    #[test]
    fn rejects_timestamp_without_space() {
        let parsed = parse_notes("[[# 0:30giving #]]");

        assert!(parsed.regular_tags.is_empty());
        assert!(parsed.timed_tags.is_empty());
        assert_eq!(parsed.invalid_tags, vec!["0:30giving"]);
    }

    #[test]
    fn parses_regular_and_timed_tags_together() {
        let parsed = parse_notes(
            r#"
            [[# sermon #]]
            [[# 0:30 giving #]]
            "#,
        );

        assert_eq!(parsed.regular_tags, vec!["sermon"]);
        assert_eq!(parsed.timed_tags.len(), 1);
    }
}