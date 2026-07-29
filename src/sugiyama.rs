// src/sugiyama.rs
use crate::models::{FocusGraph, FocusNode};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

const BARYCENTER_SCALE_FACTOR: usize = 1000;
const CROSSING_REDUCTION_PASSES: usize = 4;

const SIBLING_X_GAP: f32 = 60.0;
const UNCONNECTED_GROUP_GAP: f32 = 180.0;
const LAYER_Y_GAP: f32 = 120.0;

const DEFAULT_ORIGIN_X: f32 = 0.0;
const CENTER_SPLIT_DIVISOR: f32 = 2.0;

pub struct SugiyamaEngine;

impl SugiyamaEngine {
    pub fn layout(graph: &mut FocusGraph, frozen: &HashSet<Uuid>) {
        if graph.nodes.is_empty() {
            return;
        }

        let layers = Self::assign_layers(graph);
        let ordered_layers = Self::reduce_crossings(graph, &layers);
        Self::assign_coordinates(graph, &ordered_layers, frozen);
    }

    fn assign_layers(graph: &FocusGraph) -> HashMap<Uuid, usize> {
        graph.get_node_levels()
    }

    fn reduce_crossings(graph: &FocusGraph, layers: &HashMap<Uuid, usize>) -> Vec<Vec<Uuid>> {
        let mut grouped_layers = Self::group_nodes_by_layer(layers);

        for _ in 0..CROSSING_REDUCTION_PASSES {
            Self::sweep_downward(graph, &mut grouped_layers);
            Self::sweep_upward(graph, &mut grouped_layers);
        }

        grouped_layers
    }

    fn group_nodes_by_layer(layers: &HashMap<Uuid, usize>) -> Vec<Vec<Uuid>> {
        let max_layer = *layers.values().max().unwrap_or(&0);
        let mut grouped: Vec<Vec<Uuid>> = vec![Vec::new(); max_layer + 1];

        let mut sorted_entries: Vec<_> = layers.iter().collect();
        sorted_entries.sort_by_key(|(id, _)| **id);

        for (&node_id, &layer_idx) in sorted_entries {
            if let Some(layer_slot) = grouped.get_mut(layer_idx) {
                layer_slot.push(node_id);
            }
        }

        grouped
    }

    fn sweep_downward(graph: &FocusGraph, grouped_layers: &mut [Vec<Uuid>]) {
        for i in 1..grouped_layers.len() {
            let (left_slice, right_slice) = grouped_layers.split_at_mut(i);
            let previous_layer = &left_slice[i - 1];
            let current_layer = &mut right_slice[0];

            Self::sort_layer_by_barycenter(graph, current_layer, previous_layer, true);
        }
    }

    fn sweep_upward(graph: &FocusGraph, grouped_layers: &mut [Vec<Uuid>]) {
        for i in (0..grouped_layers.len() - 1).rev() {
            let (left_slice, right_slice) = grouped_layers.split_at_mut(i + 1);
            let current_layer = &mut left_slice[i];
            let next_layer = &right_slice[0];

            Self::sort_layer_by_barycenter(graph, current_layer, next_layer, false);
        }
    }

    fn sort_layer_by_barycenter(
        graph: &FocusGraph,
        target_layer: &mut [Uuid],
        reference_layer: &[Uuid],
        parents_to_children: bool,
    ) {
        let ref_positions = Self::build_position_map(reference_layer);

        target_layer.sort_by_key(|&node_id| {
            let neighbors = if parents_to_children {
                graph.parents_of(node_id)
            } else {
                graph.children_of(node_id)
            };

            Self::calculate_barycenter_weight(&neighbors, &ref_positions)
        });
    }

    fn build_position_map(layer: &[Uuid]) -> HashMap<Uuid, usize> {
        layer
            .iter()
            .enumerate()
            .map(|(index, &id)| (id, index))
            .collect()
    }

    fn calculate_barycenter_weight(
        neighbors: &[Uuid],
        pos_map: &HashMap<Uuid, usize>,
    ) -> usize {
        if neighbors.is_empty() {
            return 0;
        }

        let position_sum: usize = neighbors
            .iter()
            .filter_map(|neighbor_id| pos_map.get(neighbor_id).copied())
            .sum();

        (position_sum * BARYCENTER_SCALE_FACTOR) / neighbors.len()
    }

    fn assign_coordinates(
        graph: &mut FocusGraph,
        ordered_layers: &[Vec<Uuid>],
        frozen: &HashSet<Uuid>,
    ) {
        Self::assign_y_coordinates(graph, ordered_layers, frozen);

        let layer_map = Self::build_node_layer_lookup(ordered_layers);
        let (children_of_primary, independent_nodes) =
            Self::classify_parent_relationships(graph, ordered_layers, &layer_map);

        let mut memoized_widths: HashMap<Uuid, f32> = HashMap::new();
        let mut final_x_positions: HashMap<Uuid, f32> = HashMap::new();

        Self::place_top_level_roots(
            ordered_layers,
            &children_of_primary,
            &mut memoized_widths,
            &mut final_x_positions,
        );

        Self::place_independent_merge_nodes(
            graph,
            ordered_layers,
            &independent_nodes,
            &children_of_primary,
            &mut memoized_widths,
            &mut final_x_positions,
        );

        Self::apply_x_coordinates(graph, &final_x_positions, frozen);
    }

    fn assign_y_coordinates(
        graph: &mut FocusGraph,
        ordered_layers: &[Vec<Uuid>],
        frozen: &HashSet<Uuid>,
    ) {
        for (layer_index, layer_nodes) in ordered_layers.iter().enumerate() {
            let y_coord = layer_index as f32 * (FocusNode::HEIGHT + LAYER_Y_GAP);

            for &node_id in layer_nodes {
                if !frozen.contains(&node_id) {
                    if let Some(node) = graph.get_node_mut(node_id) {
                        node.y = y_coord;
                    }
                }
            }
        }
    }

    fn build_node_layer_lookup(ordered_layers: &[Vec<Uuid>]) -> HashMap<Uuid, usize> {
        ordered_layers
            .iter()
            .enumerate()
            .flat_map(|(layer_idx, nodes)| nodes.iter().map(move |&id| (id, layer_idx)))
            .collect()
    }

    fn classify_parent_relationships(
        graph: &FocusGraph,
        ordered_layers: &[Vec<Uuid>],
        _layer_map: &HashMap<Uuid, usize>,
    ) -> (HashMap<Uuid, Vec<Uuid>>, Vec<Uuid>) {
        let mut primary_parents: HashMap<Uuid, Uuid> = HashMap::new();
        let mut independent_nodes: Vec<Uuid> = Vec::new();

        for layer_idx in 1..ordered_layers.len() {
            for &child_id in &ordered_layers[layer_idx] {
                let all_parents = graph.parents_of(child_id);

                if all_parents.len() == 1 {
                    primary_parents.insert(child_id, all_parents[0]);
                } else if all_parents.len() > 1 {
                    independent_nodes.push(child_id);
                }
            }
        }

        let mut children_of_primary: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for layer_nodes in ordered_layers {
            for &node_id in layer_nodes {
                if let Some(&parent_id) = primary_parents.get(&node_id) {
                    children_of_primary
                        .entry(parent_id)
                        .or_default()
                        .push(node_id);
                }
            }
        }

        (children_of_primary, independent_nodes)
    }
    fn place_top_level_roots(
        ordered_layers: &[Vec<Uuid>],
        children_of_primary: &HashMap<Uuid, Vec<Uuid>>,
        memo: &mut HashMap<Uuid, f32>,
        positions: &mut HashMap<Uuid, f32>,
    ) {
        let top_level_roots: Vec<Uuid> = ordered_layers.first().cloned().unwrap_or_default();
        if top_level_roots.is_empty() {
            return;
        }

        let total_trees_width = Self::calculate_total_roots_width(
            &top_level_roots,
            children_of_primary,
            memo,
        );

        let mut start_cursor = -total_trees_width / CENTER_SPLIT_DIVISOR;
        for &root_id in &top_level_roots {
            let root_width = calculate_subtree_width(root_id, children_of_primary, memo);

            recursive_place_node(
                root_id,
                start_cursor,
                children_of_primary,
                memo,
                positions,
            );

            start_cursor += root_width + UNCONNECTED_GROUP_GAP;
        }
    }

    fn calculate_total_roots_width(
        roots: &[Uuid],
        children_of_primary: &HashMap<Uuid, Vec<Uuid>>,
        memo: &mut HashMap<Uuid, f32>,
    ) -> f32 {
        let width_sum: f32 = roots
            .iter()
            .map(|&root_id| calculate_subtree_width(root_id, children_of_primary, memo))
            .sum();

        let gaps_count = roots.len().saturating_sub(1) as f32;
        width_sum + (gaps_count * UNCONNECTED_GROUP_GAP)
    }

    fn place_independent_merge_nodes(
        graph: &FocusGraph,
        ordered_layers: &[Vec<Uuid>],
        independent_nodes: &[Uuid],
        children_of_primary: &HashMap<Uuid, Vec<Uuid>>,
        memo: &mut HashMap<Uuid, f32>,
        positions: &mut HashMap<Uuid, f32>,
    ) {
        let sorted_independent = Self::sort_independent_by_topological_order(
            ordered_layers,
            independent_nodes,
        );

        for node_id in sorted_independent {
            let mut local_positions: HashMap<Uuid, f32> = HashMap::new();

            recursive_place_node(
                node_id,
                DEFAULT_ORIGIN_X,
                children_of_primary,
                memo,
                &mut local_positions,
            );

            let local_root_x = *local_positions.get(&node_id).unwrap_or(&DEFAULT_ORIGIN_X);
            let desired_center_x = Self::calculate_desired_parent_center(graph, node_id, positions, local_root_x);
            let shift_delta = desired_center_x - local_root_x;

            for (id, local_x) in local_positions {
                positions.entry(id).or_insert(local_x + shift_delta);
            }
        }
    }

    fn sort_independent_by_topological_order(
        ordered_layers: &[Vec<Uuid>],
        independent_nodes: &[Uuid],
    ) -> Vec<Uuid> {
        let target_set: HashSet<Uuid> = independent_nodes.iter().copied().collect();

        ordered_layers
            .iter()
            .flatten()
            .copied()
            .filter(|id| target_set.contains(id))
            .collect()
    }

    fn calculate_desired_parent_center(
        graph: &FocusGraph,
        node_id: Uuid,
        positions: &HashMap<Uuid, f32>,
        fallback_x: f32,
    ) -> f32 {
        let parent_positions: Vec<f32> = graph
            .parents_of(node_id)
            .iter()
            .filter_map(|parent_id| positions.get(parent_id).copied())
            .collect();

        if parent_positions.is_empty() {
            fallback_x
        } else {
            let min_x = parent_positions.iter().cloned().fold(f32::INFINITY, f32::min);
            let max_x = parent_positions.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            (min_x + max_x) / CENTER_SPLIT_DIVISOR
        }
    }

    fn apply_x_coordinates(
        graph: &mut FocusGraph,
        positions: &HashMap<Uuid, f32>,
        frozen: &HashSet<Uuid>,
    ) {
        for (&node_id, &x_coord) in positions {
            if !frozen.contains(&node_id) {
                if let Some(node) = graph.get_node_mut(node_id) {
                    node.x = x_coord;
                }
            }
        }
    }
}

fn calculate_subtree_width(
    node_id: Uuid,
    children_of: &HashMap<Uuid, Vec<Uuid>>,
    memo: &mut HashMap<Uuid, f32>,
) -> f32 {
    if let Some(&cached_width) = memo.get(&node_id) {
        return cached_width;
    }

    let calculated_width = match children_of.get(&node_id) {
        Some(children) if !children.is_empty() => {
            let children_width_sum: f32 = children
                .iter()
                .map(|&child_id| calculate_subtree_width(child_id, children_of, memo))
                .sum();

            let gaps_total = (children.len() - 1) as f32 * SIBLING_X_GAP;
            (children_width_sum + gaps_total).max(FocusNode::WIDTH)
        }
        _ => FocusNode::WIDTH,
    };

    memo.insert(node_id, calculated_width);
    calculated_width
}

fn recursive_place_node(
    node_id: Uuid,
    x_start: f32,
    children_of: &HashMap<Uuid, Vec<Uuid>>,
    memo: &mut HashMap<Uuid, f32>,
    positions: &mut HashMap<Uuid, f32>,
) {
    let children = children_of.get(&node_id).cloned().unwrap_or_default();

    if children.is_empty() {
        positions.insert(node_id, x_start);
        return;
    }

    let (first_child_x, last_child_x) = place_child_nodes(
        &children,
        x_start,
        children_of,
        memo,
        positions,
    );

    let centered_parent_x = (first_child_x + last_child_x) / CENTER_SPLIT_DIVISOR;
    positions.insert(node_id, centered_parent_x);
}

fn place_child_nodes(
    children: &[Uuid],
    start_x: f32,
    children_of: &HashMap<Uuid, Vec<Uuid>>,
    memo: &mut HashMap<Uuid, f32>,
    positions: &mut HashMap<Uuid, f32>,
) -> (f32, f32) {
    let mut cursor = start_x;
    let mut first_child_x = None;
    let mut last_child_x = 0.0;

    for &child_id in children {
        let child_width = calculate_subtree_width(child_id, children_of, memo);
        let child_centered_x = cursor + (child_width / CENTER_SPLIT_DIVISOR) - (FocusNode::WIDTH / CENTER_SPLIT_DIVISOR);

        recursive_place_node(child_id, cursor, children_of, memo, positions);

        if first_child_x.is_none() {
            first_child_x = Some(child_centered_x);
        }
        last_child_x = child_centered_x;

        cursor += child_width + SIBLING_X_GAP;
    }

    (first_child_x.unwrap_or(start_x), last_child_x)
}
