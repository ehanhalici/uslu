// src/markdown.rs
use crate::models::{Edge, FocusGraph, FocusNode, NodeStatus};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Result as IoResult, Write};
use uuid::Uuid;

pub struct MarkdownIO;

impl MarkdownIO {
    pub fn export(graph: &FocusGraph, path: &str) -> IoResult<()> {
        let mut file = File::create(path)?;

        writeln!(file, "# Uslu Focus Tree\n")?;

        for node in &graph.nodes {
            writeln!(file, "## {}\n", node.title)?;
            writeln!(file, "- id: \"{}\"", node.id)?;
            writeln!(
                file,
                "- description: \"{}\"",
                escape_quotes(&node.description)
            )?;
            writeln!(
                file,
                "- progress_notes: \"{}\"",
                escape_quotes(&node.progress_notes)
            )?;

            if let Some(img_id) = &node.image_id {
                writeln!(file, "- image_id: \"{}\"", img_id)?;
            }

            writeln!(file, "- status: {:.1}", node.status.progress)?;

            let parents: Vec<String> = graph
                .parents_of(node.id)
                .into_iter()
                .map(|p| format!("\"{}\"", p))
                .collect();
            writeln!(file, "- prerequisites: [{}]", parents.join(", "))?;
            writeln!(file, "- position: [{:.1}, {:.1}]\n", node.x, node.y)?;
        }

        Ok(())
    }

    pub fn import(path: &str) -> Result<FocusGraph, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Dosya okunamadı: {}", e))?;
        Self::parse(&content)
    }

    fn parse(content: &str) -> Result<FocusGraph, String> {
        let mut graph = FocusGraph::default();

        struct RawNode {
            title: String,
            kv: HashMap<String, String>,
        }

        let mut raw_nodes: Vec<RawNode> = Vec::new();
        let mut current_title: Option<String> = None;
        let mut current_kv: HashMap<String, String> = HashMap::new();

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('#') && !trimmed.starts_with("## ") {
                continue;
            }

            if let Some(title) = trimmed.strip_prefix("## ") {
                if let Some(t) = current_title.take() {
                    raw_nodes.push(RawNode {
                        title: t,
                        kv: std::mem::take(&mut current_kv),
                    });
                }
                current_title = Some(title.trim().to_string());
                continue;
            }

            if let Some(list_item) = trimmed.strip_prefix('-') {
                if let Some((k, v)) = list_item.trim().split_once(':') {
                    current_kv.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }

        if let Some(t) = current_title {
            raw_nodes.push(RawNode {
                title: t,
                kv: current_kv,
            });
        }

        struct ProcessedNode {
            node: FocusNode,
            prereq_uuids: Vec<String>,
        }

        let mut processed_nodes: Vec<ProcessedNode> = Vec::new();

        for raw in raw_nodes {
            let id = raw
                .kv
                .get("id")
                .and_then(|s| Uuid::parse_str(s.trim_matches('"')).ok())
                .unwrap_or_else(Uuid::new_v4);
            let description = raw
                .kv
                .get("description")
                .map(|s| unescape_quotes(s.trim_matches('"')))
                .unwrap_or_default();
            let progress_notes = raw
                .kv
                .get("progress_notes")
                .map(|s| unescape_quotes(s.trim_matches('"')))
                .unwrap_or_default();

            let image_id = raw
                .kv
                .get("image_id")
                .and_then(|s| Uuid::parse_str(s.trim_matches('"')).ok());

            let status = raw
                .kv
                .get("status")
                .or_else(|| raw.kv.get("progress"))
                .map(|s| parse_status(s))
                .unwrap_or_default();
            let position = raw
                .kv
                .get("position")
                .and_then(|s| parse_position(s))
                .unwrap_or((0.0, 0.0));
            let prereq_uuids = raw
                .kv
                .get("prerequisites")
                .map(|s| extract_uuid_list(s))
                .unwrap_or_default();

            let mut node = FocusNode::new(raw.title, description);
            node.id = id;
            node.progress_notes = progress_notes;
            node.image_id = image_id;
            node.status = status;
            node.x = position.0;
            node.y = position.1;

            processed_nodes.push(ProcessedNode { node, prereq_uuids });
        }

        for item in &processed_nodes {
            graph.add_node(item.node.clone());
        }

        for item in &processed_nodes {
            for parent_uuid_str in &item.prereq_uuids {
                if let Ok(parent_id) = Uuid::parse_str(parent_uuid_str.trim_matches('"')) {
                    if graph.get_node(parent_id).is_some() {
                        graph.edges.push(Edge {
                            parent_id,
                            child_id: item.node.id,
                        });
                    }
                }
            }
        }

        Ok(graph)
    }
}

fn parse_status(s: &str) -> NodeStatus {
    let s = s.trim_matches('"');
    if let Ok(p) = s.parse::<f32>() {
        return NodeStatus {
            progress: p.clamp(0.0, 100.0),
        };
    }
    NodeStatus { progress: 0.0 }
}

fn parse_position(s: &str) -> Option<(f32, f32)> {
    let s = s.trim_matches(|c| c == '[' || c == ']');
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() == 2 {
        if let (Ok(x), Ok(y)) = (
            parts[0].trim().parse::<f32>(),
            parts[1].trim().parse::<f32>(),
        ) {
            return Some((x, y));
        }
    }
    None
}

fn extract_uuid_list(s: &str) -> Vec<String> {
    let s = s.trim_matches(|c| c == '[' || c == ']');
    s.split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

fn escape_quotes(s: &str) -> String {
    s.replace('"', "\\\"")
}

fn unescape_quotes(s: &str) -> String {
    s.replace("\\\"", "\"")
}
