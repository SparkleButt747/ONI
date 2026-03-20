//! Neo4j-backed knowledge graph with cross-project search.

use crate::knowledge_graph::{KGNode, KnowledgeStore};
use neo4rs::{query, Graph, ConfigBuilder};
use oni_core::error::Result;
use uuid::Uuid;

/// Neo4j-backed knowledge store.
pub struct Neo4jGraph {
    graph: Graph,
    project: String,
}

impl Neo4jGraph {
    /// Connect to Neo4j and create indexes.
    pub async fn new(uri: &str, project: &str) -> Result<Self> {
        let config = ConfigBuilder::default()
            .uri(uri)
            .build()
            .map_err(|e| oni_core::error::err!("Neo4j config: {}", e))?;
        let graph = Graph::connect(config)
            .await
            .map_err(|e| oni_core::error::err!("Neo4j connect: {}", e))?;

        // Create indexes on first connection
        graph
            .run(query("CREATE INDEX IF NOT EXISTS FOR (n:KGNode) ON (n.id)"))
            .await
            .map_err(|e| oni_core::error::err!("Neo4j index (id): {}", e))?;
        graph
            .run(query("CREATE INDEX IF NOT EXISTS FOR (n:KGNode) ON (n.project)"))
            .await
            .map_err(|e| oni_core::error::err!("Neo4j index (project): {}", e))?;

        // Fulltext index for content search — ignore error if already exists
        let _ = graph
            .run(query(
                "CREATE FULLTEXT INDEX kg_content IF NOT EXISTS FOR (n:KGNode) ON EACH [n.content]",
            ))
            .await;

        Ok(Self {
            graph,
            project: project.to_string(),
        })
    }

    /// Run a connectivity check.
    pub async fn ping(&self) -> Result<()> {
        self.graph
            .run(query("RETURN 1"))
            .await
            .map_err(|e| oni_core::error::err!("Neo4j ping: {}", e))?;
        Ok(())
    }

    /// Helper: extract a KGNode from a neo4rs Row (node alias "n").
    fn row_to_kg_node(row: &neo4rs::Row) -> Option<KGNode> {
        let node: neo4rs::Node = row.get("n").ok()?;
        Some(KGNode {
            id: node.get("id").ok()?,
            node_type: node.get("node_type").ok()?,
            content: node.get("content").ok()?,
            project: node.get("project").ok()?,
            access_count: node.get::<i64>("access_count").ok()? as u32,
            created_at: node.get::<i64>("created_at").ok()? as u64,
            updated_at: node.get::<i64>("updated_at").ok()? as u64,
        })
    }

    /// Helper: run a blocking operation on the tokio runtime.
    /// Since KnowledgeStore methods are sync, we need this bridge.
    fn block_on<F: std::future::Future<Output = T>, T>(fut: F) -> T {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(fut)
        })
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl KnowledgeStore for Neo4jGraph {
    fn add_node(&self, node_type: &str, content: &str, project: &str) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let proj = if project.is_empty() {
            &self.project
        } else {
            project
        };
        let now = now_secs();

        let q = query(
            "CREATE (n:KGNode {id: $id, project: $project, node_type: $node_type, \
             content: $content, access_count: 0, created_at: $now, updated_at: $now})",
        )
        .param("id", id.as_str())
        .param("project", proj)
        .param("node_type", node_type)
        .param("content", content)
        .param("now", now);

        Self::block_on(async {
            self.graph
                .run(q)
                .await
                .map_err(|e| oni_core::error::err!("Neo4j add_node: {}", e))
        })?;

        Ok(id)
    }

    fn add_edge(&self, from: &str, to: &str, relation: &str, weight: f64) -> Result<()> {
        let now = now_secs();
        let q = query(
            "MATCH (a:KGNode {id: $from}), (b:KGNode {id: $to}) \
             CREATE (a)-[:RELATED_TO {relation: $relation, weight: $weight, created_at: $now}]->(b)",
        )
        .param("from", from)
        .param("to", to)
        .param("relation", relation)
        .param("weight", weight)
        .param("now", now);

        Self::block_on(async {
            self.graph
                .run(q)
                .await
                .map_err(|e| oni_core::error::err!("Neo4j add_edge: {}", e))
        })
    }

    fn search(&self, query_str: &str, project: &str, limit: usize) -> Result<Vec<KGNode>> {
        let proj = if project.is_empty() {
            &self.project
        } else {
            project
        };

        // Try fulltext index first; fall back to CONTAINS if fulltext fails.
        let results = Self::block_on(async {
            // Fulltext search
            let ft_q = query(
                "CALL db.index.fulltext.queryNodes('kg_content', $query) YIELD node AS n, score \
                 WHERE n.project = $project \
                 RETURN n ORDER BY score DESC LIMIT $limit",
            )
            .param("query", query_str)
            .param("project", proj)
            .param("limit", limit as i64);

            match self.graph.execute(ft_q).await {
                Ok(mut stream) => {
                    let mut nodes = Vec::new();
                    while let Ok(Some(row)) = stream.next().await {
                        if let Some(node) = Self::row_to_kg_node(&row) {
                            nodes.push(node);
                        }
                    }
                    if !nodes.is_empty() {
                        return Ok::<Vec<KGNode>, oni_core::error::eyre::Report>(nodes);
                    }
                    // Fall through to CONTAINS
                    Ok::<Vec<KGNode>, oni_core::error::eyre::Report>(Vec::new())
                }
                Err(_) => Ok(Vec::new()),
            }
        })?;

        if !results.is_empty() {
            return Ok(results);
        }

        // Fallback: CONTAINS search
        Self::block_on(async {
            let q = query(
                "MATCH (n:KGNode) WHERE n.project = $project AND toLower(n.content) CONTAINS toLower($query) \
                 RETURN n ORDER BY n.access_count DESC LIMIT $limit",
            )
            .param("query", query_str)
            .param("project", proj)
            .param("limit", limit as i64);

            let mut stream = self
                .graph
                .execute(q)
                .await
                .map_err(|e| oni_core::error::err!("Neo4j search: {}", e))?;

            let mut nodes = Vec::new();
            while let Ok(Some(row)) = stream.next().await {
                if let Some(node) = Self::row_to_kg_node(&row) {
                    nodes.push(node);
                }
            }
            Ok(nodes)
        })
    }

    fn search_cross_project(&self, query_str: &str, limit: usize) -> Result<Vec<KGNode>> {
        Self::block_on(async {
            // Same-project results first
            let same_q = query(
                "MATCH (n:KGNode) WHERE n.project = $project AND toLower(n.content) CONTAINS toLower($query) \
                 RETURN n ORDER BY n.access_count DESC LIMIT $limit",
            )
            .param("query", query_str)
            .param("project", self.project.as_str())
            .param("limit", limit as i64);

            let mut nodes = Vec::new();
            let mut stream = self
                .graph
                .execute(same_q)
                .await
                .map_err(|e| oni_core::error::err!("Neo4j cross-project same: {}", e))?;
            while let Ok(Some(row)) = stream.next().await {
                if let Some(node) = Self::row_to_kg_node(&row) {
                    nodes.push(node);
                }
            }

            // Cross-project results (different project, up to 5)
            let cross_q = query(
                "MATCH (n:KGNode) WHERE n.project <> $project AND toLower(n.content) CONTAINS toLower($query) \
                 RETURN n ORDER BY n.access_count DESC LIMIT 5",
            )
            .param("query", query_str)
            .param("project", self.project.as_str());

            let mut stream = self
                .graph
                .execute(cross_q)
                .await
                .map_err(|e| oni_core::error::err!("Neo4j cross-project other: {}", e))?;
            while let Ok(Some(row)) = stream.next().await {
                if let Some(node) = Self::row_to_kg_node(&row) {
                    nodes.push(node);
                }
            }

            nodes.truncate(limit);
            Ok(nodes)
        })
    }

    fn get_related(&self, node_id: &str, depth: usize) -> Result<Vec<(KGNode, String)>> {
        let depth_val = depth.max(1) as i64;
        Self::block_on(async {
            let q = query(
                "MATCH (a:KGNode {id: $id})-[r*1..]->(b:KGNode) \
                 WHERE length(r) <= $depth \
                 UNWIND r AS rel \
                 WITH b, rel \
                 RETURN b AS n, rel.relation AS relation \
                 LIMIT 50",
            )
            .param("id", node_id)
            .param("depth", depth_val);

            let mut stream = self
                .graph
                .execute(q)
                .await
                .map_err(|e| oni_core::error::err!("Neo4j get_related: {}", e))?;

            let mut results = Vec::new();
            while let Ok(Some(row)) = stream.next().await {
                if let Some(node) = Self::row_to_kg_node(&row) {
                    let relation: String = row.get("relation").unwrap_or_default();
                    results.push((node, relation));
                }
            }
            Ok(results)
        })
    }

    fn increment_access(&self, node_id: &str) -> Result<()> {
        let now = now_secs();
        let q = query(
            "MATCH (n:KGNode {id: $id}) \
             SET n.access_count = n.access_count + 1, n.updated_at = $now",
        )
        .param("id", node_id)
        .param("now", now);

        Self::block_on(async {
            self.graph
                .run(q)
                .await
                .map_err(|e| oni_core::error::err!("Neo4j increment_access: {}", e))
        })
    }

    fn gc(&self, max_age_secs: u64) -> Result<usize> {
        let cutoff = now_secs() - max_age_secs as i64;
        Self::block_on(async {
            // Count first
            let count_q = query(
                "MATCH (n:KGNode) WHERE n.updated_at < $cutoff AND n.access_count < 3 \
                 RETURN count(n) AS cnt",
            )
            .param("cutoff", cutoff);

            let mut stream = self
                .graph
                .execute(count_q)
                .await
                .map_err(|e| oni_core::error::err!("Neo4j gc count: {}", e))?;

            let count: i64 = if let Ok(Some(row)) = stream.next().await {
                row.get("cnt").unwrap_or(0)
            } else {
                0
            };

            // Delete nodes and their relationships
            let del_q = query(
                "MATCH (n:KGNode) WHERE n.updated_at < $cutoff AND n.access_count < 3 \
                 DETACH DELETE n",
            )
            .param("cutoff", cutoff);

            self.graph
                .run(del_q)
                .await
                .map_err(|e| oni_core::error::err!("Neo4j gc delete: {}", e))?;

            Ok(count as usize)
        })
    }

    fn stats(&self) -> Result<(usize, usize)> {
        Self::block_on(async {
            let nq = query("MATCH (n:KGNode) RETURN count(n) AS cnt");
            let mut stream = self
                .graph
                .execute(nq)
                .await
                .map_err(|e| oni_core::error::err!("Neo4j stats nodes: {}", e))?;
            let node_count: i64 = if let Ok(Some(row)) = stream.next().await {
                row.get("cnt").unwrap_or(0)
            } else {
                0
            };

            let eq = query("MATCH ()-[r:RELATED_TO]->() RETURN count(r) AS cnt");
            let mut stream = self
                .graph
                .execute(eq)
                .await
                .map_err(|e| oni_core::error::err!("Neo4j stats edges: {}", e))?;
            let edge_count: i64 = if let Ok(Some(row)) = stream.next().await {
                row.get("cnt").unwrap_or(0)
            } else {
                0
            };

            Ok((node_count as usize, edge_count as usize))
        })
    }
}
