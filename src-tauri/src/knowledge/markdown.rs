use serde::Serialize;
use serde_yaml::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedNote {
    pub title: String,
    pub tags: Vec<String>,
    pub item_id: Option<String>,
    pub analysis_id: Option<String>,
    pub links: Vec<String>,
    pub warnings: Vec<String>,
}
fn scalar(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= 2048 && !s.contains('\0'))
        .map(str::to_owned)
}
fn tag_chars(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '/')
}
fn add_tag(tags: &mut BTreeSet<String>, text: &str) {
    let tag = text.trim().trim_start_matches('#');
    if tags.len() < 256
        && !tag.is_empty()
        && tag.len() <= 256
        && tag.chars().all(tag_chars)
        && tag.chars().any(char::is_alphabetic)
    {
        tags.insert(tag.into());
    }
}
// Index only ordinary scalar/list metadata; aliases can multiply allocations despite a byte limit.
fn yaml_has_references(yaml: &str) -> bool {
    for line in yaml.lines() {
        let mut quote = None;
        let mut escaped = false;
        let mut chars = line.chars().peekable();
        let mut previous = None;
        while let Some(c) = chars.next() {
            if let Some(q) = quote {
                if q == '"' && !escaped && c == '\\' {
                    escaped = true;
                    continue;
                }
                if escaped {
                    escaped = false;
                    continue;
                }
                if c == q {
                    if q == '\'' && chars.peek() == Some(&'\'') {
                        chars.next();
                    } else {
                        quote = None;
                    }
                }
            } else {
                let boundary = previous.is_none_or(|p: char| {
                    p.is_whitespace() || matches!(p, '[' | '{' | ':' | ',' | '-')
                });
                if boundary && c == '#' {
                    break;
                }
                if boundary && matches!(c, '\'' | '"') {
                    quote = Some(c);
                }
                if boundary && matches!(c, '*' | '&') {
                    return true;
                }
            }
            previous = Some(c);
        }
    }
    false
}
pub fn parse_note(path: &str, text: &str) -> ParsedNote {
    let mut result = ParsedNote {
        title: String::new(),
        tags: Vec::new(),
        item_id: None,
        analysis_id: None,
        links: Vec::new(),
        warnings: Vec::new(),
    };
    let mut body = text.trim_start_matches('\u{feff}');
    let mut tags = BTreeSet::new();
    let mut links = BTreeSet::new();
    if body.lines().next() == Some("---") {
        let start = body.find('\n').map(|i| i + 1).unwrap_or(body.len());
        let mut end = start;
        let mut closing = None;
        for line in body[start..].split_inclusive('\n') {
            if line.trim_end_matches(['\r', '\n']) == "---" {
                closing = Some((end, end + line.len()));
                break;
            }
            end += line.len();
            if end > 65536 {
                break;
            }
        }
        if let Some((end, after)) = closing {
            let yaml = &body[start..end];
            body = &body[after..];
            if yaml_has_references(yaml) {
                result
                    .warnings
                    .push("Frontmatter 含锚点或别名，已保留原文并跳过元数据解析".into());
            } else {
                match serde_yaml::from_str::<Value>(yaml) {
                    Ok(Value::Mapping(map)) => {
                        let get = |key: &str| map.get(Value::String(key.into()));
                        result.title = scalar(get("title")).unwrap_or_default();
                        result.item_id = scalar(get("item_id"));
                        result.analysis_id = scalar(get("analysis_id"));
                        match get("tags") {
                            Some(Value::Sequence(items)) => {
                                for item in items.iter().take(256) {
                                    if let Some(s) = item.as_str() {
                                        add_tag(&mut tags, s);
                                    }
                                }
                            }
                            Some(Value::String(s)) => {
                                for tag in s.split(|c: char| c.is_whitespace() || c == ',') {
                                    add_tag(&mut tags, tag);
                                }
                            }
                            Some(_) => result
                                .warnings
                                .push("Frontmatter tags 格式无效，已忽略".into()),
                            None => {}
                        }
                    }
                    Ok(Value::Null) => {}
                    _ => result
                        .warnings
                        .push("Frontmatter YAML 无效，已使用正文和文件名建立索引".into()),
                }
            }
        } else {
            result
                .warnings
                .push("Frontmatter 未闭合或超过 64 KiB，已使用正文建立索引".into());
        }
    }
    let mut fence: Option<(char, usize)> = None;
    let mut inline_ticks = 0;
    for line in body.lines() {
        let trimmed = line.trim_start();
        let first = trimmed.chars().next().unwrap_or(' ');
        let run = trimmed.chars().take_while(|&c| c == first).count();
        if let Some((marker, size)) = fence {
            if first == marker && run >= size && trimmed[run..].trim().is_empty() {
                fence = None;
            }
            continue;
        }
        if matches!(first, '`' | '~') && run >= 3 {
            fence = Some((first, run));
            continue;
        }
        if inline_ticks == 0 && result.title.is_empty() {
            let heading = trimmed.trim_start_matches('#');
            if heading.len() < trimmed.len()
                && heading.starts_with(' ')
                && trimmed.len() - heading.len() <= 6
            {
                result.title = heading.trim().chars().take(512).collect();
            }
        }
        let mut i = 0;
        while i < line.len() {
            let rest = &line[i..];
            let c = rest.chars().next().unwrap();
            if c == '\\' {
                i += c.len_utf8();
                if i < line.len() {
                    i += line[i..].chars().next().unwrap().len_utf8();
                }
                continue;
            }
            if c == '`' {
                let count = rest.bytes().take_while(|&b| b == b'`').count();
                if inline_ticks == 0 {
                    inline_ticks = count;
                } else if inline_ticks == count {
                    inline_ticks = 0;
                }
                i += count;
                continue;
            }
            if inline_ticks != 0 {
                i += c.len_utf8();
                continue;
            }
            if rest.starts_with("[[") {
                if let Some(end) = rest[2..].find("]]") {
                    if !line[..i].ends_with('!') && links.len() < 2048 {
                        if let Some(target) = super::links::normalize_target(&rest[2..2 + end]) {
                            links.insert(target);
                        }
                    }
                    i += end + 4;
                    continue;
                }
            }
            let previous = line[..i].chars().next_back();
            if c == '#'
                && previous.is_none_or(|c| {
                    c.is_whitespace() || matches!(c, '(' | '[' | '{' | '>' | ',' | ';')
                })
            {
                let tag: String = rest[1..].chars().take_while(|&c| tag_chars(c)).collect();
                add_tag(&mut tags, &tag);
                i += 1 + tag.len();
                continue;
            }
            i += c.len_utf8();
        }
    }
    if result.title.is_empty() {
        result.title = path
            .rsplit('/')
            .next()
            .unwrap_or(path)
            .strip_suffix(".md")
            .unwrap_or(path.rsplit('/').next().unwrap_or(path))
            .chars()
            .take(512)
            .collect();
    }
    result.tags = tags.into_iter().collect();
    result.links = links.into_iter().collect();
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_ignores_code_images_and_handles_real_yaml() {
        let p=parse_note("f.md","---\ntitle: 'Title: 中文'\ntags: [rust, '#学习']\nitem_id: missing\n---\n[[Folder/Note#Head|Alias]] ![[image.png]] #inline/tag\n`[[Code]] #fake`\n```md\n[[Fence]] #false\n```\n");
        assert_eq!(p.title, "Title: 中文");
        assert_eq!(p.links, vec!["Folder/Note"]);
        assert_eq!(p.tags, vec!["inline/tag", "rust", "学习"]);
        let bad = parse_note("f.md", "---\ntitle: [\n---\n# Actual\n[[Real]]");
        assert_eq!(bad.title, "Actual");
        assert!(!bad.warnings.is_empty());
    }
}

#[cfg(test)]
mod bounds_tests {
    #[test]
    fn frontmatter_aliases_are_not_expanded_and_oversized_metadata_is_warned() {
        let p = super::parse_note(
            "safe.md",
            "---\ntitle: &name 'Title'\ntags: [*name, *name]\n---\n# Safe heading\n[[Actual]]",
        );
        assert_eq!(p.title, "Safe heading");
        assert!(!p.warnings.is_empty());
        assert_eq!(p.links, vec!["Actual"]);
        let huge = format!("---\ntitle: {}\n---\n# Body", "x".repeat(65536));
        assert!(!super::parse_note("safe.md", &huge).warnings.is_empty());
        let quoted = super::parse_note(
            "safe.md",
            "---\ntitle: 'Learn *pointer and &reference'\n---\n",
        );
        assert_eq!(quoted.title, "Learn *pointer and &reference");
        assert!(quoted.warnings.is_empty());
    }
}
