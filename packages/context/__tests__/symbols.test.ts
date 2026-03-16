import { describe, it, expect } from "vitest";
import { extractSymbols } from "../src/symbols.js";

describe("symbols", () => {
  it("T-SYM-1: extracts function definitions from TypeScript", () => {
    const code = `
export function greet(name: string): string {
  return "hello " + name;
}

async function fetchData(url: string) {
  return fetch(url);
}
`;
    const symbols = extractSymbols(code, "typescript");
    const names = symbols.map((s) => s.name);
    expect(names).toContain("greet");
    expect(names).toContain("fetchData");

    const greet = symbols.find((s) => s.name === "greet")!;
    expect(greet.kind).toBe("function");
    expect(greet.startLine).toBe(2);
  });

  it("T-SYM-2: extracts class definitions from TypeScript", () => {
    const code = `
export class Database {
  constructor(private path: string) {}
  query(sql: string) { return []; }
}
`;
    const symbols = extractSymbols(code, "typescript");
    const db = symbols.find((s) => s.name === "Database");
    expect(db).toBeDefined();
    expect(db!.kind).toBe("class");
  });

  it("T-SYM-3: extracts interfaces and types from TypeScript", () => {
    const code = `
export interface User {
  id: number;
  name: string;
}

export type Config = {
  debug: boolean;
};
`;
    const symbols = extractSymbols(code, "typescript");
    const names = symbols.map((s) => s.name);
    expect(names).toContain("User");
    expect(names).toContain("Config");

    expect(symbols.find((s) => s.name === "User")!.kind).toBe("interface");
    expect(symbols.find((s) => s.name === "Config")!.kind).toBe("type");
  });

  it("T-SYM-4: extracts export const/let from TypeScript", () => {
    const code = `
export const MAX_SIZE = 1024;
export let counter = 0;
`;
    const symbols = extractSymbols(code, "typescript");
    const names = symbols.map((s) => s.name);
    expect(names).toContain("MAX_SIZE");
    expect(names).toContain("counter");
  });

  it("T-SYM-5: extracts def and class from Python", () => {
    const code = `
class MyService:
    def __init__(self, name):
        self.name = name

    def run(self):
        pass

def helper(x, y):
    return x + y

async def fetch_data(url):
    pass
`;
    const symbols = extractSymbols(code, "python");
    const names = symbols.map((s) => s.name);
    expect(names).toContain("MyService");
    expect(names).toContain("__init__");
    expect(names).toContain("run");
    expect(names).toContain("helper");
    expect(names).toContain("fetch_data");

    expect(symbols.find((s) => s.name === "MyService")!.kind).toBe("class");
    expect(symbols.find((s) => s.name === "helper")!.kind).toBe("function");
  });

  it("T-SYM-6: returns empty array for unsupported languages", () => {
    const symbols = extractSymbols("some content", "markdown");
    expect(symbols).toEqual([]);
  });

  it("T-SYM-7: returns correct line numbers", () => {
    const code = `// line 1
// line 2
function foo() {
  return 1;
}
// line 6
function bar() {
  return 2;
}
`;
    const symbols = extractSymbols(code, "typescript");
    const foo = symbols.find((s) => s.name === "foo")!;
    const bar = symbols.find((s) => s.name === "bar")!;
    expect(foo.startLine).toBe(3);
    expect(bar.startLine).toBe(7);
  });
});
