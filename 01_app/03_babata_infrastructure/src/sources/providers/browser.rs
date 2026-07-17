use std::{fs, path::Path};

use babata_application::ApplicationError;
use babata_domain::{CapabilityStatus, SourceRouteDescriptor, SourceRouteId};

#[derive(Debug, Clone, Default)]
pub struct BrowserConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    SourceRouteDescriptor {
        id: SourceRouteId("source.browser".to_owned()),
        provider: "browser".to_owned(),
        status: CapabilityStatus::Disabled,
        activation_phase: "P4".to_owned(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserBookmark {
    pub title: String,
    pub url: String,
    pub folder_path: String,
}

pub fn read_netscape_bookmarks(path: &Path) -> Result<Vec<BrowserBookmark>, ApplicationError> {
    let html = fs::read_to_string(path).map_err(|error| {
        ApplicationError::Asset(format!(
            "unable to read bookmark export: {:?}",
            error.kind()
        ))
    })?;
    let mut bookmarks = Vec::new();
    let mut folders = Vec::new();
    let mut pending_folder = None;
    let mut cursor = 0;
    while let Some((start, end, tag)) = next_tag(&html, cursor) {
        cursor = end;
        let lower = tag.to_ascii_lowercase();
        if lower.starts_with("<h3") {
            if let Some(close) = find_case_insensitive(&html[end..], "</h3>") {
                pending_folder = Some(decode_html(&strip_tags(&html[end..end + close])));
                cursor = end + close + 5;
            }
        } else if lower.starts_with("<dl") {
            if let Some(folder) = pending_folder.take() {
                folders.push(folder);
            }
        } else if lower.starts_with("</dl") {
            let _ = folders.pop();
        } else if lower.starts_with("<a") {
            let url = attribute_value(&tag, "href");
            if let (Some(url), Some(close)) = (url, find_case_insensitive(&html[end..], "</a>")) {
                let title = decode_html(&strip_tags(&html[end..end + close]));
                if !title.is_empty() && !url.is_empty() {
                    bookmarks.push(BrowserBookmark {
                        title,
                        url,
                        folder_path: folders.join(" / "),
                    });
                }
                cursor = end + close + 4;
            }
        }
        if start == end {
            break;
        }
    }
    if bookmarks.is_empty() {
        return Err(ApplicationError::Asset(
            "bookmark export contains no usable links".to_owned(),
        ));
    }
    Ok(bookmarks)
}

fn next_tag(input: &str, cursor: usize) -> Option<(usize, usize, String)> {
    let start = input[cursor..].find('<')? + cursor;
    let end = input[start..].find('>')? + start + 1;
    Some((start, end, input[start..end].to_owned()))
}

fn find_case_insensitive(input: &str, needle: &str) -> Option<usize> {
    input
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
}

fn attribute_value(tag: &str, name: &str) -> Option<String> {
    let bytes = tag.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        while index < bytes.len() && (bytes[index].is_ascii_whitespace() || bytes[index] == b'<') {
            index += 1;
        }
        let start = index;
        while index < bytes.len()
            && !bytes[index].is_ascii_whitespace()
            && bytes[index] != b'='
            && bytes[index] != b'>'
        {
            index += 1;
        }
        if start == index {
            index += 1;
            continue;
        }
        let key = &tag[start..index];
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'=' {
            continue;
        }
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let value = if index < bytes.len() && matches!(bytes[index], b'\'' | b'\"') {
            let quote = bytes[index];
            index += 1;
            let value_start = index;
            while index < bytes.len() && bytes[index] != quote {
                index += 1;
            }
            tag[value_start..index].to_owned()
        } else {
            let value_start = index;
            while index < bytes.len() && !bytes[index].is_ascii_whitespace() && bytes[index] != b'>'
            {
                index += 1;
            }
            tag[value_start..index].to_owned()
        };
        if key.eq_ignore_ascii_case(name) {
            return Some(decode_html(&value));
        }
    }
    None
}

fn strip_tags(input: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    for character in input.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(character),
            _ => {}
        }
    }
    output
}

fn decode_html(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn netscape_export_keeps_title_url_and_folder() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("bookmarks.html");
        fs::write(
            &path,
            "<DL><p><DT><H3>Reading</H3><DL><p><DT><A HREF=\"https://example.test/a?x=1&amp;y=2\">A &amp; B</A></DL></DL>",
        )
        .unwrap();
        assert_eq!(
            read_netscape_bookmarks(&path).unwrap(),
            vec![BrowserBookmark {
                title: "A & B".to_owned(),
                url: "https://example.test/a?x=1&y=2".to_owned(),
                folder_path: "Reading".to_owned(),
            }]
        );
    }
}
