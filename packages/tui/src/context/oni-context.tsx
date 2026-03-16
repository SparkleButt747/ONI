import React, {
  createContext,
  useContext,
  useState,
  useCallback,
  type ReactNode,
} from "react";
import type { TaskStatus, AgentRole, SyncStatus } from "../theme.js";

// --- Data types ---

export interface ToolCall {
  timestamp: string;
  tool: string;
  args: string;
  latency: string;
  plugin?: string;
  status?: "ok" | "fail";
}

export interface Task {
  id: string;
  mission: string;
  status: TaskStatus;
  elapsed?: string;
  blocker?: string;
}

export interface DiffData {
  file: string;
  additions: number;
  deletions: number;
  lines: Array<{
    type: "add" | "remove" | "context";
    content: string;
    lineNum?: number;
  }>;
}

export interface Message {
  id: string;
  role: "user" | "oni";
  content: string;
  agent?: AgentRole;
  toolCalls?: ToolCall[];
  diff?: DiffData | null;
}

export type AgentState = "active" | "idle" | "reviewing";
export type ViewState = "boot" | "repl" | "mc";

// --- Context shape ---

export interface ONIState {
  view: ViewState;
  setView: (v: ViewState) => void;
  convId: string;
  model: string;
  version: string;
  tokens: number;
  maxTokens: number;
  burnRate: number;
  syncStatus: SyncStatus;
  agentStates: Record<AgentRole, AgentState>;
  setAgentStates: (s: Record<AgentRole, AgentState>) => void;
  tasks: Task[];
  setTasks: (t: Task[]) => void;
  toolLog: ToolCall[];
  addToolCall: (t: ToolCall) => void;
  messages: Message[];
  addMessage: (m: Message) => void;
  activeDiff: DiffData | null;
  setActiveDiff: (d: DiffData | null) => void;
  setTokens: (n: number) => void;
  setBurnRate: (n: number) => void;
  setSyncStatus: (s: SyncStatus) => void;
}

const ONIContext = createContext<ONIState | null>(null);

export function useONI(): ONIState {
  const ctx = useContext(ONIContext);
  if (!ctx) throw new Error("useONI must be used within ONIProvider");
  return ctx;
}

export function ONIProvider({ children }: { children: ReactNode }) {
  const [view, setView] = useState<ViewState>("boot");
  const [tokens, setTokens] = useState(0);
  const [burnRate, setBurnRate] = useState(0);
  const [syncStatus, setSyncStatus] = useState<SyncStatus>("LOCAL");
  const [agentStates, setAgentStates] = useState<
    Record<AgentRole, AgentState>
  >({
    planner: "idle",
    executor: "idle",
    critic: "idle",
  });
  const [tasks, setTasks] = useState<Task[]>([]);
  const [toolLog, setToolLog] = useState<ToolCall[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [activeDiff, setActiveDiff] = useState<DiffData | null>(null);

  const addToolCall = useCallback(
    (t: ToolCall) => setToolLog((prev) => [...prev, t]),
    [],
  );
  const addMessage = useCallback(
    (m: Message) => setMessages((prev) => [...prev, m]),
    [],
  );

  return (
    <ONIContext.Provider
      value={{
        view,
        setView,
        convId: "conv_8fk2a9",
        model: "claude-sonnet-4-6",
        version: "0.1.0",
        tokens,
        maxTokens: 200_000,
        burnRate,
        syncStatus,
        agentStates,
        setAgentStates,
        tasks,
        setTasks,
        toolLog,
        addToolCall,
        messages,
        addMessage,
        activeDiff,
        setActiveDiff,
        setTokens,
        setBurnRate,
        setSyncStatus,
      }}
    >
      {children}
    </ONIContext.Provider>
  );
}
