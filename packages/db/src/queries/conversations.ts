import { randomUUID } from "node:crypto";
import type { DB } from "../database.js";

export interface Conversation {
  conv_id: string;
  source: string;
  created_at: number;
  last_active: number;
  project_dir: string | null;
}

export function createConversation(db: DB, projectDir: string): Conversation {
  const now = Date.now();
  const conv: Conversation = {
    conv_id: randomUUID(),
    source: "local",
    created_at: now,
    last_active: now,
    project_dir: projectDir,
  };
  db.prepare(
    "INSERT INTO conversations (conv_id, source, created_at, last_active, project_dir) VALUES (?, ?, ?, ?, ?)",
  ).run(conv.conv_id, conv.source, conv.created_at, conv.last_active, conv.project_dir);
  return conv;
}

export function getConversation(db: DB, convId: string): Conversation | undefined {
  return db
    .prepare("SELECT * FROM conversations WHERE conv_id = ?")
    .get(convId) as Conversation | undefined;
}

export function getLatestConversation(db: DB): Conversation | undefined {
  return db
    .prepare("SELECT * FROM conversations ORDER BY last_active DESC LIMIT 1")
    .get() as Conversation | undefined;
}

export function touchConversation(db: DB, convId: string): void {
  db.prepare("UPDATE conversations SET last_active = ? WHERE conv_id = ?").run(
    Date.now(),
    convId,
  );
}
