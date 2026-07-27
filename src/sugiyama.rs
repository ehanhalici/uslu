// src/sugiyama.rs
// Sugiyama tarzı 4 aşamalı DAG layout motoru.
//
// Aşamalar:
//   1. Layer assignment (topolojik derinlik)
//   2. Crossing reduction (barycenter heuristic)
//   3. Coordinate assignment (yatayda ortala, dikeyde katman sırasına göre)
//   4. Kenar routing — canvas tarafında orthogonal olarak çizilir (max 2 kırılım).
//
// `frozen` setindeki düğümler elle sürüklendiği için yerleri korunur.

use crate::models::{FocusGraph, FocusNode};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub struct SugiyamaEngine;

impl SugiyamaEngine {
    pub fn layout(graph: &mut FocusGraph, frozen: &HashSet<Uuid>) {
        if graph.nodes.is_empty() {
            return;
        }

        let layers = Self::assign_layers(graph);
        let ordered = Self::reduce_crossings(graph, &layers);
        Self::assign_coordinates(graph, &ordered, frozen);
    }

    /// Aşama 1 — her düğümün katmanı = (en derin parent katmanı) + 1.
    /// Root'lar 0. katmanda. Sabit noktaya ulaşana kadar iteratif hesaplanır.
    fn assign_layers(graph: &FocusGraph) -> HashMap<Uuid, usize> {
        let mut layers: HashMap<Uuid, usize> = HashMap::new();

        // Graf üzerindeki düğüm sayısı kadar maksimum iterasyon sınırı koyuyoruz.
        // Bir DAG'da en uzun yol düğüm sayısından fazla olamaz.
        let max_iterations = graph.nodes.len();

        for _ in 0..max_iterations {
            let mut changed = false;

            for node in &graph.nodes {
                let parents = graph.parents_of(node.id);
                let new_layer = if parents.is_empty() {
                    0
                } else {
                    parents
                        .iter()
                        .map(|p| layers.get(p).copied().unwrap_or(0))
                        .max()
                        .unwrap_or(0)
                        + 1
                };

                if layers.get(&node.id).copied() != Some(new_layer) {
                    layers.insert(node.id, new_layer);
                    changed = true;
                }
            }

            // Eğer bu turda hiçbir düğümün katmanı değişmediyse fixed-point'e ulaştık demektir.
            if !changed {
                break;
            }
        }

        layers
    }
    /// Aşama 2 — her katman içinde düğümleri barycenter heuristic'ine göre sırala.
    /// Birkaç iterasyon down-sweep + up-sweep yaparak kesişim sayısını azaltır.
    fn reduce_crossings(graph: &FocusGraph, layers: &HashMap<Uuid, usize>) -> Vec<Vec<Uuid>> {
        let max_layer = *layers.values().max().unwrap_or(&0);
        let mut grouped: Vec<Vec<Uuid>> = vec![Vec::new(); max_layer + 1];
        for (&id, &layer) in layers {
            if let Some(slot) = grouped.get_mut(layer) {
                slot.push(id);
            }
        }

        for _ in 0..4 {
            // Aşağı süpürme — her düğümü parent'larının ortalama pozisyonuna göre sırala
            for i in 1..grouped.len() {
                // split_at_mut: aynı Vec'in farklı kısımlarını aynı anda ödünç al
                let (left, right) = grouped.split_at_mut(i);
                Self::sort_by_barycenter(graph, &mut right[0], &left[i - 1], true);
            }
            // Yukarı süpürme — children'ların ortalama pozisyonuna göre
            for i in (0..grouped.len() - 1).rev() {
                let (left, right) = grouped.split_at_mut(i + 1);
                Self::sort_by_barycenter(graph, &mut left[i], &right[0], false);
            }
        }
        grouped
    }

    /// Verilen katmanı, komşu katmandaki pozisyonların ortalamasına göre sıralar.
    /// `parents_to_children = true` → reference katman parent'ları içerir.
    fn sort_by_barycenter(
        graph: &FocusGraph,
        layer: &mut Vec<Uuid>,
        reference: &[Uuid],
        parents_to_children: bool,
    ) {
        let pos_in_ref: HashMap<Uuid, usize> = reference
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, i))
            .collect();

        layer.sort_by_key(|id| {
            let neighbors: Vec<Uuid> = if parents_to_children {
                graph.parents_of(*id)
            } else {
                graph.children_of(*id)
            };
            if neighbors.is_empty() {
                return 0;
            }
            let sum: usize = neighbors
                .iter()
                .filter_map(|n| pos_in_ref.get(n).copied())
                .sum();
            // 1000 ile çarpıp dereceye bölme yuvarlama hatasını azaltır
            (sum * 1000) / neighbors.len()
        });
    }

    /// Aşama 3 — her katmanı yatayda ortalayarak yerleştir.
    /// `frozen` içindeki düğümlerin konumu korunur (elle sürükleniyor).
    fn assign_coordinates(graph: &mut FocusGraph, ordered: &[Vec<Uuid>], frozen: &HashSet<Uuid>) {
        const X_GAP: f32 = 60.0;
        const Y_GAP: f32 = 110.0;

        for (layer_idx, layer_nodes) in ordered.iter().enumerate() {
            let y = layer_idx as f32 * (FocusNode::HEIGHT + Y_GAP);
            let total_w = layer_nodes.len() as f32 * (FocusNode::WIDTH + X_GAP) - X_GAP;
            let start_x = -total_w / 2.0;

            for (i, &id) in layer_nodes.iter().enumerate() {
                if frozen.contains(&id) {
                    continue;
                }
                if let Some(node) = graph.get_node_mut(id) {
                    node.x = start_x + i as f32 * (FocusNode::WIDTH + X_GAP);
                    node.y = y;
                }
            }
        }
    }
}
