/**
 * ONI TUI Screenshot Tool
 *
 * Renders the TUI to a string (no TTY needed), then uses freeze to produce a PNG.
 * Usage: npx tsx scripts/take-screenshot.ts [view] [output]
 *   view:   boot | repl | mc (default: repl)
 *   output: path (default: /tmp/oni-tui.png)
 */

import { render } from "ink-testing-library";
import React from "react";
import { writeFileSync } from "fs";
import { execSync } from "child_process";

// We'll render each view directly using ink-testing-library
// which gives us string output without needing a TTY

const view = process.argv[2] || "repl";
const output = process.argv[3] || "/tmp/oni-tui.png";

async function main() {
  // Dynamically import the app after setting env
  const { ONIProvider, useONI } = await import("../src/context/oni-context.js");
  const { color } = await import("../src/theme.js");
  const { BootSequence } = await import("../src/components/BootSequence.js");
  const { REPLView } = await import("../src/views/REPLView.js");
  const { MissionControl } = await import("../src/views/MissionControl.js");
  const { Box, Text } = await import("ink");

  // Build a static snapshot component for each view
  function ScreenshotApp() {
    return React.createElement(ONIProvider, null,
      React.createElement(ScreenshotInner, null)
    );
  }

  function ScreenshotInner() {
    const oni = useONI();

    // Pre-populate state for screenshot
    React.useEffect(() => {
      oni.setSyncStatus("LIVE");
      oni.setTokens(47200);
      oni.setBurnRate(1842);
      oni.setAgentStates({
        planner: "idle",
        executor: "active",
        critic: "idle",
      });
      oni.setTasks([
        { id: "a3f82e", mission: "Refactor auth middleware", status: "RUNNING", elapsed: "2m 14s" },
        { id: "b7c1d4", mission: "Write unit tests — UserService", status: "RUNNING", elapsed: "0m 47s" },
        { id: "c9d0e1", mission: "Generate OpenAPI schema", status: "RUNNING", elapsed: "0m 08s" },
        { id: "d4e5f6", mission: "Deploy staging — awaiting CI", status: "BLOCKED", blocker: "requires approval" },
        { id: "e2f3a4", mission: "Lint fix — tsconfig paths", status: "ERROR" },
        { id: "f1a2b3", mission: "Scaffold Express router", status: "DONE", elapsed: "4m 02s" },
      ]);
      oni.addMessage({ id: "m1", role: "user", content: "the order total is wrong for discount codes" });
      oni.addMessage({ id: "m2", role: "oni", agent: "planner", content: "Decomposing · 3 subtasks · budget: 8 · no ambiguity" });
      oni.addMessage({ id: "m3", role: "oni", agent: "executor", content: "applyDiscount() runs post-tax. Swapping call order in processTotal()." });
      oni.addMessage({ id: "m4", role: "oni", agent: "critic", content: "output accepted · no regressions · tests cover this path · clean." });
      oni.addToolCall({ timestamp: "14:22:01", tool: "read_file", args: "src/services/PricingEngine.ts", latency: "9ms" });
      oni.addToolCall({ timestamp: "14:22:04", tool: "bash", args: "npx jest PricingEngine --no-coverage", latency: "1.4s" });
      oni.setActiveDiff({
        file: "src/services/OrderService.ts",
        additions: 2,
        deletions: 2,
        lines: [
          { type: "context", content: "  calculateSubtotal(items)", lineNum: 44 },
          { type: "remove", content: "  applyDiscount(calculateTax(subtotal), code)", lineNum: 45 },
          { type: "add", content: "  const discounted = applyDiscount(subtotal, code)", lineNum: 45 },
          { type: "add", content: "  calculateTax(discounted)", lineNum: 46 },
        ],
      });
    }, []);

    const width = 90;

    if (view === "boot") {
      return React.createElement(BootSequence, { width, onComplete: () => {} });
    }
    if (view === "mc") {
      return React.createElement(Box, { flexDirection: "column", width },
        React.createElement(Box, null,
          React.createElement(Text, { color: color.amber, bold: true }, "ONI "),
          React.createElement(Text, { color: color.muted }, "ONBOARD NEURAL INTELLIGENCE"),
        ),
        React.createElement(MissionControl, { width }),
      );
    }
    // Default: REPL
    return React.createElement(Box, { flexDirection: "column", width },
      React.createElement(Box, null,
        React.createElement(Text, { color: color.amber, bold: true }, "ONI "),
        React.createElement(Text, { color: color.muted }, "ONBOARD NEURAL INTELLIGENCE"),
      ),
      React.createElement(REPLView, { width }),
    );
  }

  const instance = render(React.createElement(ScreenshotApp));

  // Wait for effects to fire — boot needs longer for staggered init steps
  const waitMs = view === "boot" ? 3000 : 800;
  await new Promise((resolve) => setTimeout(resolve, waitMs));

  const lastFrame = instance.lastFrame();
  instance.unmount();

  if (!lastFrame) {
    console.error("No frame captured");
    process.exit(1);
  }

  // Write ANSI to temp file
  const ansiPath = "/tmp/oni-screenshot.ansi";
  writeFileSync(ansiPath, lastFrame);

  console.log(`Captured ${view} view (${lastFrame.split("\n").length} lines)`);

  // Use freeze to render PNG
  try {
    execSync(
      `cat "${ansiPath}" | freeze --output "${output}" --language ansi --window --theme dracula --padding 20`,
      { stdio: "inherit" },
    );
    console.log(`Screenshot saved to ${output}`);
  } catch {
    console.log(`ANSI output saved to ${ansiPath} (freeze failed — view raw with: cat ${ansiPath})`);
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
