// tests/smoke.rs — Markdown import/export round-trip doğrulaması.

use std::path::PathBuf;

#[test]
fn markdown_round_trip() {
    let example_path: PathBuf =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("example-tree.md");

    let graph = uslu::markdown::MarkdownIO::import(
        example_path.to_str().expect("path to string"),
    )
    .expect("import başarılı olmalı");

    // 6 düğüm yüklenmeli
    assert_eq!(graph.nodes.len(), 6, "6 düğüm bekleniyor");

    // "Yayınla" düğümünün 2 prereq'i olmalı (Prototip Geliştir + Test Yaz)
    let yayinla = graph
        .nodes
        .iter()
        .find(|n| n.title == "Yayınla")
        .expect("Yayınla düğümü bulunmalı");
    let parents = graph.parents_of(yayinla.id);
    assert_eq!(parents.len(), 2, "Yayınla'nın 2 prereq'i olmalı");

    // Şimdi export edip tekrar import et — veri kaybı olmamalı
    let tmp = std::env::temp_dir().join("uslu_round_trip.md");
    uslu::markdown::MarkdownIO::export(&graph, tmp.to_str().unwrap())
        .expect("export başarılı olmalı");

    let graph2 = uslu::markdown::MarkdownIO::import(tmp.to_str().unwrap())
        .expect("re-import başarılı olmalı");

    assert_eq!(graph.nodes.len(), graph2.nodes.len());
    assert_eq!(graph.edges.len(), graph2.edges.len());

    let _ = std::fs::remove_file(tmp);
}

#[test]
fn sugiyama_layout_runs_without_panic() {
    use uslu::models::{FocusGraph, FocusNode};
    use uslu::sugiyama::SugiyamaEngine;
    use std::collections::HashSet;

    let mut graph = FocusGraph::default();
    let mut n1 = FocusNode::new("A".into(), "".into());
    let mut n2 = FocusNode::new("B".into(), "".into());
    let mut n3 = FocusNode::new("C".into(), "".into());
    graph.add_node(n1.clone());
    graph.add_node(n2.clone());
    graph.add_node(n3.clone());
    graph.add_edge(n1.id, n2.id);
    graph.add_edge(n2.id, n3.id);
    graph.add_edge(n1.id, n3.id); // should be allowed (no cycle)

    let frozen = HashSet::new();
    SugiyamaEngine::layout(&mut graph, &frozen);

    // n1 should be at y=0 (root layer)
    let n1_after = graph.get_node(n1.id).unwrap();
    assert_eq!(n1_after.y, 0.0, "Kök düğüm 0. katmanda olmalı");

    // n3 should be deeper than n1
    let n3_after = graph.get_node(n3.id).unwrap();
    assert!(n3_after.y > n1_after.y, "Child daha derin olmalı");

    // Cycle detection: adding n3 -> n1 should be rejected
    let edges_before = graph.edges.len();
    graph.add_edge(n3.id, n1.id);
    assert_eq!(graph.edges.len(), edges_before, "Döngü oluşturan kenar reddedilmeli");
}
