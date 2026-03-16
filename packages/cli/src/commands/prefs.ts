import { Command } from "commander";
import chalk from "chalk";

const c = {
  lime: chalk.hex("#8fff00"),
  cyan: chalk.hex("#00d4c8"),
  coral: chalk.hex("#ff3060"),
  amber: chalk.hex("#f5a623"),
  muted: chalk.hex("#6060aa"),
  text: chalk.hex("#8888cc"),
  dim: chalk.hex("#3a3a6a"),
};

export const prefsCommand = new Command("prefs")
  .description("Manage learned preferences and rules");

prefsCommand
  .command("list")
  .description("Show all learned rules and their confidence")
  .action(async () => {
    const Database = (await import("better-sqlite3")).default;
    const { PreferenceEngine } = await import("@oni/prefs");
    const { getDataDir } = await import("../config.js");
    const { join } = await import("node:path");

    const dbPath = join(getDataDir(), "oni.db");
    const db = new Database(dbPath);
    const engine = new PreferenceEngine(db);

    const rules = engine.activeRules();
    const stats = engine.stats();

    console.log();
    console.log(c.lime.bold("LEARNED RULES") + c.muted(` ${stats.activeRules} ACTIVE · ${stats.totalEvents} EVENTS`));
    console.log(c.dim("━".repeat(60)));

    if (rules.length === 0) {
      console.log(c.muted("  NO LEARNED RULES YET. USE ONI CHAT TO BUILD PREFERENCES."));
    } else {
      for (const rule of rules) {
        const pct = Math.round(rule.confidence * 100);
        const bar = pct >= 85 ? c.lime(`${pct}%`) : pct >= 50 ? c.amber(`${pct}%`) : c.coral(`${pct}%`);
        console.log(`  ${c.text(rule.action)}  ${bar}`);
      }
    }

    console.log();
    console.log(c.muted(`${stats.totalPrefs} TOOL PREFERENCES · ${stats.totalEvents} TOTAL EVENTS`));

    db.close();
  });

prefsCommand
  .command("reset")
  .description("Clear all preferences and learned rules")
  .option("--tool <name>", "Reset only a specific tool")
  .action(async (options) => {
    const Database = (await import("better-sqlite3")).default;
    const { PreferenceEngine } = await import("@oni/prefs");
    const { getDataDir } = await import("../config.js");
    const { join } = await import("node:path");

    const dbPath = join(getDataDir(), "oni.db");
    const db = new Database(dbPath);
    const engine = new PreferenceEngine(db);

    const tool = options.tool as string | undefined;
    engine.reset(tool);

    if (tool) {
      console.log(c.lime(`RESET PREFERENCES FOR ${tool.toUpperCase()}.`));
    } else {
      console.log(c.lime("ALL PREFERENCES RESET."));
    }

    db.close();
  });

prefsCommand
  .command("stats")
  .description("Show preference statistics")
  .action(async () => {
    const Database = (await import("better-sqlite3")).default;
    const { PreferenceEngine } = await import("@oni/prefs");
    const { getDataDir } = await import("../config.js");
    const { join } = await import("node:path");

    const dbPath = join(getDataDir(), "oni.db");
    const db = new Database(dbPath);
    const engine = new PreferenceEngine(db);

    const stats = engine.stats();
    console.log();
    console.log(c.lime("PREFERENCE STATS"));
    console.log(c.dim("━".repeat(40)));
    console.log(`  ${c.muted("TOOLS TRACKED")}  ${c.text(String(stats.totalPrefs))}`);
    console.log(`  ${c.muted("ACTIVE RULES")}   ${c.text(String(stats.activeRules))}`);
    console.log(`  ${c.muted("TOTAL EVENTS")}   ${c.text(String(stats.totalEvents))}`);
    console.log();

    db.close();
  });
