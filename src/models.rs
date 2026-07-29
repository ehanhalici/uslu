// src/models.rs
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

// =============================================================================
// Sabitler (Constants)
// =============================================================================
const DEFAULT_PROGRESS_VALUE: f32 = 0.0;
const INITIAL_COORDINATE_X: f32 = 0.0;
const INITIAL_COORDINATE_Y: f32 = 0.0;
const INITIAL_COLLAPSED_STATE: bool = false;
const ROOT_LEVEL_INDEX: usize = 0;
const LEVEL_INCREMENT_STEP: usize = 1;

// =============================================================================
// Düğüm Durumu (NodeStatus)
// =============================================================================
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeStatus {
    pub progress: f32,
}

impl Default for NodeStatus {
    fn default() -> Self {
        Self {
            progress: DEFAULT_PROGRESS_VALUE,
        }
    }
}

// =============================================================================
// Odak Düğümü (FocusNode)
// =============================================================================
// FocusNode tanımında alan ekleme:
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
    pub is_collapsed: bool,
    pub is_frozen: bool,
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
            x: INITIAL_COORDINATE_X,
            y: INITIAL_COORDINATE_Y,
            is_collapsed: INITIAL_COLLAPSED_STATE,
            is_frozen: false,
        }
    }
}
// =============================================================================
// Bağlantı Kenarı (Edge)
// =============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub parent_id: Uuid,
    pub child_id: Uuid,
}

impl Edge {
    pub fn new(parent_id: Uuid, child_id: Uuid) -> Self {
        Self {
            parent_id,
            child_id,
        }
    }

    pub fn matches(&self, parent_id: Uuid, child_id: Uuid) -> bool {
        self.parent_id == parent_id && self.child_id == child_id
    }

    pub fn involves_node(&self, node_id: Uuid) -> bool {
        self.parent_id == node_id || self.child_id == node_id
    }
}

// =============================================================================
// Odak Grafiği (FocusGraph)
// =============================================================================
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FocusGraph {
    pub nodes: Vec<FocusNode>,
    pub edges: Vec<Edge>,
}

impl FocusGraph {
    // -------------------------------------------------------------------------
    // Düğüm ve Kenar Temel Operasyonları
    // -------------------------------------------------------------------------
    pub fn add_node(&mut self, node: FocusNode) -> Uuid {
        let node_id = node.id;
        if self.get_node(node_id).is_none() {
            self.nodes.push(node);
        }
        node_id
    }

    pub fn get_node(&self, id: Uuid) -> Option<&FocusNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn get_node_mut(&mut self, id: Uuid) -> Option<&mut FocusNode> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    pub fn remove_node(&mut self, id: Uuid) {
        self.nodes.retain(|node| node.id != id);
        self.edges.retain(|edge| !edge.involves_node(id));
    }

    pub fn remove_edge(&mut self, parent_id: Uuid, child_id: Uuid) {
        self.edges
            .retain(|edge| !edge.matches(parent_id, child_id));
    }

    pub fn add_edge(&mut self, parent_id: Uuid, child_id: Uuid) {
        if self.get_node(parent_id).is_none() || self.get_node(child_id).is_none() {
            return;
        }

        if self.is_invalid_edge_connection(parent_id, child_id) {
            return;
        }

        self.edges.push(Edge::new(parent_id, child_id));
    }

    // -------------------------------------------------------------------------
    // İlişki Sorguları
    // -------------------------------------------------------------------------
    pub fn parents_of(&self, id: Uuid) -> Vec<Uuid> {
        self.edges
            .iter()
            .filter(|edge| edge.child_id == id)
            .map(|edge| edge.parent_id)
            .collect()
    }

    pub fn children_of(&self, id: Uuid) -> Vec<Uuid> {
        self.edges
            .iter()
            .filter(|edge| edge.parent_id == id)
            .map(|edge| edge.child_id)
            .collect()
    }

    // -------------------------------------------------------------------------
    // Görünürlük Mantığı (DFS + Visited Set Korumalı)
    // -------------------------------------------------------------------------
    pub fn is_node_visible(&self, id: Uuid) -> bool {
        let mut visited = HashSet::new();
        self.check_visibility_dfs(id, &mut visited)
    }

    fn check_visibility_dfs(&self, id: Uuid, visited: &mut HashSet<Uuid>) -> bool {
        if !visited.insert(id) {
            return false;
        }

        let parent_ids = self.parents_of(id);
        if parent_ids.is_empty() {
            return true;
        }

        if self.are_all_parents_collapsed(&parent_ids) {
            return false;
        }

        parent_ids.iter().any(|&p_id| {
            if let Some(parent_node) = self.get_node(p_id) {
                if parent_node.is_collapsed {
                    return false;
                }
            }
            self.check_visibility_dfs(p_id, visited)
        })
    }

    // -------------------------------------------------------------------------
    // Topolojik Katmanlama
    // -------------------------------------------------------------------------
    pub fn get_node_levels(&self) -> HashMap<Uuid, usize> {
        let mut node_levels = HashMap::new();
        let max_iterations = self.nodes.len();

        for _ in 0..max_iterations {
            let state_changed = self.calculate_and_update_node_levels(&mut node_levels);
            if !state_changed {
                break;
            }
        }

        node_levels
    }
}

// =============================================================================
// Özel Yardımcı Metotlar (Private Helpers)
// =============================================================================
impl FocusGraph {
    fn is_invalid_edge_connection(&self, parent_id: Uuid, child_id: Uuid) -> bool {
        if parent_id == child_id {
            return true;
        }

        if self.edge_exists(parent_id, child_id) {
            return true;
        }

        self.would_create_cycle(parent_id, child_id)
    }

    fn edge_exists(&self, parent_id: Uuid, child_id: Uuid) -> bool {
        self.edges
            .iter()
            .any(|edge| edge.matches(parent_id, child_id))
    }

    fn would_create_cycle(&self, parent_id: Uuid, child_id: Uuid) -> bool {
        if parent_id == child_id {
            return true;
        }

        let mut traversal_stack = vec![child_id];
        let mut visited_nodes = HashSet::new();
        visited_nodes.insert(child_id);

        while let Some(current_node_id) = traversal_stack.pop() {
            if current_node_id == parent_id {
                return true;
            }

            self.push_unvisited_children(
                current_node_id,
                &mut traversal_stack,
                &mut visited_nodes,
            );
        }

        false
    }

    fn push_unvisited_children(
        &self,
        current_node_id: Uuid,
        traversal_stack: &mut Vec<Uuid>,
        visited_nodes: &mut HashSet<Uuid>,
    ) {
        for edge in &self.edges {
            if edge.parent_id == current_node_id && visited_nodes.insert(edge.child_id) {
                traversal_stack.push(edge.child_id);
            }
        }
    }

    fn are_all_parents_collapsed(&self, parent_ids: &[Uuid]) -> bool {
        parent_ids
            .iter()
            .filter_map(|&p_id| self.get_node(p_id))
            .all(|parent_node| parent_node.is_collapsed)
    }

    fn calculate_and_update_node_levels(&self, node_levels: &mut HashMap<Uuid, usize>) -> bool {
        let mut state_changed = false;

        for node in &self.nodes {
            let calculated_level = self.compute_level_for_node(node.id, node_levels);
            let current_level = node_levels.get(&node.id).copied();

            if current_level != Some(calculated_level) {
                node_levels.insert(node.id, calculated_level);
                state_changed = true;
            }
        }

        state_changed
    }

    fn compute_level_for_node(&self, node_id: Uuid, node_levels: &HashMap<Uuid, usize>) -> usize {
        let parent_ids = self.parents_of(node_id);
        if parent_ids.is_empty() {
            return ROOT_LEVEL_INDEX;
        }

        let max_parent_level = parent_ids
            .iter()
            .map(|parent_id| node_levels.get(parent_id).copied().unwrap_or(ROOT_LEVEL_INDEX))
            .max()
            .unwrap_or(ROOT_LEVEL_INDEX);

        max_parent_level + LEVEL_INCREMENT_STEP
    }
}
