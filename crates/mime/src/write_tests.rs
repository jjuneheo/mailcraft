// SPDX-FileCopyrightText: MailCraft contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::write::file_name_params;
use crate::{Attachment, Body, Message, WriteOptions, decode_words, parse, write};
use std::sync::Arc;

/// The quoted value of the plain `filename="…"` parameter, as written.
fn plain_filename(params: &str) -> &str {
    let rest = params.strip_prefix("; filename=\"").unwrap_or("");
    rest.split('"').next().unwrap_or("")
}

/// What a reader that skips RFC 2231 shows: the plain `filename`, encoded-words decoded.
fn shown_without_rfc2231(params: &str) -> String {
    decode_words(plain_filename(params))
}

#[test]
fn plain_filename_carries_the_whole_name_with_its_extension() {
    for name in [
        format!("{}.pdf", "é".repeat(100)),
        format!("{}.docx", "Quarterly-report-".repeat(20)),
        "40-2026-0012345_등록공보_테스트.pdf".to_string(),
        "日本語.pdf".to_string(),
        "日本語".to_string(),
        "ü.v1.2.pdf".to_string(),
        format!("{}.tar.gz", "ü".repeat(100)),
        "rapport.dokümént".to_string(),
    ] {
        let p = file_name_params(&name);
        assert_eq!(shown_without_rfc2231(&p), name, "{p}");
        assert!(plain_filename(&p).is_ascii(), "{p}");
    }
}

#[test]
fn plain_filename_words_fit_a_folded_line() {
    for name in [format!("ü.{}", "x".repeat(500)), "日本語".repeat(80), "ü".repeat(100)] {
        let p = file_name_params(&name);
        for word in plain_filename(&p).split(' ') {
            assert!(word.len() + 2 <= 78 && word.starts_with("=?UTF-8?B?"), "{word}");
        }
    }
}

#[test]
fn plain_filename_of_a_korean_name_is_not_underscores() {
    let p = file_name_params("등록공보.pdf");
    assert!(plain_filename(&p).starts_with("=?UTF-8?B?") && !plain_filename(&p).contains('_'), "{p}");
    assert!(p.contains("filename*=utf-8''%EB%93%B1%EB%A1%9D%EA%B3%B5%EB%B3%B4.pdf"), "{p}");
}

#[test]
fn encoded_name_is_whole_and_ascii() {
    let name = format!("{}.pdf", "日本語".repeat(40));
    let p = file_name_params(&name);
    assert!(p.is_ascii() && p.contains("filename*0*=utf-8''"), "{p}");
}

#[test]
fn written_attachment_survives_hostile_names() {
    for name in ["rés\r\numé\0.pdf", "\r\n\0", &format!("{}.docx", "ß".repeat(300))] {
        let mut m = Message { body: Body { text: "hi".into(), html: None }, ..Message::default() };
        m.attachments = vec![Attachment {
            name: name.into(),
            mime: "application/pdf".into(),
            content_id: String::new(),
            inline: false,
            data: Arc::new(vec![1, 2, 3]),
        }];
        let out = write(&m, &WriteOptions::default());
        let text = String::from_utf8_lossy(&out);
        assert!(!text.contains("\0") && !text.lines().any(|l| l.starts_with("\0")));
        assert!(!text.contains("\r\numé"), "header injection");
        let back = parse(&out);
        let clean: String = name.chars().filter(|c| !matches!(c, '\r' | '\n' | '\0')).collect();
        if !clean.is_empty() {
            assert_eq!(back.attachments.first().map(|a| a.name.as_str()), Some(clean.as_str()));
        }
    }
}
