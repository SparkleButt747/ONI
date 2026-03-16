# ONI — Contributing Guide

Everything you need to run, build, and contribute to ONI. A new contributor should be running tests within 15 minutes of reading this.

---

## Prerequisites

- **Node.js 20+** (`node --version` to check)
- **npm 10+**
- **Git**
- For context engine tests: `ripgrep` (`rg --version`)
- For E2E tests: `gh` CLI (`gh --version`)
- For macOS keychain tests: macOS (Linux CI uses libsecret mock)

---

## Setup

```bash
git clone https://github.com/yourorg/oni.git
cd oni
npm install
npm run build
npm test
```

That's it. If tests pass, you're set up correctly.

### First run
```bash
# Link for local development
npm link

# Run against real Claude (requires ONI OAuth)
oni login
oni chat
```

---

## Repo Structure

```
oni/
├── packages/
│   ├── cli/           # Entry point, commander.js routing
│   │   ├── src/
│   │   │   ├── commands/    # one file per command (chat.ts, ask.ts, mc.ts, ...)
│   │   │   └── index.ts     # commander setup
│   │   └── package.json
│   ├── agent/         # LangGraph state machine, sub-agents
│   │   ├── src/
│   │   │   ├── nodes/       # planner.ts, executor.ts, critic.ts
│   │   │   ├── state.ts     # ONIState type and transitions
│   │   │   └── graph.ts     # LangGraph graph definition
│   │   └── package.json
│   ├── context/       # tree-sitter indexer, retrieval
│   │   ├── src/
│   │   │   ├── indexer.ts
│   │   │   ├── retriever.ts
│   │   │   ├── ranker.ts
│   │   │   └── packer.ts
│   │   └── package.json
│   ├── auth/          # OAuth PKCE, keytar
│   ├── sync/          # claude.ai sync daemon
│   ├── tui/           # ink components
│   │   ├── src/
│   │   │   ├── MissionControl.tsx
│   │   │   ├── REPL.tsx
│   │   │   ├── DiffPanel.tsx
│   │   │   └── components/   # shared ink components
│   │   └── package.json
│   ├── plugins/       # MCP client, tool broker, plugin manager
│   ├── prefs/         # preference learning, rule engine
│   ├── db/            # SQLite schemas, migrations, query helpers
│   └── test-fixtures/ # shared test data
├── scripts/           # dev tooling (release, install scripts)
├── docs/              # this documentation
├── .github/
│   └── workflows/     # CI pipelines
├── biome.json
├── tsconfig.base.json
└── package.json       # root workspace
```

---

## Commands

```bash
npm run build          # build all packages (tsup)
npm run dev            # build + watch mode
npm test               # unit tests (vitest)
npm run test:coverage  # with coverage report
npm run test:integration  # integration tests
npm run evals          # LLM evals (requires ANTHROPIC_API_KEY)
npm run lint           # biome check
npm run lint:fix       # biome check --write
npm run typecheck      # tsc --noEmit across all packages
npm run clean          # remove all dist/ and .cache/
```

---

## Code Conventions

### TypeScript
- Strict mode: `"strict": true` in all tsconfig.json files
- No `any` — use `unknown` and narrow
- No `!` non-null assertion — handle null cases explicitly
- Explicit return types on public functions
- `interface` over `type` for object shapes; `type` for unions/intersections

### Naming
- Files: `kebab-case.ts`
- Classes: `PascalCase`
- Functions and variables: `camelCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Exported types/interfaces: `PascalCase`

### Error handling
- Use typed error classes, not string throws:
  ```typescript
  class AuthError extends Error {
    constructor(message: string, public code: string) {
      super(message)
      this.name = 'AuthError'
    }
  }
  ```
- Never swallow errors silently
- All async operations in a try/catch with specific error handling

### SQLite
- Use `better-sqlite3` synchronously — no async wrappers
- All schema changes via migration files in `packages/db/src/migrations/`
- Migration format: `0001_create_conversations.ts`
- Never write raw SQL strings in business logic — use query helpers in `packages/db/`

### Logging
- Use structured logging: `log.info({ tool, latency_ms }, 'tool completed')`
- Never log tokens, API keys, or file content
- Log level controlled by `ONI_LOG_LEVEL` env var (debug/info/warn/error)
- No `console.log` in packages — only in CLI entry point

---

## Adding a New Command

1. Create `packages/cli/src/commands/<name>.ts`
2. Export a `Command` from commander.js
3. Register in `packages/cli/src/index.ts`
4. Add tests in `packages/cli/src/__tests__/<name>.test.ts`
5. Add to `FEATURES.md` with acceptance criteria
6. Update `oni help` output (automatically via commander)

```typescript
// packages/cli/src/commands/example.ts
import { Command } from 'commander'

export const exampleCommand = new Command('example')
  .description('Does something useful')
  .option('--flag', 'Enable a flag')
  .action(async (options) => {
    // implementation
  })
```

---

## Adding a New Tool (Built-in)

1. Define in `packages/agent/src/tools/`
2. Schema via Zod, exported as `ZodSchema`
3. Register in `packages/agent/src/tools/index.ts`
4. Tool broker automatically picks it up
5. Add to `FEATURES.md` under the relevant feature section
6. Add eval fixture in `packages/test-fixtures/evals/`

```typescript
// packages/agent/src/tools/example.ts
import { z } from 'zod'

export const exampleToolSchema = z.object({
  path: z.string().describe('File path to operate on'),
})

export async function exampleTool(args: z.infer<typeof exampleToolSchema>): Promise<string> {
  // implementation
  return result
}
```

---

## Adding a New MCP Plugin to the Registry

1. Verify plugin meets registry criteria (see SECURITY.md)
2. Add entry to `packages/plugins/src/registry.json`:
   ```json
   {
     "name": "myplugin",
     "description": "Does useful things",
     "source": "https://github.com/org/mcp-myplugin",
     "transport": "stdio",
     "install": "npx mcp-myplugin",
     "tools": ["tool_one", "tool_two"],
     "auth": { "type": "env_var", "envVar": "MYPLUGIN_TOKEN" }
   }
   ```
3. Open PR with justification for inclusion
4. Plugin reviewed by maintainer before merge

---

## Pull Request Process

### Before opening a PR
- [ ] `npm test` passes
- [ ] `npm run typecheck` passes
- [ ] `npm run lint` passes (no warnings)
- [ ] New code has unit tests
- [ ] New features have acceptance criteria in `FEATURES.md` (or linked issue)
- [ ] No new `any` types introduced
- [ ] No secrets, tokens, or API keys in code or tests

### PR description template
```markdown
## What
Brief description of the change.

## Why
Why this change is needed.

## How
Key implementation decisions.

## Testing
How you tested this. Which tests are new.

## Checklist
- [ ] Tests pass
- [ ] Types clean
- [ ] Lint clean
- [ ] FEATURES.md updated (if applicable)
- [ ] PHASES.md exit criteria checked (if this closes a milestone item)
```

### Review criteria
- One approving review required for merge
- CI must pass (unit + integration + lint + typecheck)
- No decrease in coverage without explicit justification

---

## Release Process

1. Update version in root `package.json`
2. Update `CHANGELOG.md` (generated via `npx changeset`)
3. `git tag v{version} && git push --tags`
4. GitHub Actions release workflow:
   - Runs full test suite
   - Builds standalone binary
   - Publishes to npm
   - Creates GitHub release with changelog
   - Updates Homebrew tap formula

---

## Getting Help

- Open an issue for bugs or feature requests
- Tag `[question]` for usage questions
- Check `ARCHITECTURE.md` before asking about system design
- Check `FEATURES.md` for intended behaviour before filing a bug

---

## Licence

ONI is released under the MIT licence. All contributors retain copyright of their contributions and grant ONI the right to include them under this licence.
