// src/sugiyama.rs
// Tidy-Tree + Direct Sibling Grouping + Multi-parent Centering Layout Engine

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

    /// Aşama 1 — Katmanlama (Topolojik Derinlik)
    fn assign_layers(graph: &FocusGraph) -> HashMap<Uuid, usize> {
        let mut layers: HashMap<Uuid, usize> = HashMap::new();
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

            if !changed {
                break;
            }
        }

        layers
    }

    /// Aşama 2 — Çakışma Azaltma (Barycenter Heuristic)
    fn reduce_crossings(graph: &FocusGraph, layers: &HashMap<Uuid, usize>) -> Vec<Vec<Uuid>> {
        let max_layer = *layers.values().max().unwrap_or(&0);
        let mut grouped: Vec<Vec<Uuid>> = vec![Vec::new(); max_layer + 1];
        for (&id, &layer) in layers {
            if let Some(slot) = grouped.get_mut(layer) {
                slot.push(id);
            }
        }

        for _ in 0..4 {
            for i in 1..grouped.len() {
                let (left, right) = grouped.split_at_mut(i);
                Self::sort_by_barycenter(graph, &mut right[0], &left[i - 1], true);
            }
            for i in (0..grouped.len() - 1).rev() {
                let (left, right) = grouped.split_at_mut(i + 1);
                Self::sort_by_barycenter(graph, &mut left[i], &right[0], false);
            }
        }
        grouped
    }

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
            (sum * 1000) / neighbors.len()
        });
    }

    /// Aşama 3 — Doğrudan Kardeş İzolasyonu ve Merkezleme
    ///
    /// Birden fazla ebeveyni olan (merge) düğümler artık tek bir keyfi
    /// ebeveynin altında "gizli yolcu" gibi taşınmıyor: onlar ve TÜM alt
    /// ağaçları bağımsız bir blok olarak kendi içinde tidy-tree ile
    /// yerleştirilip, sonra bir bütün halinde ebeveynlerinin ortasına
    /// kaydırılıyor. Böylece:
    ///   - Tek-ebeveynli kardeşler (ör. Kaza Namazlari, Vird, ...) artık
    ///     birbirine eşit aralıklı kalır; aralarından biri, aşağıdaki dev
    ///     bir merge alt-ağacını taşımak zorunda kalmaz.
    ///   - Merge düğümünün kendi çocukları da (ör. Ev Bul'un altındaki
    ///     Eksiksiz Rutin / Mahmud Efendi), merge düğümü ortalanırken
    ///     ONUNLA BİRLİKTE kayar — asla ebeveynlerinden kopup sola/sağa
    ///     yaslanmış görünmezler.
    fn assign_coordinates(graph: &mut FocusGraph, ordered: &[Vec<Uuid>], frozen: &HashSet<Uuid>) {
        const X_GAP: f32 = 60.0;        // Doğrudan kardeş düğümler arası boşluk
        const GROUP_GAP: f32 = 180.0;   // Birbiriyle hiç bağlantısı olmayan kökler arası boşluk
        const Y_GAP: f32 = 120.0;

        // 1. Y Koordinatları Ataması
        for (layer_idx, layer_nodes) in ordered.iter().enumerate() {
            let y = layer_idx as f32 * (FocusNode::HEIGHT + Y_GAP);
            for &id in layer_nodes {
                if frozen.contains(&id) {
                    continue;
                }
                if let Some(node) = graph.get_node_mut(id) {
                    node.y = y;
                }
            }
        }

        // Düğümün hangi katmanda olduğunu hızlı bulmak için ters index.
        let layer_of: HashMap<Uuid, usize> = ordered
            .iter()
            .enumerate()
            .flat_map(|(l, nodes)| nodes.iter().map(move |&id| (id, l)))
            .collect();

        // 2. Her düğüm için "bir önceki katmandaki gerçek ebeveynleri" bul.
        //    Tam olarak TEK böyle ebeveyni olan düğümler normal ağaç
        //    çocuğu sayılır (children_of_primary'ye eklenir). İKİ ya da
        //    daha fazla (ya da hiç, nadir bir DAG kenar durumu) ebeveyni
        //    olan düğümler "bağımsız" sayılır: hiçbir ebeveynin
        //    subtree_width'ini şişirmezler, kendi bloklarını kendileri
        //    oluşturup sonradan ortalanırlar (Aşama 6).
        let mut primary_parent: HashMap<Uuid, Uuid> = HashMap::new();
        let mut independent: Vec<Uuid> = Vec::new();

        for layer_idx in 1..ordered.len() {
            for &child in &ordered[layer_idx] {
                let valid_parents: Vec<Uuid> = graph
                    .parents_of(child)
                    .into_iter()
                    .filter(|p| layer_of.get(p) == Some(&(layer_idx - 1)))
                    .collect();

                if valid_parents.len() == 1 {
                    primary_parent.insert(child, valid_parents[0]);
                } else {
                    // 0 (nadir kenar durumu) veya >=2 (gerçek merge) ebeveyn.
                    independent.push(child);
                }
            }
        }

        // 3. Ebeveynlerin Çocuk Listeleri — SADECE tek-ebeveynli (normal)
        //    düğümler ekleniyor; bağımsız/merge düğümler hiçbir listeye
        //    girmiyor, dolayısıyla hiçbir kardeşin genişliğini şişirmiyor.
        let mut children_of_primary: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for layer_nodes in ordered {
            for &id in layer_nodes {
                if let Some(&parent) = primary_parent.get(&id) {
                    children_of_primary.entry(parent).or_default().push(id);
                }
            }
        }

        // 4. Alt Ağaç Genişliği Hesabı (Piksel Cinsinden) — değişmedi.
        fn subtree_width(
            id: Uuid,
            children_of: &HashMap<Uuid, Vec<Uuid>>,
            memo: &mut HashMap<Uuid, f32>,
            x_gap: f32,
        ) -> f32 {
            if let Some(&cached) = memo.get(&id) {
                return cached;
            }

            let width = match children_of.get(&id) {
                Some(children) if !children.is_empty() => {
                    let children_sum: f32 = children
                        .iter()
                        .map(|&c| subtree_width(c, children_of, memo, x_gap))
                        .sum();
                    let gaps = (children.len() - 1) as f32 * x_gap;
                    (children_sum + gaps).max(FocusNode::WIDTH)
                }
                _ => FocusNode::WIDTH,
            };

            memo.insert(id, width);
            width
        }

        // 5. Recursive Yerleştirme — değişmedi. `positions` artık dışarıdan
        //    verilen bir haritaya yazıyor ki hem ana gövde (Aşama A) hem de
        //    her bağımsız merge bloğu (Aşama B) için tekrar kullanılabilsin.
        fn place(
            id: Uuid,
            x_start: f32,
            children_of: &HashMap<Uuid, Vec<Uuid>>,
            memo: &mut HashMap<Uuid, f32>,
            positions: &mut HashMap<Uuid, f32>,
            x_gap: f32,
        ) {
            let children = children_of.get(&id).cloned().unwrap_or_default();

            if children.is_empty() {
                positions.insert(id, x_start);
            } else {
                let mut cursor = x_start;
                let mut first_x = None;
                let mut last_x = 0.0;

                for &child in &children {
                    let child_w = subtree_width(child, children_of, memo, x_gap);
                    let child_x = cursor + (child_w / 2.0) - (FocusNode::WIDTH / 2.0);

                    place(child, cursor, children_of, memo, positions, x_gap);

                    first_x.get_or_insert(child_x);
                    last_x = child_x;

                    cursor += child_w + x_gap;
                }

                let parent_x = (first_x.unwrap() + last_x) / 2.0;
                positions.insert(id, parent_x);
            }
        }

        let mut memo: HashMap<Uuid, f32> = HashMap::new();
        let mut positions: HashMap<Uuid, f32> = HashMap::new();

        // 6a. Ana gövde: 0. katmandaki gerçek kökler, yan yana ve ortalanmış.
        let top_level_roots: Vec<Uuid> = ordered.first().cloned().unwrap_or_default();

        let total_width: f32 = top_level_roots
            .iter()
            .map(|&root| subtree_width(root, &children_of_primary, &mut memo, X_GAP))
            .sum::<f32>()
            + ((top_level_roots.len().saturating_sub(1)) as f32 * GROUP_GAP);

        let mut cursor = -total_width / 2.0;
        for &root in &top_level_roots {
            let root_w = subtree_width(root, &children_of_primary, &mut memo, X_GAP);
            place(root, cursor, &children_of_primary, &mut memo, &mut positions, X_GAP);
            cursor += root_w + GROUP_GAP;
        }

        // 6b. Bağımsız/merge düğümler: katman sırasına göre (üsttekiler
        //     önce) işleniyor ki bir düğümün ebeveynleri işlendiğinde
        //     zaten kesinleşmiş x'e sahip olsunlar. Her biri önce KENDİ
        //     alt ağacıyla birlikte yerel bir başlangıç noktasına (0.0)
        //     göre tidy-tree ile diziliyor, sonra TÜM gerçek
        //     ebeveynlerinin x'lerinin ortasına gelecek şekilde tüm alt
        //     ağaç TEK BİR PARÇA halinde kaydırılıyor — çocuklar asla
        //     ebeveynden kopmuyor.
        let independent_by_layer: Vec<Uuid> = {
            let want: HashSet<Uuid> = independent.into_iter().collect();
            ordered
                .iter()
                .flatten()
                .copied()
                .filter(|id| want.contains(id))
                .collect()
        };

        for id in independent_by_layer {
            let mut local_positions: HashMap<Uuid, f32> = HashMap::new();
            place(id, 0.0, &children_of_primary, &mut memo, &mut local_positions, X_GAP);
            let local_root_x = *local_positions.get(&id).unwrap_or(&0.0);

            let parent_xs: Vec<f32> = graph
                .parents_of(id)
                .iter()
                .filter_map(|p| positions.get(p).copied())
                .collect();

            let desired_x = if parent_xs.is_empty() {
                local_root_x
            } else {
                let min_x = parent_xs.iter().cloned().fold(f32::INFINITY, f32::min);
                let max_x = parent_xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                (min_x + max_x) / 2.0
            };

            let delta = desired_x - local_root_x;

            for (node_id, local_x) in local_positions {
                positions.insert(node_id, local_x + delta);
            }
        }

        // 7. Konumları Uygula
        for (id, x_pos) in &positions {
            if frozen.contains(id) {
                continue;
            }
            if let Some(node) = graph.get_node_mut(*id) {
                node.x = *x_pos;
            }
        }
    }
}