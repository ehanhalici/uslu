// src/models.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeStatus {
    pub progress: f32,
}

impl Default for NodeStatus {
    fn default() -> Self {
        Self { progress: 0.0 }
    }
}

// src/models.rs içine eklenecek/güncellenecek kısımlar:

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusNode {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub progress_notes: String,
    pub image_id: Option<Uuid>,
    pub status: NodeStatus,
    pub x: f32,
    pub y: f32,
    pub is_collapsed: bool, // YENİ: Bu düğümün alt çocukları gizlendi mi?
}

impl FocusNode {
    pub const WIDTH: f32 = 180.0;
    pub const HEIGHT: f32 = 160.0;

    pub fn new(title: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            title,
            description,
            progress_notes: String::new(),
            image_id: None,
            status: NodeStatus::default(),
            x: 0.0,
            y: 0.0,
            is_collapsed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub parent_id: Uuid,
    pub child_id: Uuid,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FocusGraph {
    pub nodes: Vec<FocusNode>,
    pub edges: Vec<Edge>,
}

impl FocusGraph {
    pub fn add_node(&mut self, node: FocusNode) -> Uuid {
        let id = node.id;
        self.nodes.push(node);
        id
    }

    pub fn get_node(&self, id: Uuid) -> Option<&FocusNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn get_node_mut(&mut self, id: Uuid) -> Option<&mut FocusNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn remove_node(&mut self, id: Uuid) {
        self.nodes.retain(|n| n.id != id);
        self.edges.retain(|e| e.parent_id != id && e.child_id != id);
    }

    pub fn remove_edge(&mut self, parent_id: Uuid, child_id: Uuid) {
        self.edges
            .retain(|e| !(e.parent_id == parent_id && e.child_id == child_id));
    }

    pub fn add_edge(&mut self, parent_id: Uuid, child_id: Uuid) {
        if parent_id == child_id {
            return;
        }
        if self
            .edges
            .iter()
            .any(|e| e.parent_id == parent_id && e.child_id == child_id)
        {
            return;
        }
        if self.would_create_cycle(parent_id, child_id) {
            return;
        }
        self.edges.push(Edge {
            parent_id,
            child_id,
        });
    }

    fn would_create_cycle(&self, parent_id: Uuid, child_id: Uuid) -> bool {
        if parent_id == child_id {
            return true;
        }
        let mut stack = vec![child_id];
        let mut visited = std::collections::HashSet::new();
        visited.insert(child_id);

        while let Some(curr) = stack.pop() {
            if curr == parent_id {
                return true;
            }
            for e in &self.edges {
                if e.parent_id == curr && visited.insert(e.child_id) {
                    stack.push(e.child_id);
                }
            }
        }
        false
    }

    pub fn parents_of(&self, id: Uuid) -> Vec<Uuid> {
        self.edges
            .iter()
            .filter(|e| e.child_id == id)
            .map(|e| e.parent_id)
            .collect()
    }

    pub fn children_of(&self, id: Uuid) -> Vec<Uuid> {
        self.edges
            .iter()
            .filter(|e| e.child_id == id)
            .map(|e| e.parent_id)
            .collect()
    }

    pub fn is_node_visible(&self, id: Uuid) -> bool {
        let parents = self.parents_of(id);
        if parents.is_empty() {
            return true;
        }

        // Bir child birden fazla parent'a sahipse:
        // TÜM ebeveynleri 'is_collapsed = true' yapmışsa kapalıdır, en az biri açıksa görünür.
        let all_parents_collapsed = parents
            .iter()
            .filter_map(|&p_id| self.get_node(p_id))
            .all(|p| p.is_collapsed);

        if all_parents_collapsed {
            return false;
        }

        // Üst ebeveynlerin de görünürlüğünü rekürsif kontrol et
        parents.iter().any(|&p_id| self.is_node_visible(p_id))
    }

    /// Düğümlerin topolojik katman/seviye (0-indexed) haritasını döner.
    pub fn get_node_levels(&self) -> std::collections::HashMap<Uuid, usize> {
        let mut levels = std::collections::HashMap::new();
        for _ in 0..self.nodes.len() {
            let mut changed = false;
            for node in &self.nodes {
                let parents = self.parents_of(node.id);
                let new_lvl = if parents.is_empty() {
                    0
                } else {
                    parents
                        .iter()
                        .map(|p| levels.get(p).copied().unwrap_or(0))
                        .max()
                        .unwrap_or(0)
                        + 1
                };
                if levels.get(&node.id).copied() != Some(new_lvl) {
                    levels.insert(node.id, new_lvl);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        levels
    }
}
