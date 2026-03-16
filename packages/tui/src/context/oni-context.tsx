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

export type DispatchFn = (message: string) => Promise<void>;

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
  updateLastMessage: (updater: (prev: Message) => Message) => void;
  activeDiff: DiffData | null;
  setActiveDiff: (d: DiffData | null) => void;
  setTokens: (n: number) => void;
  setBurnRate: (n: number) => void;
  setSyncStatus: (s: SyncStatus) => void;
  dispatch: DispatchFn | null;
  setDispatch: (fn: DispatchFn | null) => void;
  isProcessing: boolean;
  setIsProcessing: (v: boolean) => void;
}

const ONIContext = createContext<ONIState | null>(null);

export function useONI(): ONIState {
  const ctx = useContext(ONIContext);
  if (!ctx) throw new Error("useONI must be used within ONIProvider");
  return ctx;
}

export interface ONIProviderProps {
  children: ReactNode;
  convId?: string;
  model?: string;
}

export function ONIProvider({
  children,
  convId: initialConvId = "conv_8fk2a9",
  model: initialModel = "claude-sonnet-4-6",
}: ONIProviderProps) {
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
  const [dispatch, setDispatch] = useState<DispatchFn | null>(null);
  const [isProcessing, setIsProcessing] = useState(false);

  const addToolCall = useCallback(
    (t: ToolCall) => setToolLog((prev) => [...prev, t]),
    [],
  );
  const addMessage = useCallback(
    (m: Message) => setMessages((prev) => [...prev, m]),
    [],
  );
  const updateLastMessage = useCallback(
    (updater: (prev: Message) => Message) =>
      setMessages((prev) => {
        if (prev.length === 0) return prev;
        const updated = [...prev];
        updated[updated.length - 1] = updater(updated[updated.length - 1]);
        return updated;
      }),
    [],
  );

  return (
    <ONIContext.Provider
      value={{
        view,
        setView,
        convId: initialConvId,
        model: initialModel,
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
        updateLastMessage,
        activeDiff,
        setActiveDiff,
        setTokens,
        setBurnRate,
        setSyncStatus,
        dispatch,
        setDispatch: (fn: DispatchFn | null) => setDispatch(() => fn),
        isProcessing,
        setIsProcessing,
      }}
    >
      {children}
    </ONIContext.Provider>
  );
}
