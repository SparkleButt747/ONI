import { useEffect, useRef } from "react";
import { useONI, type ONIState, type DispatchFn } from "../context/oni-context.js";

/**
 * Factory function type — receives ONI context, returns a dispatch function.
 * This is created in the CLI layer where the agent dependencies live,
 * avoiding circular deps between @oni/tui and @oni/agent.
 */
export type CreateDispatchFn = (oni: ONIState) => DispatchFn;

/**
 * Wires an external dispatch factory into the ONI context.
 * When `createDispatch` is provided, it builds a dispatch function bound to
 * the current ONI state and installs it on the context so that InputPrompt
 * and other components can call `oni.dispatch(msg)`.
 *
 * When `createDispatch` is null (demo mode), the hook is a no-op.
 */
export function useAgent(createDispatch: CreateDispatchFn | null) {
  const oni = useONI();
  const installed = useRef(false);

  useEffect(() => {
    if (!createDispatch || installed.current) return;
    installed.current = true;

    const wrappedDispatch: DispatchFn = async (message: string) => {
      oni.setIsProcessing(true);
      oni.setAgentStates({ planner: "idle", executor: "active", critic: "idle" });
      try {
        await createDispatch(oni)(message);
      } finally {
        oni.setIsProcessing(false);
        oni.setAgentStates({ planner: "idle", executor: "idle", critic: "idle" });
      }
    };

    oni.setDispatch(wrappedDispatch);
  }, [createDispatch, oni]);
}
