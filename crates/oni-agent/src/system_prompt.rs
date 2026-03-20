use crate::callbacks;
use crate::preferences::LearnedRule;
use oni_core::personality;
use oni_core::types::ModelTier;

/// Build the base system prompt with personality.
///
/// Personality is condensed to a single-line voice directive to reduce token overhead.
/// Ablation data (2026-03-19): full personality prompt (~200 tokens) cost 15% pass rate.
/// Fix: condense SOUL.md to essential voice traits only, not full emotional state dump.
pub fn build_system_prompt(
    project_dir: Option<&str>,
    tier: ModelTier,
    tools_available: &[&str],
) -> String {
    let tool_section = if tools_available.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nYou have access to these tools: {}. Call them by including a JSON block in your response with the format:\n```json\n{{\"tool\": \"tool_name\", \"args\": {{...}}}}\n```",
            tools_available.join(", ")
        )
    };

    let project_section = match project_dir {
        Some(dir) => {
            let mut section = format!("\n\nYou are working in the project directory: {}", dir);
            if let Some(oni_ctx) = oni_context::retriever::read_oni_context(std::path::Path::new(dir)) {
                section.push_str("\n\n## PROJECT CONTEXT\n");
                section.push_str(&oni_ctx);
            }
            section
        }
        None => String::new(),
    };

    // Load personality — condensed form only (voice + relationship stage).
    // Full emotional state / journal is available via /status, not injected every turn.
    let personality_prompt = personality::build_personality_prompt();

    if personality_prompt.is_empty() {
        format!(
            "You are ONI (Onboard Native Intelligence), a local AI coding assistant running on {}.\n\
            Be concise and direct. Lead with the answer, not the reasoning.\n\
            Use markdown for formatting.{}{}\n\
            When you don't know something, say so clearly.",
            tier.display_name(),
            project_section,
            tool_section
        )
    } else {
        // Condense personality to max 80 tokens — just voice directive + relationship.
        // The full SOUL.md content is trimmed to first paragraph only.
        let condensed = condense_personality(&personality_prompt);
        format!(
            "{}\n\n## OPERATIONAL CONTEXT\nModel tier: {}{}{}\n",
            condensed,
            tier.display_name(),
            project_section,
            tool_section
        )
    }
}

/// Condense personality prompt to essential voice traits.
/// Takes the full build_personality_prompt() output and extracts just:
/// - First paragraph of SOUL.md (the core identity/voice)
/// - Current relationship stage (one line)
/// Drops: full emotional state values, journal entries, detailed modifiers.
fn condense_personality(full_prompt: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    let mut in_first_section = true;
    let mut blank_seen = false;

    for line in full_prompt.lines() {
        if line.trim().is_empty() {
            if in_first_section && !lines.is_empty() {
                blank_seen = true;
            }
            continue;
        }

        // Stop after first blank line (end of first paragraph/section)
        if blank_seen && in_first_section {
            in_first_section = false;
        }

        // Always grab the first section (core identity)
        if in_first_section {
            lines.push(line);
            continue;
        }

        // Cherry-pick relationship line if present
        if line.contains("Relationship:") || line.contains("relationship:") {
            lines.push(line);
        }
    }

    if lines.is_empty() {
        // Fallback: just take first 3 lines
        full_prompt.lines().take(3).collect::<Vec<_>>().join("\n")
    } else {
        lines.join("\n")
    }
}

/// Build system prompt with active learned preference rules injected.
pub fn build_system_prompt_with_rules(
    project_dir: Option<&str>,
    tier: ModelTier,
    tools_available: &[&str],
    rules: &[LearnedRule],
) -> String {
    let base = build_system_prompt(project_dir, tier, tools_available);
    if rules.is_empty() {
        return base;
    }

    let mut section = String::from("\n\n## LEARNED PREFERENCES\n");
    for rule in rules {
        section.push_str(&format!(
            "- {} (confidence: {:.0}%)\n",
            rule.description,
            rule.confidence * 100.0
        ));
    }

    format!("{}{}", base, section)
}

/// Retrieve relevant context from the project index and append it to the system prompt.
/// Returns the original prompt unchanged if the index doesn't exist or retrieval fails.
pub fn build_system_prompt_with_context(
    project_dir: Option<&str>,
    tier: ModelTier,
    tools_available: &[&str],
    user_query: &str,
) -> String {
    build_system_prompt_with_context_opts(project_dir, tier, tools_available, user_query, true, true)
}

/// Context-enriched system prompt with feature flag control.
///
/// KG fix (ablation 2026-03-19): only inject nodes with access_count > 0.
/// Stale/never-accessed nodes were adding noise and costing 19% pass rate.
pub fn build_system_prompt_with_context_opts(
    project_dir: Option<&str>,
    tier: ModelTier,
    tools_available: &[&str],
    user_query: &str,
    enable_kg: bool,
    enable_callbacks: bool,
) -> String {
    let base = build_system_prompt(project_dir, tier, tools_available);

    let Some(dir) = project_dir else {
        return base;
    };

    let db_path = std::path::Path::new(dir).join(".oni").join("index.db");
    if !db_path.exists() {
        return base;
    }

    let conn = match rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Could not open context DB: {}", e);
            return base;
        }
    };

    let chunks = match oni_context::retriever::retrieve(&conn, user_query, Some(4096)) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Context retrieval failed: {}", e);
            return base;
        }
    };

    if chunks.is_empty() {
        return base;
    }

    let mut context_section = String::from("\n\n## CONTEXT\n\nRelevant files from the project index:\n");
    for chunk in &chunks {
        context_section.push_str(&format!("\n### {}\n```\n{}\n```\n", chunk.path, chunk.content));
    }

    let mut result = format!("{}{}", base, context_section);

    // Knowledge graph context — only inject previously-accessed nodes.
    // Ablation finding: stale nodes (access_count == 0) injected noise that hurt 19%.
    if enable_kg {
        inject_kg_context_legacy(&mut result, user_query, dir);
    }

    // Memory callbacks — controlled by feature flag
    if enable_callbacks {
        let oni_db_path = oni_core::config::data_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("oni.db");
        if let Some(callback) = callbacks::find_callback(user_query, &oni_db_path) {
            result.push_str(&format!(
                "\n\n## MEMORY CALLBACK\n{}\nWeave this naturally into your response if relevant — don't force it.",
                callback
            ));
        }
    }

    result
}

/// Legacy KG injection — loads in-memory KG from disk.
fn inject_kg_context_legacy(result: &mut String, user_query: &str, _dir: &str) {
    let mut kg = crate::knowledge_graph::KnowledgeGraph::load();
    let kg_nodes = kg.context_for_query(user_query, 5);
    let relevant_nodes: Vec<_> = kg_nodes.into_iter()
        .filter(|n| n.access_count > 0)
        .collect();
    if !relevant_nodes.is_empty() {
        let mut kg_section =
            String::from("\n\n## REMEMBERED KNOWLEDGE\n\nRelevant facts from past sessions:\n");
        for node in &relevant_nodes {
            kg_section.push_str(&format!("- [{:?}] {}\n", node.node_type, node.content));
        }
        result.push_str(&kg_section);
    }
}

/// Inject KG context using a `KnowledgeStore` backend (Neo4j or in-memory).
pub fn inject_kg_context_from_store(
    result: &mut String,
    user_query: &str,
    project: &str,
    store: &dyn crate::knowledge_graph::KnowledgeStore,
) {
    let nodes = match store.search(user_query, project, 5) {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!("KG store search failed: {}", e);
            return;
        }
    };
    let relevant: Vec<_> = nodes.into_iter().filter(|n| n.access_count > 0).collect();
    if relevant.is_empty() {
        return;
    }
    let mut section =
        String::from("\n\n## REMEMBERED KNOWLEDGE\n\nRelevant facts from past sessions:\n");
    for node in &relevant {
        section.push_str(&format!("- [{}] {}\n", node.node_type, node.content));
    }

    // Cross-project knowledge
    if let Ok(cross) = store.search_cross_project(user_query, 5) {
        let cross_relevant: Vec<_> = cross
            .into_iter()
            .filter(|n| n.access_count > 0 && n.project != project)
            .collect();
        if !cross_relevant.is_empty() {
            section.push_str("\nFrom other projects:\n");
            for node in &cross_relevant {
                section.push_str(&format!("- [{}|{}] {}\n", node.project, node.node_type, node.content));
            }
        }
    }

    result.push_str(&section);
}
