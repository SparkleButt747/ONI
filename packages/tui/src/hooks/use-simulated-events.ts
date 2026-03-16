import { useEffect, useRef } from "react";
import { useONI } from "../context/oni-context.js";
import type { DiffData } from "../context/oni-context.js";

export function useSimulatedEvents() {
  const oni = useONI();
  const hasRun = useRef(false);

  useEffect(() => {
    if (hasRun.current) return;
    hasRun.current = true;

    const timers: ReturnType<typeof setTimeout>[] = [];
    const at = (ms: number, fn: () => void) => {
      timers.push(setTimeout(fn, ms));
    };

    // Boot → REPL transition happens via BootView onComplete
    // These events fire after REPL is active

    at(3500, () => {
      oni.setSyncStatus("LIVE");
      oni.setTokens(0);
    });

    // Populate tasks for MC view
    at(3200, () => {
      oni.setTasks([
        {
          id: "a3f82e",
          mission: "Refactor auth middleware",
          status: "RUNNING",
          elapsed: "2m 14s",
        },
        {
          id: "b7c1d4",
          mission: "Write unit tests — UserService",
          status: "RUNNING",
          elapsed: "0m 47s",
        },
        {
          id: "c9d0e1",
          mission: "Generate OpenAPI schema",
          status: "RUNNING",
          elapsed: "0m 08s",
        },
        {
          id: "d4e5f6",
          mission: "Deploy staging — awaiting CI",
          status: "BLOCKED",
          blocker: "requires approval",
        },
        {
          id: "e2f3a4",
          mission: "Lint fix — tsconfig paths",
          status: "ERROR",
        },
        {
          id: "f1a2b3",
          mission: "Scaffold Express router",
          status: "DONE",
          elapsed: "4m 02s",
        },
      ]);
    });

    // Simulate a conversation
    at(4000, () => {
      oni.addMessage({
        id: "m1",
        role: "user",
        content:
          "the order total is wrong for discount codes. look at the pricing engine and fix it",
      });
      oni.setAgentStates({
        planner: "active",
        executor: "idle",
        critic: "idle",
      });
    });

    at(4500, () => {
      oni.addMessage({
        id: "m1-plan",
        role: "oni",
        agent: "planner",
        content:
          "Decomposing · 3 subtasks · tool budget: 8 · no ambiguity — proceeding",
      });
      oni.setAgentStates({
        planner: "idle",
        executor: "active",
        critic: "idle",
      });
    });

    at(5000, () => {
      oni.addToolCall({
        timestamp: "14:22:01",
        tool: "read_file",
        args: "src/services/PricingEngine.ts",
        latency: "9ms",
      });
      oni.setTokens(8400);
    });

    at(5500, () => {
      oni.addToolCall({
        timestamp: "14:22:01",
        tool: "read_file",
        args: "src/services/OrderService.ts:processTotal",
        latency: "7ms",
      });
      oni.setTokens(14200);
    });

    at(6000, () => {
      oni.addToolCall({
        timestamp: "14:22:04",
        tool: "bash",
        args: "npx jest PricingEngine --no-coverage 2>&1 | tail -20",
        latency: "1.4s",
      });
      oni.setTokens(22800);
      oni.setBurnRate(1842);
    });

    at(7000, () => {
      const diff: DiffData = {
        file: "src/services/OrderService.ts",
        additions: 2,
        deletions: 2,
        lines: [
          {
            type: "context",
            content: "  calculateSubtotal(items)",
            lineNum: 44,
          },
          {
            type: "remove",
            content: "  applyDiscount(calculateTax(subtotal), code)",
            lineNum: 45,
          },
          {
            type: "add",
            content: "  const discounted = applyDiscount(subtotal, code)",
            lineNum: 45,
          },
          {
            type: "add",
            content: "  calculateTax(discounted)",
            lineNum: 46,
          },
          { type: "context", content: "  return total", lineNum: 47 },
        ],
      };

      oni.addMessage({
        id: "m2",
        role: "oni",
        agent: "executor",
        content:
          "applyDiscount() runs after calculateTax() — discounts reduce post-tax total instead of pre-tax subtotal. Off-by-tax-rate on every discounted order. Swapping call order in processTotal().",
        diff,
      });
      oni.setActiveDiff(diff);
      oni.setTokens(35400);
      oni.addToolCall({
        timestamp: "14:22:06",
        tool: "write_file",
        args: "src/services/OrderService.ts",
        latency: "5ms",
      });
    });

    at(8500, () => {
      oni.setAgentStates({
        planner: "idle",
        executor: "idle",
        critic: "active",
      });
    });

    at(9200, () => {
      oni.addMessage({
        id: "m3",
        role: "oni",
        agent: "critic",
        content:
          "output accepted · no regressions · tests cover this path · clean.",
      });
      oni.setAgentStates({
        planner: "idle",
        executor: "idle",
        critic: "idle",
      });
      oni.setTokens(47200);
      oni.setBurnRate(1842);
    });

    return () => timers.forEach(clearTimeout);
  }, []);
}
