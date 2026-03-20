//! Knowledge Graph — trait-based abstraction over graph backends.
//!
//! Two backends:
//! - `InMemoryKG` — serialized to ~/.local/share/oni/knowledge-graph.json (original)
//! - `Neo4jGraph` — Neo4j-backed, cross-project search (see `neo4j_graph.rs`)

use oni_core::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// ── Shared types ─────────────────────────────────────────────────────────────

/// A node in the knowledge graph, common to all backends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KGNode {
    pub id: String,
    pub node_type: String,
    pub content: String,
    pub project: String,
    pub access_count: u32,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Backend-agnostic knowledge store trait.
///
/// All methods take `&self` (interior mutability) so the store can be shared
/// behind `Arc<dyn KnowledgeStore>`.
pub trait KnowledgeStore: Send + Sync {
    /// Add a node. Returns the node ID.
    fn add_node(&self, node_type: &str, content: &str, project: &str) -> Result<String>;

    /// Add an edge between two nodes.
    fn add_edge(&self, from: &str, to: &str, relation: &str, weight: f64) -> Result<()>;

    /// Search within a single project.
    fn search(&self, query: &str, project: &str, limit: usize) -> Result<Vec<KGNode>>;

    /// Search across all projects.
    fn search_cross_project(&self, query: &str, limit: usize) -> Result<Vec<KGNode>>;

    /// Get related nodes up to `depth` hops. Returns (node, relation_name).
    fn get_related(&self, node_id: &str, depth: usize) -> Result<Vec<(KGNode, String)>>;

    /// Increment access counter for a node.
    fn increment_access(&self, node_id: &str) -> Result<()>;

    /// Garbage collect stale nodes. Returns count of removed nodes.
    fn gc(&self, max_age_secs: u64) -> Result<usize>;

    /// Stats: (node_count, edge_count).
    fn stats(&self) -> Result<(usize, usize)>;
}

// ── Legacy types (kept for serde compat with existing JSON files) ────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    Discovery,
    Fact,
    FileContext,
    Pattern,
    UserPreference,
    Error,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::Fact => "fact",
            Self::FileContext => "file",
            Self::Pattern => "pattern",
            Self::UserPreference => "preference",
            Self::Error => "error",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "discovery" => Self::Discovery,
            "fact" => Self::Fact,
            "file" => Self::FileContext,
            "pattern" => Self::Pattern,
            "preference" => Self::UserPreference,
            "error" => Self::Error,
            _ => Self::Fact,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeNode {
    pub id: String,
    pub node_type: NodeType,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u32,
}

impl KnowledgeNode {
    /// Convert to the backend-agnostic `KGNode`.
    pub fn to_kg_node(&self, project: &str) -> KGNode {
        KGNode {
            id: self.id.clone(),
            node_type: self.node_type.as_str().to_string(),
            content: self.content.clone(),
            project: project.to_string(),
            access_count: self.access_count,
            created_at: self.created_at,
            updated_at: self.last_accessed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEdge {
    pub from: String,
    pub to: String,
    pub relation: EdgeRelation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeRelation {
    RelatedTo,
    CausedBy,
    DependsOn,
    Resolves,
    Contradicts,
    Supersedes,
}

impl EdgeRelation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RelatedTo => "related_to",
            Self::CausedBy => "caused_by",
            Self::DependsOn => "depends_on",
            Self::Resolves => "resolves",
            Self::Contradicts => "contradicts",
            Self::Supersedes => "supersedes",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "caused_by" => Self::CausedBy,
            "depends_on" => Self::DependsOn,
            "resolves" => Self::Resolves,
            "contradicts" => Self::Contradicts,
            "supersedes" => Self::Supersedes,
            _ => Self::RelatedTo,
        }
    }
}

// ── In-memory KG ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub nodes: HashMap<String, KnowledgeNode>,
    pub edges: Vec<KnowledgeEdge>,
    next_id: u64,
    /// Project directory this graph is associated with (not serialized in legacy format).
    #[serde(default)]
    pub project: String,
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            next_id: 1,
            project: String::new(),
        }
    }
}

fn graph_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("oni")
        .join("knowledge-graph.json")
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl KnowledgeGraph {
    /// Load from disk or create empty.
    pub fn load() -> Self {
        let path = graph_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save to disk.
    pub fn save(&self) {
        let path = graph_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ =
            std::fs::write(&path, serde_json::to_string_pretty(self).unwrap_or_default());
    }

    /// Set the project directory for this graph.
    pub fn set_project(&mut self, project: &str) {
        self.project = project.to_string();
    }

    /// Add a new node. Returns the node ID.
    pub fn add_node(&mut self, node_type: NodeType, content: &str) -> String {
        let id = format!("kn_{}", self.next_id);
        self.next_id += 1;
        let now = now_secs();
        self.nodes.insert(
            id.clone(),
            KnowledgeNode {
                id: id.clone(),
                node_type,
                content: content.to_string(),
                metadata: HashMap::new(),
                created_at: now,
                last_accessed: now,
                access_count: 0,
            },
        );
        id
    }

    /// Add a node with metadata.
    pub fn add_node_with_meta(
        &mut self,
        node_type: NodeType,
        content: &str,
        meta: HashMap<String, String>,
    ) -> String {
        let id = self.add_node(node_type, content);
        if let Some(node) = self.nodes.get_mut(&id) {
            node.metadata = meta;
        }
        id
    }

    /// Add an edge between two nodes.
    pub fn add_edge(&mut self, from: &str, to: &str, relation: EdgeRelation) {
        if self.nodes.contains_key(from) && self.nodes.contains_key(to) {
            self.edges.push(KnowledgeEdge {
                from: from.to_string(),
                to: to.to_string(),
                relation,
            });
        }
    }

    /// Query nodes by type.
    pub fn nodes_by_type(&self, node_type: &NodeType) -> Vec<&KnowledgeNode> {
        self.nodes
            .values()
            .filter(|n| &n.node_type == node_type)
            .collect()
    }

    /// Search nodes by content substring (case-insensitive).
    pub fn search(&mut self, query: &str) -> Vec<&KnowledgeNode> {
        let q = query.to_lowercase();
        let matching_ids: HashSet<String> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.content.to_lowercase().contains(&q))
            .map(|(id, _)| id.clone())
            .collect();

        // Update access counts
        for id in &matching_ids {
            if let Some(node) = self.nodes.get_mut(id) {
                node.access_count += 1;
                node.last_accessed = now_secs();
            }
        }

        self.nodes
            .values()
            .filter(|n| matching_ids.contains(&n.id))
            .collect()
    }

    /// Get related nodes (one hop).
    pub fn related(&self, node_id: &str) -> Vec<(&KnowledgeNode, &EdgeRelation)> {
        let mut results = Vec::new();
        for edge in &self.edges {
            if edge.from == node_id {
                if let Some(node) = self.nodes.get(&edge.to) {
                    results.push((node, &edge.relation));
                }
            }
            if edge.to == node_id {
                if let Some(node) = self.nodes.get(&edge.from) {
                    results.push((node, &edge.relation));
                }
            }
        }
        results
    }

    /// Garbage collection — remove nodes not accessed in `max_age_secs`
    /// and with low access counts.
    pub fn gc(&mut self, max_age_secs: u64) {
        let now = now_secs();
        let stale_ids: HashSet<String> = self
            .nodes
            .iter()
            .filter(|(_, n)| {
                let age = now.saturating_sub(n.last_accessed);
                age > max_age_secs && n.access_count < 3
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in &stale_ids {
            self.nodes.remove(id);
        }
        self.edges
            .retain(|e| !stale_ids.contains(&e.from) && !stale_ids.contains(&e.to));
    }

    /// Summary stats.
    pub fn stats(&self) -> (usize, usize) {
        (self.nodes.len(), self.edges.len())
    }

    /// Get the most relevant nodes for a query — top N by recency + access count.
    pub fn context_for_query(&mut self, query: &str, max_nodes: usize) -> Vec<&KnowledgeNode> {
        let mut results = self.search(query);
        let now = now_secs();
        results.sort_by(|a, b| {
            let score_a =
                a.access_count as f64 / (1.0 + (now - a.last_accessed) as f64 / 86400.0);
            let score_b =
                b.access_count as f64 / (1.0 + (now - b.last_accessed) as f64 / 86400.0);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(max_nodes);
        results
    }
}

// ── KnowledgeStore impl for in-memory KG ─────────────────────────────────────

use std::sync::Mutex;

/// Thread-safe wrapper around the in-memory KG that implements `KnowledgeStore`.
pub struct InMemoryKnowledgeStore {
    inner: Mutex<KnowledgeGraph>,
}

impl InMemoryKnowledgeStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(KnowledgeGraph::default()),
        }
    }

    pub fn load_or_default() -> Self {
        Self {
            inner: Mutex::new(KnowledgeGraph::load()),
        }
    }

    pub fn with_project(project: &str) -> Self {
        let mut kg = KnowledgeGraph::load();
        kg.set_project(project);
        Self {
            inner: Mutex::new(kg),
        }
    }

    /// Save the inner graph to disk.
    pub fn save(&self) {
        if let Ok(kg) = self.inner.lock() {
            kg.save();
        }
    }

    /// Get mutable access to the inner graph (for legacy callers).
    pub fn with_inner<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut KnowledgeGraph) -> R,
    {
        let mut kg = self.inner.lock().expect("KG lock poisoned");
        f(&mut kg)
    }
}

impl KnowledgeStore for InMemoryKnowledgeStore {
    fn add_node(&self, node_type: &str, content: &str, _project: &str) -> Result<String> {
        let mut kg = self.inner.lock().map_err(|e| oni_core::error::err!("KG lock: {}", e))?;
        let nt = NodeType::from_str_lossy(node_type);
        let id = kg.add_node(nt, content);
        Ok(id)
    }

    fn add_edge(&self, from: &str, to: &str, relation: &str, _weight: f64) -> Result<()> {
        let mut kg = self.inner.lock().map_err(|e| oni_core::error::err!("KG lock: {}", e))?;
        let rel = EdgeRelation::from_str_lossy(relation);
        kg.add_edge(from, to, rel);
        Ok(())
    }

    fn search(&self, query: &str, _project: &str, limit: usize) -> Result<Vec<KGNode>> {
        let mut kg = self.inner.lock().map_err(|e| oni_core::error::err!("KG lock: {}", e))?;
        let project = kg.project.clone();
        let results = kg.search(query);
        let mut nodes: Vec<KGNode> = results
            .into_iter()
            .map(|n| n.to_kg_node(&project))
            .collect();
        nodes.truncate(limit);
        Ok(nodes)
    }

    fn search_cross_project(&self, query: &str, limit: usize) -> Result<Vec<KGNode>> {
        // In-memory KG is single-project; just delegate to search.
        self.search(query, "", limit)
    }

    fn get_related(&self, node_id: &str, _depth: usize) -> Result<Vec<(KGNode, String)>> {
        let kg = self.inner.lock().map_err(|e| oni_core::error::err!("KG lock: {}", e))?;
        let project = kg.project.clone();
        let results = kg.related(node_id);
        Ok(results
            .into_iter()
            .map(|(n, rel)| (n.to_kg_node(&project), rel.as_str().to_string()))
            .collect())
    }

    fn increment_access(&self, node_id: &str) -> Result<()> {
        let mut kg = self.inner.lock().map_err(|e| oni_core::error::err!("KG lock: {}", e))?;
        if let Some(node) = kg.nodes.get_mut(node_id) {
            node.access_count += 1;
            node.last_accessed = now_secs();
        }
        Ok(())
    }

    fn gc(&self, max_age_secs: u64) -> Result<usize> {
        let mut kg = self.inner.lock().map_err(|e| oni_core::error::err!("KG lock: {}", e))?;
        let before = kg.nodes.len();
        kg.gc(max_age_secs);
        Ok(before - kg.nodes.len())
    }

    fn stats(&self) -> Result<(usize, usize)> {
        let kg = self.inner.lock().map_err(|e| oni_core::error::err!("KG lock: {}", e))?;
        Ok(kg.stats())
    }
}
