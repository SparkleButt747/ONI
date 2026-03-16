import Database from "better-sqlite3";
import { runMigrations } from "./migrations/0001_initial.js";

export type DB = Database.Database;

export function createDatabase(path: string): DB {
  const db = new Database(path);
  db.pragma("journal_mode = WAL");
  db.pragma("foreign_keys = ON");
  runMigrations(db);
  return db;
}

export function closeDatabase(db: DB): void {
  db.close();
}
