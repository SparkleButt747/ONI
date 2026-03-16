import { randomUUID } from "node:crypto";
import type { DB } from "../database.js";

export interface StoredMessage {
  msg_id: string;
  conv_id: string;
  role: string;
  content: string;
  origin: string;
  ts: number;
  tokens: number | null;
}

export function insertMessage(
  db: DB,
  convId: string,
  role: "user" | "assistant",
  content: string,
  tokens?: number,
): StoredMessage {
  const msg: StoredMessage = {
    msg_id: randomUUID(),
    conv_id: convId,
    role,
    content,
    origin: "terminal",
    ts: Date.now(),
    tokens: tokens ?? null,
  };
  db.prepare(
    "INSERT INTO messages (msg_id, conv_id, role, content, origin, ts, tokens) VALUES (?, ?, ?, ?, ?, ?, ?)",
  ).run(msg.msg_id, msg.conv_id, msg.role, msg.content, msg.origin, msg.ts, msg.tokens);
  return msg;
}

export function getMessages(db: DB, convId: string): StoredMessage[] {
  return db
    .prepare("SELECT * FROM messages WHERE conv_id = ? ORDER BY ts ASC")
    .all(convId) as StoredMessage[];
}
