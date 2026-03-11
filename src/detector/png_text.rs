use anyhow::Result;
use std::fs;
use std::path::Path;

use super::{Confidence, Signal, SignalBuilder, SignalSource};
use crate::known_tools;

const RELEVANT_KEYWORDS: &[&str] = &[
    "Software",
    "Comment",
    "Description",
    "Source",
    "Author",
    "parameters",
    "prompt",
];

pub fn detect(path: &Path) -> Result<Vec<Signal>> {
    let data = fs::read(path)?;
    if data.len() < 8 || &data[..8] != b"\x89PNG\r\n\x1a\n" {
        return Ok(vec![]);
    }
    let mut signals = Vec::new();
    let mut pos: usize = 8;
    while pos + 12 <= data.len() {
        let length =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        let chunk_type = &data[pos + 4..pos + 8];
        let chunk_data_end = pos + 8 + length;
        if chunk_data_end > data.len() {
            break;
        }
        let chunk_data = &data[pos + 8..chunk_data_end];
        match chunk_type {
            b"tEXt" => {
                if let Some(null_pos) = chunk_data.iter().position(|&b| b == 0) {
                    if let (Ok(keyword), Ok(value)) = (
                        std::str::from_utf8(&chunk_data[..null_pos]),
                        std::str::from_utf8(&chunk_data[null_pos + 1..]),
                    ) {
                        check_text_chunk(keyword, value, &mut signals);
                    }
                }
            }
            b"iTXt" => {
                if let Some(null_pos) = chunk_data.iter().position(|&b| b == 0) {
                    if let Ok(keyword) = std::str::from_utf8(&chunk_data[..null_pos]) {
                        let rest = &chunk_data[null_pos + 1..];
                        let mut nulls_found = 0;
                        let mut text_start = 0;
                        for (i, &b) in rest.iter().enumerate() {
                            if b == 0 {
                                nulls_found += 1;
                                if nulls_found == 3 {
                                    text_start = i + 1;
                                    break;
                                }
                            }
                        }
                        if text_start > 0 && text_start < rest.len() {
                            if let Ok(value) = std::str::from_utf8(&rest[text_start..]) {
                                check_text_chunk(keyword, value, &mut signals);
                            }
                        }
                    }
                }
            }
            b"IEND" => break,
            _ => {}
        }
        pos = chunk_data_end + 4;
    }
    Ok(signals)
}

fn check_text_chunk(keyword: &str, value: &str, signals: &mut Vec<Signal>) {
    let keyword_lower = keyword.to_lowercase();
    let is_relevant = RELEVANT_KEYWORDS
        .iter()
        .any(|k| keyword_lower == k.to_lowercase());
    if !is_relevant {
        return;
    }
    if let Some(tool_name) = known_tools::match_ai_tool(value) {
        signals.push(
            SignalBuilder::new(
                SignalSource::PngText,
                Confidence::Low,
                "signal_png_text_chunk",
            )
            .param("keyword", keyword)
            .tool(tool_name)
            .detail(keyword, truncate(value, 200))
            .build(),
        );
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
