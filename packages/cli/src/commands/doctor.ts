import { Command } from "commander";
import chalk from "chalk";

interface Check {
  label: string;
  ok: boolean;
  detail: string;
}

const PASS = chalk.green("\u2713");
const FAIL = chalk.red("\u2717");

function report(checks: Check[]): void {
  console.log();
  console.log(chalk.bold("oni doctor"));
  console.log(chalk.dim("\u2500".repeat(40)));

  for (const check of checks) {
    const icon = check.ok ? PASS : FAIL;
    const label = check.ok ? chalk.white(check.label) : chalk.red(check.label);
    console.log(`  ${icon} ${label}  ${chalk.dim(check.detail)}`);
  }

  const failed = checks.filter((c) => !c.ok).length;
  console.log();

  if (failed === 0) {
    console.log(chalk.green("All checks passed."));
  } else {
    console.log(chalk.red(`${failed} check${failed > 1 ? "s" : ""} failed.`));
  }
}

export const doctorCommand = new Command("doctor")
  .description("Check ONI setup and diagnose common issues")
  .action(async () => {
    const checks: Check[] = [];

    // 1. CLI version
    checks.push({
      label: "oni version",
      ok: true,
      detail: "0.1.0",
    });

    // 2. Node.js version
    const nodeVersion = process.versions.node;
    const nodeMajor = Number.parseInt(nodeVersion.split(".")[0], 10);
    checks.push({
      label: "node.js",
      ok: nodeMajor >= 20,
      detail: nodeMajor >= 20
        ? `v${nodeVersion}`
        : `v${nodeVersion} (requires 20+)`,
    });

    // 3. API key
    try {
      const { resolveApiKey } = await import("@oni/auth");
      const resolved = await resolveApiKey();
      checks.push({
        label: "api key",
        ok: resolved !== null,
        detail: resolved
          ? `found (${resolved.source})`
          : "not found \u2014 run `oni login` or set ANTHROPIC_API_KEY",
      });
    } catch (err) {
      checks.push({
        label: "api key",
        ok: false,
        detail: `keychain error: ${(err as Error).message}`,
      });
    }

    // 4. SQLite database
    try {
      const { existsSync } = await import("node:fs");
      const { join } = await import("node:path");
      const { getDataDir } = await import("../config.js");
      const dbPath = join(getDataDir(), "oni.db");
      const exists = existsSync(dbPath);

      if (exists) {
        const { createDatabase } = await import("@oni/db");
        const db = createDatabase(dbPath);
        db.close();
        checks.push({
          label: "database",
          ok: true,
          detail: dbPath,
        });
      } else {
        checks.push({
          label: "database",
          ok: true,
          detail: "not yet created (will be created on first chat)",
        });
      }
    } catch (err) {
      checks.push({
        label: "database",
        ok: false,
        detail: `sqlite error: ${(err as Error).message}`,
      });
    }

    // 5. Project index
    try {
      const { existsSync } = await import("node:fs");
      const { join } = await import("node:path");
      const projectDir = process.cwd();
      const indexPath = join(projectDir, ".oni", "index.db");
      const indexed = existsSync(indexPath);
      checks.push({
        label: "project index",
        ok: indexed,
        detail: indexed
          ? `indexed (${projectDir})`
          : "not indexed \u2014 run `oni init` in your project",
      });
    } catch {
      checks.push({
        label: "project index",
        ok: false,
        detail: "could not check project index",
      });
    }

    report(checks);

    const failed = checks.filter((c) => !c.ok).length;
    if (failed > 0) {
      process.exitCode = 1;
    }
  });
