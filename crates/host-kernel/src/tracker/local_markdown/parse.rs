use super::*;
use std::collections::BTreeMap;

pub(crate) struct LocalReference {
    pub(crate) raw: String,
    pub(crate) file: Option<String>,
    pub(crate) number: Option<u64>,
    pub(crate) title: String,
}

pub(crate) fn normalize_metadata(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn parse_field_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let trimmed = trimmed.strip_prefix("**").unwrap_or(trimmed);
    let (name, value) = trimmed.split_once(':')?;
    let name = name.strip_suffix("**").unwrap_or(name).trim();
    if name.is_empty() || !name.chars().next()?.is_ascii_alphabetic() {
        return None;
    }
    Some((
        normalize_metadata(name),
        value.trim().trim_matches('*').trim().to_string(),
    ))
}

pub(crate) fn is_local_metadata_field(name: &str) -> bool {
    matches!(
        name,
        "status"
            | "type"
            | "assignee"
            | "assignees"
            | "part of"
            | "parent"
            | "blocked by"
            | "closed"
    )
}

pub(crate) fn header_fields(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut fields = BTreeMap::<String, Vec<String>>::new();
    for line in text.lines() {
        if line.trim_start().starts_with("## ") {
            break;
        }
        if let Some((name, value)) = parse_field_line(line) {
            fields.entry(name).or_default().push(value);
        }
    }
    fields
}

pub(crate) fn section_lines(text: &str, heading: &str) -> Vec<String> {
    let wanted = normalize_metadata(heading);
    let mut in_section = false;
    let mut result = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(section) = trimmed.strip_prefix("## ") {
            let current = normalize_metadata(section);
            if in_section && current != wanted {
                break;
            }
            in_section = current == wanted;
            continue;
        }
        if in_section {
            result.push(line.to_string());
        }
    }
    result
}

pub(crate) fn blocked_by_reference_lines(text: &str) -> Vec<String> {
    let section = section_lines(text, "Blocked by");
    if !section.is_empty() {
        return section;
    }
    header_fields(text)
        .get("blocked by")
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn normalize_reference_title(value: &str) -> String {
    normalize_metadata(
        value
            .trim()
            .trim_start_matches(['-', '*', '+'])
            .trim()
            .trim_matches(['`', '*', '_', '.', ',', ';', ':'])
            .trim_start_matches(['—', '–', ':', '-'])
            .trim(),
    )
}

pub(crate) fn parse_reference(raw: &str) -> Option<LocalReference> {
    let raw = raw.trim();
    if raw.is_empty() || normalize_metadata(raw).starts_with("none") {
        return None;
    }
    let cleaned = raw
        .trim_start_matches(['-', '*', '+'])
        .trim()
        .trim_matches(['`', '*', '_'])
        .trim();
    if cleaned.is_empty() || normalize_metadata(cleaned).starts_with("none") {
        return None;
    }
    if let Some(start) = cleaned.find('(') {
        if let Some(end) = cleaned[start + 1..].find(')') {
            let target = &cleaned[start + 1..start + 1 + end];
            let file = target
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(target)
                .to_string();
            return Some(LocalReference {
                raw: raw.into(),
                file: Some(file),
                number: None,
                title: String::new(),
            });
        }
    }
    let digits = cleaned.strip_prefix('#').unwrap_or(cleaned);
    let digit_end = digits
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(digits.len());
    if digit_end == 0 {
        return Some(LocalReference {
            raw: raw.into(),
            file: None,
            number: None,
            title: String::new(),
        });
    }
    let number = digits[..digit_end].parse::<u64>().ok();
    let rest = digits[digit_end..].trim();
    let title = rest
        .strip_prefix(['—', '–', ':', '-'])
        .unwrap_or(rest)
        .trim();
    let file = cleaned
        .split_whitespace()
        .find(|part| part.to_ascii_lowercase().ends_with(".md"))
        .map(|part| part.trim_matches(['`', '*', '_', ',', ';']).to_string());
    Some(LocalReference {
        raw: raw.into(),
        file,
        number,
        title: normalize_reference_title(title),
    })
}

pub(crate) fn parse_reference_line(raw: &str) -> Vec<LocalReference> {
    let raw = raw.trim();
    if raw.is_empty() || normalize_metadata(raw).starts_with("none") {
        return Vec::new();
    }
    let mut parts = raw
        .split([',', ';'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() == 1 {
        // A markdown link may contain commas in its label; parse it as one token.
        parts = vec![raw];
    }
    parts.into_iter().filter_map(parse_reference).collect()
}

pub(crate) fn local_slug(title: &str) -> String {
    let mut slug = String::new();
    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "issue".into()
    } else {
        slug.chars().take(48).collect()
    }
}

pub(crate) fn graph_cycle_node(adjacency: &BTreeMap<String, Vec<String>>) -> Option<String> {
    fn visit(
        node: &str,
        adjacency: &BTreeMap<String, Vec<String>>,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if visiting.contains(node) {
            return true;
        }
        if !visited.insert(node.to_string()) {
            return false;
        }
        visiting.insert(node.to_string());
        let cycle = adjacency
            .get(node)
            .into_iter()
            .flatten()
            .any(|neighbor| visit(neighbor, adjacency, visiting, visited));
        visiting.remove(node);
        cycle
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    adjacency
        .keys()
        .find(|node| visit(node, adjacency, &mut visiting, &mut visited))
        .cloned()
}
