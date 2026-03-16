import { describe, test, expect } from "vitest";
import { BudgetTracker } from "../src/budget.js";

describe("BudgetTracker", () => {
  test("tracks session tokens", () => {
    const bt = new BudgetTracker({ sessionLimit: 1000 });
    expect(bt.record(500)).toBe(true);
    expect(bt.sessionUsed).toBe(500);
    expect(bt.sessionRemaining).toBe(500);
  });

  test("returns false when session limit exceeded", () => {
    const bt = new BudgetTracker({ sessionLimit: 100 });
    bt.record(80);
    expect(bt.record(30)).toBe(false);
  });

  test("canSpend checks before spending", () => {
    const bt = new BudgetTracker({ sessionLimit: 100 });
    bt.record(80);
    expect(bt.canSpend(30)).toBe(false);
    expect(bt.canSpend(10)).toBe(true);
  });

  test("unlimited when limit is 0", () => {
    const bt = new BudgetTracker({ sessionLimit: 0 });
    expect(bt.record(999999)).toBe(true);
    expect(bt.sessionRemaining).toBeNull();
  });
});
