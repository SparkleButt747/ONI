import { readFileSync, existsSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";

export interface OniConfig {
  model: string;
  contextBudget: number;
  monthlyTokenLimit: number;
  verbosity: "silent" | "normal" | "verbose";
  dryRunDefault: boolean;
  colors: boolean;
}

const DEFAULTS: OniConfig = {
  model: "claude-sonnet-4-6",
  contextBudget: 80_000,
  monthlyTokenLimit: 0,
  verbosity: "normal",
  dryRunDefault: true,
  colors: true,
};

function getConfigDir(): string {
  return join(homedir(), ".config", "oni");
}

function getConfigPath(): string {
  return join(getConfigDir(), "config.json");
}

function loadJsonSafe(path: string): Record<string, unknown> {
  try {
    if (existsSync(path)) {
      return JSON.parse(readFileSync(path, "utf-8"));
    }
  } catch {
    // ignore malformed config
  }
  return {};
}

export function loadConfig(projectDir?: string): OniConfig {
  const globalConfig = loadJsonSafe(getConfigPath());
  const localConfig = projectDir
    ? loadJsonSafe(join(projectDir, ".oni", "config.json"))
    : {};

  return {
    ...DEFAULTS,
    ...globalConfig,
    ...localConfig,
  } as OniConfig;
}

export function setConfigValue(key: string, value: string): void {
  const dir = getConfigDir();
  mkdirSync(dir, { recursive: true });
  const path = getConfigPath();
  const existing = loadJsonSafe(path);
  // Try to parse as number or boolean
  let parsed: unknown = value;
  if (value === "true") parsed = true;
  else if (value === "false") parsed = false;
  else if (!isNaN(Number(value)) && value.length > 0) parsed = Number(value);
  existing[key] = parsed;
  writeFileSync(path, JSON.stringify(existing, null, 2) + "\n");
}

export function getConfigValue(key: string): unknown {
  const config = loadConfig();
  return (config as unknown as Record<string, unknown>)[key];
}

export function getDataDir(): string {
  const dir = join(homedir(), ".local", "share", "oni");
  mkdirSync(dir, { recursive: true });
  return dir;
}
