// src/markdown.rs
use crate::models::{FocusGraph, FocusNode, NodeStatus};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Result as IoResult, Write};
use uuid::Uuid;

// =============================================================================
// Sabitler (Org-Mode Dönüşüm Sabitleri)
// =============================================================================
const HEADER_TITLE: &str = "#+TITLE: Uslu Focus Tree\n";
const NODE_SECTION_PREFIX: &str = "** ";
const DESC_SECTION_PREFIX: &str = "*** description:";
const NOTES_SECTION_PREFIX: &str = "*** progress_notes:";
const ITEM_PREFIX: char = '-';
const KEY_VALUE_SEPARATOR: char = ':';
const QUOTE_CHAR: char = '"';

const KEY_ID: &str = "id";
const KEY_STATUS: &str = "status";
const KEY_PROGRESS: &str = "progress";
const KEY_POSITION: &str = "position";
const KEY_PREREQUISITES: &str = "prerequisites";
const KEY_IS_COLLAPSED: &str = "is_collapsed";
const KEY_IS_FROZEN: &str = "is_frozen";

const MIN_PROGRESS_PERCENT: f32 = 0.0;
const MAX_PROGRESS_PERCENT: f32 = 100.0;
const DEFAULT_COORDINATE: f32 = 0.0;

const POSITION_ARRAY_DELIMITERS: &[char] = &['[', ']'];

// Org-mode Hiyerarşi & Girintileme Sabitleri
const ORG_INDENT_OFFSET_SPACES: &str = "      "; // 6 boşluk
const ORG_HEADER_OFFSET_ASTERISKS: &str = "***";  // 3 yıldız ofseti (* -> ****)

struct RawNode {
    title: String,
    description: String,
    progress_notes: String,
    kv: HashMap<String, String>,
}

struct ProcessedNode {
    node: FocusNode,
    prereq_uuids: Vec<String>,
}

pub struct OrgmodeIO;

impl OrgmodeIO {
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
    // 1. Düğüm Başlığı (** Node Title)
    writeln!(file, "{}{}\n", NODE_SECTION_PREFIX, node.title)?;
    writeln!(file, "- id: \"{}\"", node.id)?;

    // 2. Description Alanı (*** description:)
    writeln!(file, "{}", DESC_SECTION_PREFIX)?;
    let formatted_desc = encode_org_content_indentation(&node.description);
    if !formatted_desc.is_empty() {
        writeln!(file, "{}", formatted_desc)?;
    }
    writeln!(file)?;

    // 3. Progress Notes Alanı (*** progress_notes:)
    writeln!(file, "{}", NOTES_SECTION_PREFIX)?;
    let formatted_notes = encode_org_content_indentation(&node.progress_notes);
    if !formatted_notes.is_empty() {
        writeln!(file, "{}", formatted_notes)?;
    }
    writeln!(file)?;

    // 4. Diğer Metadata Key-Value Değerleri (- key: value)
    if let Some(img_id) = &node.image_id {
        writeln!(file, "- image_id: \"{}\"", img_id)?;
    }

    writeln!(file, "- status: {:.1}", node.status.progress)?;
    writeln!(file, "- is_collapsed: {}", node.is_collapsed)?;
    writeln!(file, "- is_frozen: {}", node.is_frozen)?;

    let parents_formatted = format_parent_uuids(graph, node.id);
    writeln!(file, "- prerequisites: [{}]", parents_formatted)?;
    writeln!(file, "- position: [{:.1}, {:.1}]\n", node.x, node.y)?;

    Ok(())
}

fn encode_org_content_indentation(content: &str) -> String {
    let mut encoded_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('*') {
            // Yıldız ile başlayan başlıklara 3 adet '*' ekleyerek seviyeyi düşür (demote)
            encoded_lines.push(format!("{}{}", ORG_HEADER_OFFSET_ASTERISKS, line));
        } else if !line.is_empty() {
            // Düz metinlerin veya listelerin başına 6 boşluk ekle
            encoded_lines.push(format!("{}{}", ORG_INDENT_OFFSET_SPACES, line));
        } else {
            encoded_lines.push(String::new());
        }
    }

    encoded_lines.join("\n")
}

fn decode_org_content_indentation(content: &str) -> String {
    let mut decoded_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("****") {
            // 3 adet '*' ofsetini kaldır (* seviyesine geri çek)
            let line_without_offset = &line[ORG_HEADER_OFFSET_ASTERISKS.len()..];
            decoded_lines.push(line_without_offset.to_string());
        } else if line.starts_with(ORG_INDENT_OFFSET_SPACES) {
            // Başındaki 6 boşluk girintisini kaldır
            let line_without_indent = &line[ORG_INDENT_OFFSET_SPACES.len()..];
            decoded_lines.push(line_without_indent.to_string());
        } else {
            decoded_lines.push(line.to_string());
        }
    }

    decoded_lines.join("\n")
}

fn parse_raw_nodes_from_content(content: &str) -> Vec<RawNode> {
    let mut raw_nodes = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_kv: HashMap<String, String> = HashMap::new();

    let mut current_desc_lines: Vec<String> = Vec::new();
    let mut current_notes_lines: Vec<String> = Vec::new();

    let mut active_block: Option<&str> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // 1. Yeni Ana Düğüm Başlığı (** Node Title)
        if let Some(new_title) = trimmed.strip_prefix(NODE_SECTION_PREFIX) {
            finalize_previous_node(
                &mut raw_nodes,
                &mut current_title,
                &mut current_desc_lines,
                &mut current_notes_lines,
                &mut current_kv,
            );
            current_title = Some(new_title.trim().to_string());
            active_block = None;
            continue;
        }

        // 2. Alt Blok Tanımları (*** description: / *** progress_notes:)
        if trimmed.starts_with(DESC_SECTION_PREFIX) {
            active_block = Some("desc");
            continue;
        }

        if trimmed.starts_with(NOTES_SECTION_PREFIX) {
            active_block = Some("notes");
            continue;
        }

        // 3. Metadata Kontrolü (- key: value)
        // Eğer satır `- key:` formatındaysa (örneğin - prerequisites:, - status:) 
        // ve bir task-box değilse (`- [ ]`), alt blok modundan çıkıp metadata olarak işle:
        if is_node_metadata_line(trimmed) {
            active_block = None;
            if let Some(list_item) = trimmed.strip_prefix(ITEM_PREFIX) {
                parse_key_value_pair(list_item, &mut current_kv);
                continue;
            }
        }

        // 4. Metin veya Task List Satırlarını İlgili Bloğa Ekle
        match active_block {
            Some("desc") => current_desc_lines.push(line.to_string()),
            Some("notes") => current_notes_lines.push(line.to_string()),
            _ => {}
        }
    }

    finalize_previous_node(
        &mut raw_nodes,
        &mut current_title,
        &mut current_desc_lines,
        &mut current_notes_lines,
        &mut current_kv,
    );

    raw_nodes
}

// Metadata satırı olup olmadığını kontrol eden yardımcı fonksiyon:
fn is_node_metadata_line(trimmed_line: &str) -> bool {
    if !trimmed_line.starts_with(ITEM_PREFIX) {
        return false;
    }
    
    // Görev listesi satırlarını (- [ ] veya - [x]) metadata olarak algılamasını engelle:
    if trimmed_line.starts_with("- [ ]") 
        || trimmed_line.starts_with("- [x]") 
        || trimmed_line.starts_with("- [X]") 
    {
        return false;
    }

    // İçinde `:` geçen (- id: "...", - prerequisites: [...]) satırları metadata say:
    trimmed_line.contains(KEY_VALUE_SEPARATOR)
}

fn finalize_previous_node(
    raw_nodes: &mut Vec<RawNode>,
    title_opt: &mut Option<String>,
    desc_lines: &mut Vec<String>,
    notes_lines: &mut Vec<String>,
    kv: &mut HashMap<String, String>,
) {
    if let Some(title) = title_opt.take() {
        let raw_desc_str = desc_lines.join("\n");
        let raw_notes_str = notes_lines.join("\n");

        let description = decode_org_content_indentation(raw_desc_str.trim_matches('\n'));
        let progress_notes = decode_org_content_indentation(raw_notes_str.trim_matches('\n'));

        raw_nodes.push(RawNode {
            title,
            description,
            progress_notes,
            kv: std::mem::take(kv),
        });

        desc_lines.clear();
        notes_lines.clear();
    }
}

fn parse_key_value_pair(list_item: &str, kv: &mut HashMap<String, String>) {
    if let Some((raw_key, raw_val)) = list_item.trim().split_once(KEY_VALUE_SEPARATOR) {
        let key = raw_key.trim().to_string();
        let val = raw_val.trim().to_string();
        kv.insert(key, val);
    }
}

fn process_raw_nodes(raw_nodes: Vec<RawNode>) -> Vec<ProcessedNode> {
    raw_nodes.into_iter().map(build_processed_node).collect()
}

fn build_processed_node(raw: RawNode) -> ProcessedNode {
    let id = extract_node_id(&raw.kv);
    let image_id = extract_uuid_opt(&raw.kv, "image_id");

    let status = extract_node_status(&raw.kv);
    let position = extract_position_coordinates(&raw.kv);
    let prereq_uuids = extract_prerequisites_list(&raw.kv);
    let is_collapsed = extract_bool(&raw.kv, KEY_IS_COLLAPSED);
    let is_frozen = extract_bool(&raw.kv, KEY_IS_FROZEN);

    let mut node = FocusNode::new(raw.title, raw.description);
    node.id = id;
    node.progress_notes = raw.progress_notes;
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

fn format_parent_uuids(graph: &FocusGraph, node_id: Uuid) -> String {
    graph
        .parents_of(node_id)
        .into_iter()
        .map(|parent_id| format!("\"{}\"", parent_id))
        .collect::<Vec<String>>()
        .join(", ")
}
