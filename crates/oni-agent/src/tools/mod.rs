pub mod ask_user;
pub mod bash;
pub mod edit_file;
pub mod forge_tool;
pub mod get_url;
pub mod list_dir;
pub mod read_file;
pub mod search_files;
pub mod undo;
pub mod write_file;

use oni_core::error::Result;
use oni_core::types::ToolCapability;
use std::collections::{HashMap, HashSet};

pub use ask_user::AskUserChannel;
pub use undo::UndoHistory;

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> serde_json::Value;
    fn execute(&self, args: serde_json::Value) -> Result<String>;

    /// Declare which capabilities this tool requires.
    /// Default impl returns an empty slice (no capabilities needed — safe / read-only).
    fn required_capabilities(&self) -> &[ToolCapability] {
        &[]
    }
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
    _allow_write: bool,
    _allow_exec: bool,
    /// Granted capabilities for this registry instance.
    capabilities: HashSet<ToolCapability>,
    /// Shared undo history — snapshotted before every write/edit operation.
    pub undo_history: UndoHistory,
}

impl ToolRegistry {
    pub fn new(allow_write: bool, allow_exec: bool) -> Self {
        Self::new_with_channels(allow_write, allow_exec, None)
    }

    pub fn new_with_channels(
        allow_write: bool,
        allow_exec: bool,
        ask_user_channel: Option<AskUserChannel>,
    ) -> Self {
        let undo_history = UndoHistory::new(50);

        // Build capability set from legacy boolean flags.
        let mut capabilities = HashSet::new();
        // All registries get read + network + user-interaction.
        capabilities.insert(ToolCapability::ReadFs);
        capabilities.insert(ToolCapability::NetworkFetch);
        capabilities.insert(ToolCapability::UserInteraction);
        if allow_write {
            capabilities.insert(ToolCapability::WriteFs);
        }
        if allow_exec {
            capabilities.insert(ToolCapability::ExecShell);
        }

        let mut registry = Self {
            tools: HashMap::new(),
            _allow_write: allow_write,
            _allow_exec: allow_exec,
            capabilities,
            undo_history: undo_history.clone(),
        };

        registry.register(Box::new(read_file::ReadFileTool));
        registry.register(Box::new(list_dir::ListDirTool));
        registry.register(Box::new(search_files::SearchFilesTool));
        registry.register(Box::new(get_url::GetUrlTool));
        registry.register(Box::new(undo::UndoTool::new(undo_history)));

        if let Some(channel) = ask_user_channel {
            registry.register(Box::new(ask_user::AskUserTool::new(channel)));
        }

        if allow_write {
            registry.register(Box::new(write_file::WriteFileTool));
            registry.register(Box::new(edit_file::EditFileTool));
        }
        if allow_exec {
            registry.register(Box::new(bash::BashTool));
            registry.register(Box::new(forge_tool::ForgeTool));
        }

        registry
    }

    /// Create a registry restricted to a specific capability set.
    /// Tools whose required capabilities are not granted will be omitted.
    pub fn new_with_capabilities(
        capabilities: HashSet<ToolCapability>,
        ask_user_channel: Option<AskUserChannel>,
    ) -> Self {
        let allow_write = capabilities.contains(&ToolCapability::WriteFs);
        let allow_exec = capabilities.contains(&ToolCapability::ExecShell);
        let undo_history = UndoHistory::new(50);

        let mut registry = Self {
            tools: HashMap::new(),
            _allow_write: allow_write,
            _allow_exec: allow_exec,
            capabilities,
            undo_history: undo_history.clone(),
        };

        registry.register(Box::new(read_file::ReadFileTool));
        registry.register(Box::new(list_dir::ListDirTool));
        registry.register(Box::new(search_files::SearchFilesTool));
        registry.register(Box::new(get_url::GetUrlTool));
        registry.register(Box::new(undo::UndoTool::new(undo_history)));

        if let Some(channel) = ask_user_channel {
            registry.register(Box::new(ask_user::AskUserTool::new(channel)));
        }

        if allow_write {
            registry.register(Box::new(write_file::WriteFileTool));
            registry.register(Box::new(edit_file::EditFileTool));
        }
        if allow_exec {
            registry.register(Box::new(bash::BashTool));
            registry.register(Box::new(forge_tool::ForgeTool));
        }

        registry
    }

    /// Return capability sets for the three orchestration roles.
    pub fn executor_capabilities() -> HashSet<ToolCapability> {
        [
            ToolCapability::ReadFs,
            ToolCapability::WriteFs,
            ToolCapability::ExecShell,
            ToolCapability::NetworkFetch,
            ToolCapability::UserInteraction,
        ]
        .into_iter()
        .collect()
    }

    pub fn critic_capabilities() -> HashSet<ToolCapability> {
        [ToolCapability::ReadFs].into_iter().collect()
    }

    pub fn planner_capabilities() -> HashSet<ToolCapability> {
        // Planner plans, doesn't execute — no tool access.
        HashSet::new()
    }

    fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Check whether the registry has all capabilities required by the named tool.
    fn check_capabilities(&self, name: &str) -> Result<()> {
        let Some(tool) = self.tools.get(name) else {
            // Unknown tool — will be caught in execute().
            return Ok(());
        };
        let missing: Vec<_> = tool
            .required_capabilities()
            .iter()
            .filter(|cap| !self.capabilities.contains(cap))
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            let names: Vec<String> = missing.iter().map(|c| c.to_string()).collect();
            Err(oni_core::error::err!(
                "Tool '{}' requires capabilities not granted to this context: [{}]",
                name,
                names.join(", ")
            ))
        }
    }

    pub fn execute(&self, name: &str, args: serde_json::Value) -> Result<String> {
        // Capability gate — returns an error message if capabilities are missing.
        self.check_capabilities(name)?;

        // Snapshot before write operations so they can be undone.
        if name == "write_file" || name == "edit_file" {
            if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                self.undo_history.snapshot(path);
            }
        }

        match self.tools.get(name) {
            Some(tool) => tool.execute(args),
            None => Err(oni_core::error::err!("Unknown tool: {}", name)),
        }
    }

    pub fn tool_schemas(&self) -> Vec<serde_json::Value> {
        self.tools.values().map(|t| t.schema()).collect()
    }

    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
}
