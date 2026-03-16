# ONI — Testing Strategy

---

## Testing Philosophy

ONI is an agent. Agents fail in ways that differ from normal software:
- Non-deterministic outputs from Claude
- Tool call sequences that depend on LLM reasoning
- Preference state that accumulates over sessions
- External API dependencies (claude.ai, GitHub, MCP servers)

Our testing strategy addresses this by separating concerns: deterministic code is unit-tested normally; LLM-dependent behaviour is tested via evals with fixtures; integration is tested with mocked APIs.

---

## Test Pyramid

```
                    ┌──────────┐
                    │  E2E /   │  ← ~10 tests, slow, real network
                    │  Smoke   │
                  ┌─┴──────────┴─┐
                  │  Integration  │  ← ~80 tests, mocked APIs
                ┌─┴──────────────┴─┐
                │    Unit Tests     │  ← ~300 tests, fast, deterministic
              ┌─┴──────────────────┴─┐
              │   LLM Evals (offline) │  ← ~50 fixtures, non-deterministic
              └────────────────────────┘
```

---

## Unit Tests (Vitest)

### Coverage targets
- `packages/auth/` — 95%+
- `packages/context/` — 90%+
- `packages/prefs/` — 90%+
- `packages/db/` — 85%+
- `packages/agent/` state machine — 90%+
- `packages/plugins/` tool broker — 85%+

### What to unit test

**Auth module:**
```typescript
// src/auth/__tests__/pkce.test.ts
describe('PKCE', () => {
  test('generates code_verifier of valid length', () => {
    const v = generateVerifier()
    expect(v.length).toBeGreaterThanOrEqual(43)
    expect(v.length).toBeLessThanOrEqual(128)
  })

  test('code_challenge is base64url-encoded SHA-256', async () => {
    const challenge = await generateChallenge('abc123')
    expect(challenge).toMatch(/^[A-Za-z0-9_-]+$/)
  })

  test('challenge differs from verifier', async () => {
    const v = generateVerifier()
    const c = await generateChallenge(v)
    expect(c).not.toBe(v)
  })
})
```

**Context engine:**
```typescript
// src/context/__tests__/ranker.test.ts
describe('Ranker', () => {
  test('recency decay reduces weight exponentially', () => {
    const now = Date.now()
    const oneHourAgo = now - 3600000
    const w1 = recencyWeight(now)
    const w2 = recencyWeight(oneHourAgo)
    expect(w1).toBeGreaterThan(w2)
    expect(w2).toBeCloseTo(Math.exp(-1/0.5), 3) // half-life 30min
  })

  test('token budget never exceeded', () => {
    const chunks = generateChunks(100, 1000) // 100 chunks of 1000 tokens
    const packed = pack(chunks, { budget: 20000 })
    const total = packed.reduce((s, c) => s + c.tokens, 0)
    expect(total).toBeLessThanOrEqual(20000)
  })
})
```

**Preference engine:**
```typescript
// src/prefs/__tests__/weights.test.ts
describe('Preference weights', () => {
  test('accept signal increases weight', async () => {
    const db = createTestDb()
    await recordSignal(db, 'bash', 'debug', 'accept')
    const w = getWeight(db, 'bash', 'debug')
    expect(w).toBeGreaterThan(0.5)
  })

  test('decay applied on read, not write', async () => {
    const db = createTestDb()
    insertOldObservation(db, 'web_search', 'debug', 30) // 30 days ago
    const w = getWeight(db, 'web_search', 'debug')
    expect(w).toBeLessThan(0.5) // decayed significantly
  })

  test('always flag sets weight to 1.0 and bypasses proposal', async () => {
    const db = createTestDb()
    await recordSignal(db, 'read_file', 'refactor', 'always')
    const w = getWeight(db, 'read_file', 'refactor')
    expect(w).toBe(1.0)
  })
})
```

**Tool broker:**
```typescript
// src/plugins/__tests__/broker.test.ts
describe('ToolBroker', () => {
  test('namespaces plugin tool that conflicts with built-in', async () => {
    const broker = new ToolBroker()
    await broker.loadPlugin(mockPlugin({ name: 'github', tools: ['bash'] }))
    const tools = broker.allTools().map(t => t.name)
    expect(tools).toContain('bash')           // built-in
    expect(tools).toContain('github:bash')    // namespaced
    expect(tools.filter(t => t === 'bash').length).toBe(1) // no dupe
  })

  test('plugin crash does not remove built-in tools', async () => {
    const broker = new ToolBroker()
    await broker.loadPlugin(mockCrashingPlugin())
    const tools = broker.allTools().map(t => t.name)
    expect(tools).toContain('read_file')
    expect(tools).toContain('bash')
  })
})
```

**Agent state machine:**
```typescript
// src/agent/__tests__/state.test.ts
describe('Agent state machine', () => {
  test('critic reject increments replanCount', async () => {
    const state = createState()
    const next = await criticRejectTransition(state)
    expect(next.replanCount).toBe(1)
    expect(next.status).toBe('planning')
  })

  test('replanCount ≥ 2 transitions to blocked', async () => {
    const state = createState({ replanCount: 2 })
    const next = await criticRejectTransition(state)
    expect(next.status).toBe('blocked')
  })

  test('executor blocker transitions to blocked', async () => {
    const state = createState()
    const next = await executorBlockerTransition(state, 'CI requires approval')
    expect(next.status).toBe('blocked')
    expect(next.blocker).toBe('CI requires approval')
  })
})
```

### Running unit tests
```bash
npm test                   # all unit tests
npm test -- --watch        # watch mode
npm test -- packages/auth  # single package
npm run test:coverage      # with coverage report
```

---

## Integration Tests

Mock external APIs. Test module interactions with realistic data.

### Mocking strategy
- **Claude API:** `msw` (Mock Service Worker) intercepts fetch, returns fixture SSE streams
- **claude.ai sync API:** msw intercepts polling endpoints
- **MCP servers:** mock stdio process with scripted responses
- **SQLite:** real SQLite in `:memory:` database — not mocked
- **keytar:** stubbed with in-memory map

### Key integration tests

**Auth flow (PKCE round-trip):**
```typescript
test('OAuth PKCE flow stores token in keychain', async () => {
  const mockServer = startMockAuthServer()
  const keychainSpy = mockKeytar()
  
  await runOniLogin({ port: 3841 })
  
  expect(keychainSpy.get('oni-cli', 'access_token')).toBeDefined()
  expect(keychainSpy.get('oni-cli', 'refresh_token')).toBeDefined()
  // No token in any config file
  expect(fs.existsSync('~/.config/oni/token')).toBe(false)
})
```

**Context engine + agent integration:**
```typescript
test('context engine chunks injected into system prompt', async () => {
  const project = scaffoldProject({ files: testProject })
  await indexProject(project.dir)
  
  const captured = []
  const agent = createAgent({ onAPICall: req => captured.push(req) })
  await agent.run('explain the auth middleware', { cwd: project.dir })
  
  const systemPrompt = captured[0].system
  expect(systemPrompt).toContain('src/auth/middleware.ts')
})
```

**Preference learning cycle:**
```typescript
test('accept signal raises score above auto-threshold after 10 observations', async () => {
  const db = createTestDb()
  
  for (let i = 0; i < 10; i++) {
    await recordSignal(db, 'bash', 'debug', 'accept')
  }
  
  const score = computeScore(db, 'bash', 'debug')
  expect(score).toBeGreaterThanOrEqual(0.85)
})
```

**MCP plugin integration:**
```typescript
test('github plugin tools appear in Claude API call', async () => {
  const server = startMockMCPServer({ tools: ['create_pr', 'list_issues'] })
  const broker = new ToolBroker()
  await broker.loadPlugin({ name: 'github', transport: 'stdio', process: server })
  
  const tools = broker.allTools().map(t => t.name)
  expect(tools).toContain('github:create_pr')
  expect(tools).toContain('github:list_issues')
  expect(tools).toContain('read_file') // built-ins still present
})
```

### Running integration tests
```bash
npm run test:integration
```

---

## LLM Evals (Offline Fixtures)

LLM behaviour is non-deterministic. We test it with:
1. **Golden fixtures** — curated input/output pairs tested against exact criteria
2. **Behavioural assertions** — test that outputs satisfy properties, not exact match

### Eval framework
Custom eval runner in `packages/evals/`:
```typescript
interface Eval {
  name: string
  input: { role: 'user' | 'assistant', content: string }[]
  assertions: Assertion[]
}

type Assertion =
  | { type: 'contains', value: string }
  | { type: 'not_contains', value: string }
  | { type: 'has_tool_call', tool: string }
  | { type: 'no_tool_call', tool: string }
  | { type: 'sub_agent_prefix', agent: 'planner' | 'executor' | 'critic' }
  | { type: 'max_length', chars: number }
```

### Critical evals

**Planner decomposes correctly:**
```yaml
name: planner_decomposes_multi_file_task
input:
  - role: user
    content: "refactor the auth module to use async/await throughout"
assertions:
  - type: sub_agent_prefix
    agent: planner
  - type: contains
    value: "subtask"     # or numbered list
  - type: not_contains
    value: "I will"      # executor narrating intention (wrong)
```

**Executor reports completed actions, not intentions:**
```yaml
name: executor_reports_actions_not_intentions
input:
  - role: user
    content: "read the middleware file"
  - role: assistant
    content: "[Σ] Plan: read middleware.ts. 1 subtask."
assertions:
  - type: has_tool_call
    tool: read_file
  - type: not_contains
    value: "I'm going to"
  - type: not_contains
    value: "I will now"
```

**Critic rejects deliberately bad output:**
```yaml
name: critic_rejects_security_issue
input:
  - role: user
    content: "store the JWT secret in the env file"
  - role: assistant
    content: "[⚡] Written JWT_SECRET to .env.staging in plaintext."
assertions:
  - type: sub_agent_prefix
    agent: critic
  - type: contains_any
    values: ["plaintext", "insecure", "reject", "risk"]
```

**Personality: no filler phrases:**
```yaml
name: no_comfort_phrasing
input:
  - role: user
    content: "why is the build failing"
assertions:
  - type: not_contains
    value: "Great question"
  - type: not_contains
    value: "I'd be happy to"
  - type: not_contains
    value: "Certainly"
  - type: not_contains
    value: "Of course"
  - type: max_length
    chars: 500   # concise response
```

**Context retrieval quality (20 queries):**
Each query has an expected file in top-3 results. Pass rate target: ≥16/20 (80%).

### Running evals
```bash
npm run evals               # run all evals (requires ANTHROPIC_API_KEY)
npm run evals -- --fixture planner_decomposes  # single eval
npm run evals -- --report   # save JSON report
```

Note: evals call the real Claude API. Budget approximately $2–5 per full eval run.

---

## E2E / Smoke Tests

Minimal set. Real network, real Claude API. Run in CI on release tags only.

### Smoke test suite
```typescript
describe('E2E Smoke Tests', () => {
  test('oni login completes PKCE flow', async () => {
    // Requires browser automation (Playwright)
    // or a mock claude.ai auth server
    const result = await runOniLogin()
    expect(result.exitCode).toBe(0)
  })

  test('oni ask reads a file and explains it', async () => {
    const result = await run('echo "explain this" | oni ask', { cwd: testProject })
    expect(result.stdout).toContain(testProject.knownSymbol)
    expect(result.exitCode).toBe(0)
  })

  test('oni chat completes a one-turn session', async () => {
    const result = await runInteractive(['oni chat', ':q'])
    expect(result.exitCode).toBe(0)
  })
})
```

### Running E2E
```bash
ANTHROPIC_API_KEY=xxx npm run test:e2e
```

---

## CI Pipeline

```yaml
# .github/workflows/ci.yml
on: [push, pull_request]

jobs:
  unit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: '20' }
      - run: npm ci
      - run: npm test
      - run: npm run test:coverage
      - uses: codecov/codecov-action@v4

  integration:
    runs-on: ubuntu-latest
    steps:
      - run: npm run test:integration

  lint:
    runs-on: ubuntu-latest
    steps:
      - run: npx biome check .
      - run: npx tsc --noEmit

  evals:
    runs-on: ubuntu-latest
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    env:
      ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
    steps:
      - run: npm run evals -- --report
      - uses: actions/upload-artifact@v4
        with:
          name: eval-report
          path: eval-report.json

  e2e:
    runs-on: ubuntu-latest
    if: startsWith(github.ref, 'refs/tags/')
    env:
      ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
    steps:
      - run: npm run test:e2e
```

---

## Coverage Thresholds

```json
// vitest.config.ts
{
  "coverage": {
    "thresholds": {
      "lines": 80,
      "functions": 80,
      "branches": 75,
      "statements": 80
    }
  }
}
```

PRs that drop coverage below threshold fail CI.

---

## Test Data and Fixtures

- `packages/test-fixtures/projects/` — small TypeScript, Python, Rust projects for context engine tests
- `packages/test-fixtures/evals/` — YAML eval definitions
- `packages/test-fixtures/api-responses/` — MSW handler fixtures (SSE streams, sync API responses)
- `packages/test-fixtures/mcp/` — mock MCP server scripts

All fixtures are committed. No fixture should require a live API call.
