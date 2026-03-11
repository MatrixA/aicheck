use anyhow::Result;
use id3::{Tag, TagLike};
use std::path::Path;

use super::{Confidence, Signal, SignalBuilder, SignalSource};
use crate::known_tools;

/// Known AI audio platform URL domains.
const AI_URL_DOMAINS: &[(&str, &str)] = &[
    ("suno.com", "suno"),
    ("udio.com", "udio"),
    ("soundraw.io", "soundraw"),
    ("aiva.ai", "aiva"),
    ("boomy.com", "boomy"),
    ("mubert.com", "mubert"),
];

fn detect_comments(tag: &Tag) -> Vec<Signal> {
    let mut signals = Vec::new();
    for comment in tag.comments() {
        let text = &comment.text;
        if text.is_empty() {
            continue;
        }
        if let Some(tool_name) = known_tools::match_ai_tool(text) {
            signals.push(
                SignalBuilder::new(
                    SignalSource::Id3Metadata,
                    Confidence::Medium,
                    "signal_id3_comment",
                )
                .param("text", text.as_str())
                .tool(tool_name)
                .detail("COMM", text.as_str())
                .build(),
            );
        }
    }
    signals
}

fn detect_urls(tag: &Tag) -> Vec<Signal> {
    let mut signals = Vec::new();
    let url_frame_ids = ["WOAS", "WOAF", "WOAR", "WORS", "WPUB"];
    for frame in tag.frames() {
        let frame_id = frame.id();
        if !url_frame_ids.contains(&frame_id) {
            continue;
        }
        if let Some(link) = frame.content().link() {
            check_url(&mut signals, frame_id, link);
        }
    }
    for ext_link in tag.extended_links() {
        let url = &ext_link.link;
        check_url(&mut signals, "WXXX", url);
    }
    signals
}

fn check_url(signals: &mut Vec<Signal>, frame_id: &str, url: &str) {
    let lower = url.to_lowercase();
    for &(domain, tool_name) in AI_URL_DOMAINS {
        if lower.contains(domain) {
            signals.push(
                SignalBuilder::new(
                    SignalSource::Id3Metadata,
                    Confidence::Medium,
                    "signal_id3_url",
                )
                .param("url", url)
                .tool(tool_name)
                .detail(frame_id, url)
                .build(),
            );
            break;
        }
    }
}

fn detect_text_frames(tag: &Tag) -> Vec<Signal> {
    let mut signals = Vec::new();
    let check_frames = ["TENC", "TPUB", "TSSE"];
    for frame_id in &check_frames {
        if let Some(text) = tag.get(frame_id).and_then(|f| f.content().text()) {
            if let Some(tool_name) = known_tools::match_ai_tool(text) {
                signals.push(
                    SignalBuilder::new(
                        SignalSource::Id3Metadata,
                        Confidence::Medium,
                        "signal_id3_text_frame",
                    )
                    .param("frame", *frame_id)
                    .param("text", text)
                    .tool(tool_name)
                    .detail(*frame_id, text)
                    .build(),
                );
            }
        }
    }
    for txxx in tag.extended_texts() {
        let combined = format!("{} {}", txxx.description, txxx.value);
        if let Some(tool_name) = known_tools::match_ai_tool(&combined) {
            signals.push(
                SignalBuilder::new(
                    SignalSource::Id3Metadata,
                    Confidence::Medium,
                    "signal_id3_txxx",
                )
                .param("desc", &txxx.description)
                .param("value", &txxx.value)
                .tool(tool_name)
                .detail("TXXX description", &txxx.description)
                .detail("TXXX value", &txxx.value)
                .build(),
            );
        }
    }
    signals
}

pub fn detect(path: &Path) -> Result<Vec<Signal>> {
    let tag = match Tag::read_from_path(path) {
        Ok(t) => t,
        Err(_) => return Ok(vec![]),
    };
    let mut signals = Vec::new();
    signals.extend(detect_comments(&tag));
    signals.extend(detect_urls(&tag));
    signals.extend(detect_text_frames(&tag));
    Ok(signals)
}

pub fn dump_info(path: &Path) -> Result<Vec<(String, String)>> {
    let tag = match Tag::read_from_path(path) {
        Ok(t) => t,
        Err(_) => return Ok(vec![]),
    };
    let mut props = Vec::new();
    if let Some(title) = tag.title() {
        props.push(("Title (TIT2)".to_string(), title.to_string()));
    }
    if let Some(artist) = tag.artist() {
        props.push(("Artist (TPE1)".to_string(), artist.to_string()));
    }
    if let Some(album) = tag.album() {
        props.push(("Album (TALB)".to_string(), album.to_string()));
    }
    let text_frame_ids = ["TENC", "TPUB", "TSSE", "TCON", "TDRC", "TYER"];
    for frame_id in &text_frame_ids {
        if let Some(text) = tag.get(frame_id).and_then(|f| f.content().text()) {
            props.push((frame_id.to_string(), text.to_string()));
        }
    }
    for comment in tag.comments() {
        let key = if comment.description.is_empty() {
            "COMM".to_string()
        } else {
            format!("COMM ({})", comment.description)
        };
        props.push((key, comment.text.clone()));
    }
    let url_frame_ids = ["WOAS", "WOAF", "WOAR", "WORS", "WPUB"];
    for frame_id in &url_frame_ids {
        if let Some(frame) = tag.get(frame_id) {
            if let Some(link) = frame.content().link() {
                props.push((frame_id.to_string(), link.to_string()));
            }
        }
    }
    for txxx in tag.extended_texts() {
        props.push((format!("TXXX:{}", txxx.description), txxx.value.clone()));
    }
    for wxxx in tag.extended_links() {
        props.push((format!("WXXX:{}", wxxx.description), wxxx.link.clone()));
    }
    Ok(props)
}

#[cfg(test)]
mod tests {
    use super::*;
    use id3::frame::{Comment, ExtendedLink, ExtendedText};
    use id3::Frame;

    fn make_tag_with_comment(text: &str) -> Tag {
        let mut tag = Tag::new();
        tag.add_frame(Comment {
            lang: "eng".into(),
            description: String::new(),
            text: text.into(),
        });
        tag
    }

    #[test]
    fn test_detect_suno_comment() {
        let tag = make_tag_with_comment("made with suno; created=2026-03-11T02:33:28.817Z; id=abc");
        let signals = detect_comments(&tag);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].tool, Some("suno".to_string()));
        assert_eq!(signals[0].confidence, Confidence::Medium);
    }

    #[test]
    fn test_detect_no_ai_comment() {
        let tag = make_tag_with_comment("Recorded live at the concert hall");
        let signals = detect_comments(&tag);
        assert!(signals.is_empty());
    }

    #[test]
    fn test_detect_url_suno() {
        let mut tag = Tag::new();
        tag.add_frame(Frame::link("WOAS", "https://suno.com/song/abc-123"));
        let signals = detect_urls(&tag);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].tool, Some("suno".to_string()));
    }

    #[test]
    fn test_detect_url_no_match() {
        let mut tag = Tag::new();
        tag.add_frame(Frame::link("WOAS", "https://spotify.com/track/abc"));
        let signals = detect_urls(&tag);
        assert!(signals.is_empty());
    }

    #[test]
    fn test_detect_wxxx_udio() {
        let mut tag = Tag::new();
        tag.add_frame(ExtendedLink {
            description: "source".into(),
            link: "https://udio.com/songs/xyz".into(),
        });
        let signals = detect_urls(&tag);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].tool, Some("udio".to_string()));
    }

    #[test]
    fn test_detect_tenc_ai_tool() {
        let mut tag = Tag::new();
        tag.add_frame(Frame::text("TENC", "Suno AI"));
        let signals = detect_text_frames(&tag);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].tool, Some("suno".to_string()));
    }

    #[test]
    fn test_detect_txxx_ai_tool() {
        let mut tag = Tag::new();
        tag.add_frame(ExtendedText {
            description: "generator".into(),
            value: "Suno AI v4".into(),
        });
        let signals = detect_text_frames(&tag);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].tool, Some("suno".to_string()));
    }

    #[test]
    fn test_detect_empty_tag() {
        let tag = Tag::new();
        assert!(detect_comments(&tag).is_empty());
        assert!(detect_urls(&tag).is_empty());
        assert!(detect_text_frames(&tag).is_empty());
    }

    #[test]
    fn test_dump_info_basic() {
        let mut tag = Tag::new();
        tag.set_title("My Song");
        tag.set_artist("Test Artist");
        tag.add_frame(Comment {
            lang: "eng".into(),
            description: String::new(),
            text: "made with suno".into(),
        });
        let comments = detect_comments(&tag);
        assert_eq!(comments.len(), 1);
    }
}
