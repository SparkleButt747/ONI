#!/usr/bin/env node

// Development: use tsx to run TypeScript directly
// Production: this file would be the compiled output from tsup/esbuild
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const entry = join(__dirname, "..", "src", "index.ts");

try {
  execFileSync("npx", ["tsx", entry, ...process.argv.slice(2)], {
    stdio: "inherit",
    cwd: process.cwd(),
  });
} catch (err) {
  process.exit(err.status ?? 1);
}
