// src/markdown.rs
use crate::models::{FocusGraph, FocusNode, NodeStatus};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Result as IoResult, Write};

use uuid::Uuid;

// =============================================================================
// Sabitler (Constants)
// =============================================================================
const HEADER_TITLE: &str = "# Uslu Focus Tree\n";
const SECTION_PREFIX: &str = "## ";
const ITEM_PREFIX: char = '-';
const KEY_VALUE_SEPARATOR: char = ':';
const QUOTE_CHAR: char = '"';
const BACKSLASH_CHAR: char = '\\';
const NEWLINE_CHAR: char = '\n';

const KEY_ID: &str = "id";
const KEY_DESCRIPTION: &str = "description";
const KEY_PROGRESS_NOTES: &str = "progress_notes";
const KEY_IMAGE_ID: &str = "image_id";
const KEY_STATUS: &str = "status";
const KEY_PROGRESS: &str = "progress";
const KEY_POSITION: &str = "position";
const KEY_PREREQUISITES: &str = "prerequisites";
const KEY_IS_COLLAPSED: &str = "is_collapsed";

const MIN_PROGRESS_PERCENT: f32 = 0.0;
const MAX_PROGRESS_PERCENT: f32 = 100.0;
const DEFAULT_COORDINATE: f32 = 0.0;

const POSITION_ARRAY_DELIMITERS: &[char] = &['[', ']'];

struct RawNode {
    title: String,
    kv: HashMap<String, String>,
}

struct ProcessedNode {
    node: FocusNode,
    prereq_uuids: Vec<String>,
}

pub struct MarkdownIO;

impl MarkdownIO {
    pub fn export(graph: &FocusGraph, path: &str) -> IoResult<()> {
        let tmp_path = format!("{}.tmp", path);
        {
            let mut file = File::create(&tmp_path)?;
            writeln!(file, "{}", HEADER_TITLE)?;

            for node in &graph.nodes {
                write_single_node_to_file(&mut file, node, graph)?;
            }
            file.flush()?;
            file.sync_all()?;
        }
        std::fs::rename(tmp_path, path)?;
        Ok(())
    }

    pub fn import(path: &str) -> Result<FocusGraph, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Dosya okunamadı: {}", e))?;
        Self::parse(&content)
    }

    fn parse(content: &str) -> Result<FocusGraph, String> {
        let raw_nodes = parse_raw_nodes_from_content(content);
        let processed_nodes = process_raw_nodes(raw_nodes);

        let mut graph = FocusGraph::default();
        populate_graph_nodes(&mut graph, &processed_nodes);
        populate_graph_edges(&mut graph, &processed_nodes);

        Ok(graph)
    }
}

fn write_single_node_to_file(
    file: &mut File,
    node: &FocusNode,
    graph: &FocusGraph,
) -> IoResult<()> {
    writeln!(file, "{}{}\n", SECTION_PREFIX, escape_string(&node.title))?;
    writeln!(file, "- id: \"{}\"", node.id)?;
    writeln!(
        file,
        "- description: \"{}\"",
        escape_string(&node.description)
    )?;
    writeln!(
        file,
        "- progress_notes: \"{}\"",
        escape_string(&node.progress_notes)
    )?;

    if let Some(img_id) = &node.image_id {
        writeln!(file, "- image_id: \"{}\"", img_id)?;
    }

    writeln!(file, "- status: {:.1}", node.status.progress)?;
    writeln!(file, "- is_collapsed: {}", node.is_collapsed)?;
    writeln!(file, "- is_frozen: {}", node.is_frozen)?; // ← YENİ EKLENDİ (C.1 Fix)

    let parents_formatted = format_parent_uuids(graph, node.id);
    writeln!(file, "- prerequisites: [{}]", parents_formatted)?;
    writeln!(file, "- position: [{:.1}, {:.1}]\n", node.x, node.y)?;

    Ok(())
}

fn format_parent_uuids(graph: &FocusGraph, node_id: Uuid) -> String {
    graph
        .parents_of(node_id)
        .into_iter()
        .map(|parent_id| format!("\"{}\"", parent_id))
        .collect::<Vec<String>>()
        .join(", ")
}

fn parse_raw_nodes_from_content(content: &str) -> Vec<RawNode> {
    let mut raw_nodes = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_kv: HashMap<String, String> = HashMap::new();
    let mut current_multiline_key: Option<String> = None;
    let mut current_multiline_val = String::new();

    for line in content.lines() {
        if handle_multiline_continuation(
            line,
            &mut current_multiline_key,
            &mut current_multiline_val,
            &mut current_kv,
        ) {
            continue;
        }

        let trimmed = line.trim();
        if is_ignorable_line(trimmed) {
            continue;
        }

        if let Some(new_title) = trimmed.strip_prefix(SECTION_PREFIX) {
            finalize_previous_node(&mut raw_nodes, &mut current_title, &mut current_kv);
            current_title = Some(unescape_string(new_title.trim()));
            continue;
        }

        if let Some(list_item) = trimmed.strip_prefix(ITEM_PREFIX) {
            parse_key_value_pair(
                list_item,
                &mut current_multiline_key,
                &mut current_multiline_val,
                &mut current_kv,
            );
        }
    }

    flush_multiline_remainder(current_multiline_key, current_multiline_val, &mut current_kv);
    finalize_previous_node(&mut raw_nodes, &mut current_title, &mut current_kv);

    raw_nodes
}

fn handle_multiline_continuation(
    line: &str,
    multiline_key: &mut Option<String>,
    multiline_val: &mut String,
    kv: &mut HashMap<String, String>,
) -> bool {
    if let Some(key) = multiline_key.take() {
        multiline_val.push(NEWLINE_CHAR);
        multiline_val.push_str(line);

        if is_value_unclosed_multiline(multiline_val) {
            *multiline_key = Some(key);
        } else {
            kv.insert(key, multiline_val.clone());
            multiline_val.clear();
        }
        return true;
    }
    false
}

fn is_ignorable_line(trimmed: &str) -> bool {
    trimmed.is_empty() || (trimmed.starts_with('#') && !trimmed.starts_with(SECTION_PREFIX))
}

fn finalize_previous_node(
    raw_nodes: &mut Vec<RawNode>,
    title_opt: &mut Option<String>,
    kv: &mut HashMap<String, String>,
) {
    if let Some(title) = title_opt.take() {
        raw_nodes.push(RawNode {
            title,
            kv: std::mem::take(kv),
        });
    }
}

fn parse_key_value_pair(
    list_item: &str,
    multiline_key: &mut Option<String>,
    multiline_val: &mut String,
    kv: &mut HashMap<String, String>,
) {
    if let Some((raw_key, raw_val)) = list_item.trim().split_once(KEY_VALUE_SEPARATOR) {
        let key = raw_key.trim().to_string();
        let val = raw_val.trim().to_string();

        if is_value_unclosed_multiline(&val) {
            *multiline_key = Some(key);
            *multiline_val = val;
        } else {
            kv.insert(key, val);
        }
    }
}

fn flush_multiline_remainder(
    multiline_key: Option<String>,
    multiline_val: String,
    kv: &mut HashMap<String, String>,
) {
    if let Some(key) = multiline_key {
        kv.insert(key, multiline_val);
    }
}

fn process_raw_nodes(raw_nodes: Vec<RawNode>) -> Vec<ProcessedNode> {
    raw_nodes.into_iter().map(build_processed_node).collect()
}


fn build_processed_node(raw: RawNode) -> ProcessedNode {
    let id = extract_node_id(&raw.kv);
    let description = extract_clean_string(&raw.kv, KEY_DESCRIPTION);
    let progress_notes = extract_clean_string(&raw.kv, KEY_PROGRESS_NOTES);
    let image_id = extract_uuid_opt(&raw.kv, KEY_IMAGE_ID);

    let status = extract_node_status(&raw.kv);
    let position = extract_position_coordinates(&raw.kv);
    let prereq_uuids = extract_prerequisites_list(&raw.kv);
    let is_collapsed = extract_bool(&raw.kv, KEY_IS_COLLAPSED);
    let is_frozen = extract_bool(&raw.kv, "is_frozen");

    let mut node = FocusNode::new(raw.title, description);
    node.id = id;
    node.progress_notes = progress_notes;
    node.image_id = image_id;
    node.status = status;
    node.x = position.0;
    node.y = position.1;
    node.is_collapsed = is_collapsed;
    node.is_frozen = is_frozen;

    ProcessedNode { node, prereq_uuids }
}

fn extract_node_id(kv: &HashMap<String, String>) -> Uuid {
    kv.get(KEY_ID)
        .and_then(|s| Uuid::parse_str(s.trim_matches(QUOTE_CHAR)).ok())
        .unwrap_or_else(Uuid::new_v4)
}

fn extract_clean_string(kv: &HashMap<String, String>, key: &str) -> String {
    kv.get(key)
        .map(|s| unescape_string(s.trim_matches(QUOTE_CHAR)))
        .unwrap_or_default()
}

fn extract_uuid_opt(kv: &HashMap<String, String>, key: &str) -> Option<Uuid> {
    kv.get(key)
        .and_then(|s| Uuid::parse_str(s.trim_matches(QUOTE_CHAR)).ok())
}

fn extract_bool(kv: &HashMap<String, String>, key: &str) -> bool {
    kv.get(key)
        .map(|s| s.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn extract_node_status(kv: &HashMap<String, String>) -> NodeStatus {
    kv.get(KEY_STATUS)
        .or_else(|| kv.get(KEY_PROGRESS))
        .map(|s| parse_status(s))
        .unwrap_or_default()
}

fn extract_position_coordinates(kv: &HashMap<String, String>) -> (f32, f32) {
    kv.get(KEY_POSITION)
        .and_then(|s| parse_position(s))
        .unwrap_or((DEFAULT_COORDINATE, DEFAULT_COORDINATE))
}

fn extract_prerequisites_list(kv: &HashMap<String, String>) -> Vec<String> {
    kv.get(KEY_PREREQUISITES)
        .map(|s| extract_uuid_list(s))
        .unwrap_or_default()
}

fn populate_graph_nodes(graph: &mut FocusGraph, processed_nodes: &[ProcessedNode]) {
    for item in processed_nodes {
        graph.add_node(item.node.clone());
    }
}

fn populate_graph_edges(graph: &mut FocusGraph, processed_nodes: &[ProcessedNode]) {
    for item in processed_nodes {
        for parent_uuid_str in &item.prereq_uuids {
            if let Ok(parent_id) = Uuid::parse_str(parent_uuid_str.trim_matches(QUOTE_CHAR)) {
                graph.add_edge(parent_id, item.node.id);
            }
        }
    }
}

fn parse_status(s: &str) -> NodeStatus {
    let cleaned = s.trim_matches(QUOTE_CHAR);
    if let Ok(progress) = cleaned.parse::<f32>() {
        if progress.is_finite() {
            return NodeStatus {
                progress: progress.clamp(MIN_PROGRESS_PERCENT, MAX_PROGRESS_PERCENT),
            };
        }
    }
    NodeStatus {
        progress: MIN_PROGRESS_PERCENT,
    }
}

fn parse_position(s: &str) -> Option<(f32, f32)> {
    let cleaned = s.trim_matches(POSITION_ARRAY_DELIMITERS);
    let parts: Vec<&str> = cleaned.split(',').collect();

    if parts.len() == 2 {
        if let (Ok(x), Ok(y)) = (
            parts[0].trim().parse::<f32>(),
            parts[1].trim().parse::<f32>(),
        ) {
            if x.is_finite() && y.is_finite() {
                return Some((x, y));
            }
        }
    }
    None
}

fn extract_uuid_list(s: &str) -> Vec<String> {
    let cleaned = s.trim_matches(POSITION_ARRAY_DELIMITERS);
    cleaned
        .split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

fn escape_string(s: &str) -> String {
    s.replace(BACKSLASH_CHAR, "\\\\")
        .replace(QUOTE_CHAR, "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn unescape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == BACKSLASH_CHAR {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some(other) => {
                    result.push(BACKSLASH_CHAR);
                    result.push(other);
                }
                None => result.push(BACKSLASH_CHAR),
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn is_value_unclosed_multiline(s: &str) -> bool {
    let trimmed = s.trim();
    if !trimmed.starts_with(QUOTE_CHAR) {
        return false;
    }

    let mut quote_count = 0;
    let mut chars = trimmed.chars().peekable();
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if escaped {
            escaped = false;
        } else if c == BACKSLASH_CHAR {
            escaped = true;
        } else if c == QUOTE_CHAR {
            quote_count += 1;
        }
    }

    quote_count % 2 == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiline_description_export_import() {
        let mut graph = FocusGraph::default();
        let mut node = FocusNode::new(
            "Test Node".to_string(),
            "Line 1\nLine 2\nLine 3".to_string(),
        );
        node.progress_notes = "Note line 1\nNote line 2".to_string();
        graph.add_node(node);

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_uslu_multiline.md");
        let temp_path_str = temp_path.to_str().unwrap();

        MarkdownIO::export(&graph, temp_path_str).unwrap();

        let imported_graph = MarkdownIO::import(temp_path_str).unwrap();
        let imported_node = &imported_graph.nodes[0];

        assert_eq!(imported_node.description, "Line 1\nLine 2\nLine 3");
        assert_eq!(imported_node.progress_notes, "Note line 1\nNote line 2");

        let _ = std::fs::remove_file(temp_path);
    }
}
