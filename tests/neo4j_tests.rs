//! Tests for KnowledgeStore trait — in-memory and Neo4j backends.

use oni_agent::knowledge_graph::{
    InMemoryKnowledgeStore, KnowledgeStore,
};

// ── T-KGS: KnowledgeStore trait with in-memory backend ───────────────────────

#[test]
/// T-KGS-1: InMemoryKnowledgeStore starts empty.
fn t_kgs_1_in_memory_starts_empty() {
    let store = InMemoryKnowledgeStore::new();
    let (nodes, edges) = store.stats().unwrap();
    assert_eq!(nodes, 0);
    assert_eq!(edges, 0);
}

#[test]
/// T-KGS-2: add_node returns a non-empty ID and increments node count.
fn t_kgs_2_add_node_returns_id() {
    let store = InMemoryKnowledgeStore::new();
    let id = store.add_node("fact", "the sky is blue", "/tmp/project").unwrap();
    assert!(!id.is_empty());
    let (nodes, _) = store.stats().unwrap();
    assert_eq!(nodes, 1);
}

#[test]
/// T-KGS-3: Sequential add_node calls return distinct IDs.
fn t_kgs_3_add_node_ids_unique() {
    let store = InMemoryKnowledgeStore::new();
    let id1 = store.add_node("fact", "one", "/tmp").unwrap();
    let id2 = store.add_node("discovery", "two", "/tmp").unwrap();
    assert_ne!(id1, id2);
}

#[test]
/// T-KGS-4: search finds nodes by content substring.
fn t_kgs_4_search_finds_matching_nodes() {
    let store = InMemoryKnowledgeStore::new();
    store.add_node("fact", "Rust ownership rules", "/tmp").unwrap();
    store.add_node("fact", "Python typing", "/tmp").unwrap();
    let results = store.search("ownership", "/tmp", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].content.contains("ownership"));
}

#[test]
/// T-KGS-5: search returns empty when no match.
fn t_kgs_5_search_no_match_empty() {
    let store = InMemoryKnowledgeStore::new();
    store.add_node("fact", "unrelated content", "/tmp").unwrap();
    let results = store.search("nonexistent query xyz", "/tmp", 10).unwrap();
    assert!(results.is_empty());
}

#[test]
/// T-KGS-6: add_edge and stats reflect the edge.
fn t_kgs_6_add_edge_reflected_in_stats() {
    let store = InMemoryKnowledgeStore::new();
    let a = store.add_node("error", "segfault", "/tmp").unwrap();
    let b = store.add_node("fact", "null check missing", "/tmp").unwrap();
    store.add_edge(&a, &b, "caused_by", 1.0).unwrap();
    let (_, edges) = store.stats().unwrap();
    assert_eq!(edges, 1);
}

#[test]
/// T-KGS-7: get_related returns connected nodes.
fn t_kgs_7_get_related_returns_connected() {
    let store = InMemoryKnowledgeStore::new();
    let a = store.add_node("error", "crash", "/tmp").unwrap();
    let b = store.add_node("fact", "fix applied", "/tmp").unwrap();
    store.add_edge(&a, &b, "resolves", 0.9).unwrap();
    let related = store.get_related(&a, 1).unwrap();
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].0.id, b);
    assert_eq!(related[0].1, "resolves");
}

#[test]
/// T-KGS-8: increment_access bumps the counter.
fn t_kgs_8_increment_access_bumps_counter() {
    let store = InMemoryKnowledgeStore::new();
    let id = store.add_node("fact", "test node", "/tmp").unwrap();
    store.increment_access(&id).unwrap();
    store.increment_access(&id).unwrap();
    // Search to verify — search also bumps, so total will be 3
    let results = store.search("test node", "/tmp", 10).unwrap();
    assert!(results[0].access_count >= 2);
}

#[test]
/// T-KGS-9: gc removes old low-access nodes (via serialization backdating).
fn t_kgs_9_gc_removes_stale_nodes() {
    let store = InMemoryKnowledgeStore::new();

    // Use the inner graph to backdate nodes
    store.with_inner(|kg| {
        use oni_agent::knowledge_graph::NodeType;
        let id = kg.add_node(NodeType::Fact, "stale node");
        // Backdate last_accessed by 100 seconds
        if let Some(node) = kg.nodes.get_mut(&id) {
            node.last_accessed = node.last_accessed.saturating_sub(100);
        }
    });

    let (before, _) = store.stats().unwrap();
    assert_eq!(before, 1);

    let removed = store.gc(1).unwrap(); // max_age = 1 second
    assert_eq!(removed, 1);
    let (after, _) = store.stats().unwrap();
    assert_eq!(after, 0);
}

#[test]
/// T-KGS-10: search_cross_project delegates to search for in-memory backend.
fn t_kgs_10_cross_project_delegates() {
    let store = InMemoryKnowledgeStore::new();
    store.add_node("fact", "cross project test", "/tmp").unwrap();
    let results = store.search_cross_project("cross project", 10).unwrap();
    assert_eq!(results.len(), 1);
}

// ── T-N4J: Neo4j backend tests (skipped if Neo4j not running) ────────────────

use oni_agent::neo4j_graph::Neo4jGraph;

/// Try to connect to local Neo4j. Returns None if not available.
fn try_neo4j() -> Option<Neo4jGraph> {
    let rt = tokio::runtime::Runtime::new().ok()?;
    rt.block_on(async {
        Neo4jGraph::new("bolt://localhost:7687", "/test/project-a").await.ok()
    })
}

#[test]
/// T-N4J-1: Neo4j add_node + search round-trip.
fn t_n4j_1_add_and_search() {
    let Some(graph) = try_neo4j() else {
        eprintln!("SKIP: Neo4j not available");
        return;
    };

    // Use a unique content string to avoid collisions with other tests
    let unique = format!("t_n4j_1_test_{}", uuid::Uuid::new_v4());
    let id = graph.add_node("fact", &unique, "/test/project-a").unwrap();
    assert!(!id.is_empty());

    let results = graph.search(&unique, "/test/project-a", 10).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].id, id);
    assert_eq!(results[0].content, unique);

    // Clean up
    let _ = graph.gc(0);
}

#[test]
/// T-N4J-2: Neo4j cross-project search returns nodes from other projects.
fn t_n4j_2_cross_project_search() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let graph_a = rt.block_on(async {
        Neo4jGraph::new("bolt://localhost:7687", "/test/project-a").await
    });
    let Ok(graph_a) = graph_a else {
        eprintln!("SKIP: Neo4j not available");
        return;
    };

    let unique = format!("crossproject_{}", uuid::Uuid::new_v4());

    // Add node in project-b using project-a's connection
    graph_a.add_node("pattern", &unique, "/test/project-b").unwrap();

    // Search from project-a — cross_project should find the node from project-b
    let results = graph_a.search_cross_project(&unique, 10).unwrap();
    assert!(!results.is_empty());
    let has_project_b = results.iter().any(|n| n.project == "/test/project-b");
    assert!(has_project_b, "Expected to find node from project-b in cross-project search");

    // Clean up
    let _ = graph_a.gc(0);
}

#[test]
/// T-N4J-3: Neo4j increment_access updates the counter.
fn t_n4j_3_increment_access() {
    let Some(graph) = try_neo4j() else {
        eprintln!("SKIP: Neo4j not available");
        return;
    };

    let unique = format!("t_n4j_3_test_{}", uuid::Uuid::new_v4());
    let id = graph.add_node("fact", &unique, "/test/project-a").unwrap();

    graph.increment_access(&id).unwrap();
    graph.increment_access(&id).unwrap();

    let results = graph.search(&unique, "/test/project-a", 10).unwrap();
    assert!(!results.is_empty());
    assert!(results[0].access_count >= 2, "Expected access_count >= 2, got {}", results[0].access_count);

    let _ = graph.gc(0);
}

#[test]
/// T-N4J-4: Neo4j add_edge + stats.
fn t_n4j_4_add_edge_stats() {
    let Some(graph) = try_neo4j() else {
        eprintln!("SKIP: Neo4j not available");
        return;
    };

    let u1 = format!("t_n4j_4a_{}", uuid::Uuid::new_v4());
    let u2 = format!("t_n4j_4b_{}", uuid::Uuid::new_v4());

    let a = graph.add_node("error", &u1, "/test/project-a").unwrap();
    let b = graph.add_node("fact", &u2, "/test/project-a").unwrap();
    graph.add_edge(&a, &b, "caused_by", 0.8).unwrap();

    let (nodes, edges) = graph.stats().unwrap();
    assert!(nodes >= 2, "Expected at least 2 nodes, got {}", nodes);
    assert!(edges >= 1, "Expected at least 1 edge, got {}", edges);

    let _ = graph.gc(0);
}

#[test]
/// T-N4J-5: Neo4j gc removes stale nodes.
fn t_n4j_5_gc_removes_stale() {
    let Some(graph) = try_neo4j() else {
        eprintln!("SKIP: Neo4j not available");
        return;
    };

    let unique = format!("t_n4j_5_test_{}", uuid::Uuid::new_v4());
    graph.add_node("discovery", &unique, "/test/project-a").unwrap();

    // GC with max_age=0 should remove nodes created just now (updated_at <= now)
    let removed = graph.gc(0).unwrap();
    assert!(removed >= 1, "Expected gc to remove at least 1 node, got {}", removed);

    // Verify it's gone
    let results = graph.search(&unique, "/test/project-a", 10).unwrap();
    assert!(results.is_empty(), "Expected node to be gc'd");
}
