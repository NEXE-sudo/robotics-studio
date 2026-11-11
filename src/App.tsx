import { useState, useEffect, useRef, useCallback } from "react";
import Editor from "@monaco-editor/react";
import { invoke } from "@tauri-apps/api/core";
import RobotView, { RobotControls } from "./components/RobotView";
import { listen, emit } from "@tauri-apps/api/event";
import TFTree from "./components/TFTree";
import RosGraphExplorer from "./components/RosGraphExplorer";
import type { RosGraphSnapshot } from "./components/RosGraphExplorer";
import Settings from "./components/Settings";
import NewProjectModal from "./components/NewProjectModal";
import {
  KeyBinding,
  DEFAULT_KEYBINDINGS,
  parseKeybinding,
  keyEventMatches,
} from "./keybindings";
import {
  X,
  Sparkles,
  Bot,
  Folder,
  FolderOpen,
  FileText,
  Ban,
  Save,
  Hammer,
  RotateCw,
  Play,
  Square,
  Settings2,
  LoaderCircle,
  ChevronDown,
  ChevronUp,
} from "lucide-react";

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
}

interface OpenFile {
  path: string;
  name: string;
  content: string;
  savedContent: string;
}

interface RecentProject {
  path: string;
  name: string;
  last_opened: string;
}

interface RosPackageInfo {
  name: string;
  path: string;
  package_type: string;
  version: string | null;
  description: string | null;
  dependencies: string[];
  executables: string[];
  launch_files: string[];
  config_files: string[];
  metadata_error: string | null;
}

interface LogEvent {
  kind: "append" | "replace";
  text: string;
}

const MAX_RECENT_PROJECTS = 8;
const MAX_LOG_LINES = 1000;
const MAX_LOG_CHARACTERS = 2_000_000;

function trimLog(lines: string[]) {
  const trimmed = lines.slice(-MAX_LOG_LINES);
  let characters = trimmed.reduce((total, line) => total + line.length, 0);
  while (characters > MAX_LOG_CHARACTERS && trimmed.length > 1) {
    characters -= trimmed.shift()?.length ?? 0;
  }
  return trimmed;
}

function appendLog(lines: string[], line: string) {
  return trimLog([...lines, line]);
}

function normalizeRecentProjects(projects: RecentProject[]) {
  const byPath = new Map<string, RecentProject>();

  for (const project of projects) {
    const path = project.path.trim();
    if (!path) continue;

    const existing = byPath.get(path);
    const next = {
      path,
      name: project.name || path.split(/[\\/]/).filter(Boolean).pop() || path,
      last_opened: project.last_opened || String(Date.now()),
    };

    if (!existing || Number(next.last_opened) >= Number(existing.last_opened)) {
      byPath.set(path, next);
    }
  }

  return [...byPath.values()]
    .sort((a, b) => Number(b.last_opened) - Number(a.last_opened))
    .slice(0, MAX_RECENT_PROJECTS);
}

type BottomTab =
  | "ros"
  | "build"
  | "sim"
  | "problems"
  | "tf"
  | "dashboard"
  | "environment";
type ActivityView = "explorer" | "ros" | "search";

const LANG_MAP: Record<string, string> = {
  py: "python",
  ts: "typescript",
  tsx: "typescript",
  js: "javascript",
  jsx: "javascript",
  json: "json",
  yaml: "yaml",
  yml: "yaml",
  xml: "xml",
  launch: "xml",
  urdf: "xml",
  xacro: "xml",
  md: "markdown",
  toml: "toml",
  cpp: "cpp",
  hpp: "cpp",
  h: "cpp",
  c: "c",
  sh: "shell",
  cmake: "cmake",
  txt: "plaintext",
};

function languageFor(name: string) {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  return LANG_MAP[ext] ?? "plaintext";
}

function clamp(n: number, min: number, max: number) {
  return Math.max(min, Math.min(max, n));
}

function FileTree({
  entries,
  depth,
  expandedPaths,
  childrenByPath,
  activeFilePath,
  onToggleFolder,
  onOpenFile,
}: {
  entries: FileEntry[];
  depth: number;
  expandedPaths: Set<string>;
  childrenByPath: Record<string, FileEntry[]>;
  activeFilePath: string | null;
  onToggleFolder: (path: string) => void;
  onOpenFile: (path: string, name: string) => void;
}) {
  return (
    <>
      {entries.map((entry) => {
        const isExpanded = expandedPaths.has(entry.path);
        return (
          <div key={entry.path}>
            <div
              className={`file-row ${activeFilePath === entry.path ? "active" : ""}`}
              style={{
                paddingLeft: 16 + depth * 14,
                display: "flex",
                alignItems: "center",
                gap: 6,
              }}
              onClick={() =>
                entry.is_dir
                  ? onToggleFolder(entry.path)
                  : onOpenFile(entry.path, entry.name)
              }
            >
              {entry.is_dir ? (
                isExpanded ? (
                  <FolderOpen size={16} style={{ flexShrink: 0 }} />
                ) : (
                  <Folder size={16} style={{ flexShrink: 0 }} />
                )
              ) : (
                <FileText size={16} style={{ flexShrink: 0 }} />
              )}
              <span>{entry.name}</span>
            </div>
            {entry.is_dir && isExpanded && childrenByPath[entry.path] && (
              <FileTree
                entries={childrenByPath[entry.path]}
                depth={depth + 1}
                expandedPaths={expandedPaths}
                childrenByPath={childrenByPath}
                activeFilePath={activeFilePath}
                onToggleFolder={onToggleFolder}
                onOpenFile={onOpenFile}
              />
            )}
          </div>
        );
      })}
    </>
  );
}

function App() {
  const [workspaceError, setWorkspaceError] = useState<string | null>(null);
  const [buildError, setBuildError] = useState<string | null>(null);
  // --- File browser state ---
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [currentPath, setCurrentPath] = useState<string | null>(null);

  // --- Multi-tab editor state ---
  const [openFiles, setOpenFiles] = useState<OpenFile[]>([]);
  const [activeFilePath, setActiveFilePath] = useState<string | null>(null);
  const activeFile = openFiles.find((f) => f.path === activeFilePath) ?? null;

  // Auto-initialize ROS environment on mount
  useEffect(() => {
    initializeRosEnvironment();
  }, []);

  useEffect(() => {
    if (!activeFile || !currentPath) {
      setActiveFileIgnored(false);
      return;
    }
    let cancelled = false;
    invoke<boolean>("check_aiignore", {
      workspaceRoot: currentPath,
      filePath: activeFile.path,
    })
      .then((ignored) => {
        if (!cancelled) setActiveFileIgnored(ignored);
      })
      .catch(() => {
        if (!cancelled) setActiveFileIgnored(false);
      });
    return () => {
      cancelled = true;
    };
  }, [activeFile?.path, currentPath]);

  // --- ROS workspace (for build/sim commands) ---
  const [rosWorkspacePath, setRosWorkspacePath] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [newProjectOpen, setNewProjectOpen] = useState(false);
  const [activeFileIgnored, setActiveFileIgnored] = useState(false);
  const [worldPath, setWorldPath] = useState<string | null>(null); // null = bundled default
  const [availableRobots, setAvailableRobots] = useState<string[]>([]);
  const [logFilter, setLogFilter] = useState("");
  const [nodeStatus, setNodeStatus] = useState<
    Record<string, "alive" | "crashed">
  >({});
  const [keymap, setKeymap] = useState<KeyBinding[]>(DEFAULT_KEYBINDINGS);

  const pickCustomWorld = async () => {
    try {
      const selected = await invoke<string | null>("import_world");
      if (selected) setWorldPath(selected);
    } catch (error) {
      console.error("World import failed:", error);
    }
  };

  // --- ROS event log ---
  const [rosEvents, setRosEvents] = useState<string[]>([]);
  const [rosConnected, setRosConnected] = useState(false);
  const [rosPackages, setRosPackages] = useState<RosPackageInfo[]>([]);
  const [rosGraph, setRosGraph] = useState<RosGraphSnapshot | null>(null);
  const [rosPackagesLoading, setRosPackagesLoading] = useState(false);
  const [rosPackagesError, setRosPackagesError] = useState<string | null>(null);
  const [rosPackageFilter, setRosPackageFilter] = useState("");
  const [expandedRosPackage, setExpandedRosPackage] = useState<string | null>(
    null,
  );
  const [rosPackageContentsError, setRosPackageContentsError] = useState<
    string | null
  >(null);
  const rosPackagesRequest = useRef(0);
  const [initializing, setInitializing] = useState(false);
  const [safetyState, setSafetyState] = useState<
    "READY" | "ACTIVE" | "RESET_PENDING" | "FAULT" | "DISCONNECTED"
  >("DISCONNECTED");
  const [safetyError, setSafetyError] = useState<string | null>(null);
  const [safetyPending, setSafetyPending] = useState(false);
  const [eventHealth, setEventHealth] = useState<{
    queueCapacity: number;
    subscriberCount: number;
    lastSequence: number;
    droppedTelemetry: number;
    criticalOverflows: number;
    maxBacklog: number;
    persistenceHealthy: boolean;
    persistenceError: string;
    oldestAvailableSequence: number;
    retainedCriticalEvents: number;
  } | null>(null);
  const [eventRecovery, setEventRecovery] = useState<{
    gapDetected: boolean;
    oldestAvailableSequence: number;
    latestSequence: number;
    snapshotIncluded: boolean;
    retainedCriticalEvents: number;
  } | null>(null);
  const [robotCommandError, setRobotCommandError] = useState<string | null>(
    null,
  );
  const rosWorkspaceReady = useRef(false);
  const authRecoveryStarted = useRef(false);
  const [setupOutput, setSetupOutput] = useState<string[]>([]); // Environment setup logs
  const [currentSetupLine, setCurrentSetupLine] = useState<string>(""); // Current line for toast

  // --- Build output ---
  const [buildOutput, setBuildOutput] = useState<string[]>([]);
  const [building, setBuilding] = useState(false);
  const diagnostics = [
    ...(workspaceError ? [workspaceError] : []),
    ...(buildError ? [buildError] : []),
    ...setupOutput.filter((line) => line.startsWith("❌")),
    ...buildOutput.filter((line) => /error|failed/i.test(line)),
    ...rosEvents.filter((line) => /NodeCrashed|error/i.test(line)),
    ...(eventHealth && !eventHealth.persistenceHealthy
      ? [
          `Critical ROS event persistence failed: ${eventHealth.persistenceError}`,
        ]
      : []),
  ];
  const filteredRosPackages = rosPackages.filter((pkg) => {
    const filter = rosPackageFilter.trim().toLocaleLowerCase();
    return (
      !filter ||
      [
        pkg.name,
        pkg.description ?? "",
        pkg.package_type,
        ...pkg.dependencies,
      ]
        .join(" ")
        .toLocaleLowerCase()
        .includes(filter)
    );
  });

  // --- AI chat ---
  const [chatInput, setChatInput] = useState("");
  const [chatHistory, setChatHistory] = useState<
    { role: string; text: string }[]
  >([]);
  const [aiLoading, setAiLoading] = useState(false);

  // --- Bottom panel tab ---
  const [activeTab, setActiveTab] = useState<BottomTab>("ros");
  const [activityView, setActivityView] = useState<ActivityView>("explorer");

  // ---------- Panel layout (resizable + collapsible) ----------
  const resolveResponsiveDefault = (variable: string, fallback: number) => {
    if (typeof window === "undefined") return fallback;
    const raw = getComputedStyle(document.documentElement)
      .getPropertyValue(variable)
      .trim();
    if (!raw) return fallback;

    const numbers = raw.match(/-?\d*\.?\d+/g)?.map(Number) ?? [];
    if (numbers.length >= 3) {
      const [min, preferred, max] = numbers;
      const preferredValue = raw.includes("vw")
        ? (window.innerWidth * preferred) / 100
        : raw.includes("vh")
          ? (window.innerHeight * preferred) / 100
          : preferred;
      return clamp(preferredValue, min, max);
    }

    const numeric = Number.parseFloat(raw);
    return Number.isFinite(numeric) ? numeric : fallback;
  };

  const [sidebarWidth, setSidebarWidth] = useState(() =>
    resolveResponsiveDefault("--sidebar-default-width", 240),
  );
  const [chatWidth, setChatWidth] = useState(() =>
    Math.max(
      resolveResponsiveDefault("--sidebar-default-width", 240) * 1.25,
      320,
    ),
  );
  const [bottomHeight, setBottomHeight] = useState(() =>
    resolveResponsiveDefault("--bottom-panel-default-height", 220),
  );
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [chatCollapsed, setChatCollapsed] = useState(false);
  const [bottomCollapsed, setBottomCollapsed] = useState(false);

  const dragRef = useRef<null | "sidebar" | "chat" | "bottom">(null);

  const onDragMove = useCallback((e: MouseEvent) => {
    if (!dragRef.current) return;
    if (dragRef.current === "sidebar") {
      setSidebarWidth(clamp(e.clientX - 48, 160, 500));
    } else if (dragRef.current === "chat") {
      setChatWidth(clamp(window.innerWidth - e.clientX, 220, 560));
    } else if (dragRef.current === "bottom") {
      setBottomHeight(clamp(window.innerHeight - e.clientY, 100, 560));
    }
  }, []);

  const onDragEnd = useCallback(() => {
    dragRef.current = null;
    document.body.style.cursor = "";
    window.removeEventListener("mousemove", onDragMove);
    window.removeEventListener("mouseup", onDragEnd);
  }, [onDragMove]);

  const startDrag =
    (which: "sidebar" | "chat" | "bottom") => (e: React.MouseEvent) => {
      e.preventDefault();
      dragRef.current = which;
      document.body.style.cursor =
        which === "bottom" ? "row-resize" : "col-resize";
      window.addEventListener("mousemove", onDragMove);
      window.addEventListener("mouseup", onDragEnd);
    };

  // ---------- File browser actions ----------
  const [childrenByPath, setChildrenByPath] = useState<
    Record<string, FileEntry[]>
  >({});
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);

  const loadFolder = async (path: string): Promise<boolean> => {
    try {
      const result = await invoke<FileEntry[]>("list_dir", { path });
      setCurrentPath(path);
      setEntries(result);
      setChildrenByPath({});
      setExpandedPaths(new Set());

      const projectName = path.split(/[\\/]/).filter(Boolean).pop() ?? path;
      const nextProject: RecentProject = {
        path,
        name: projectName,
        last_opened: String(Date.now()),
      };

      setRecentProjects((prev) => {
        const updated = normalizeRecentProjects([nextProject, ...prev]);
        void invoke("save_recent_projects", { projects: updated }).catch(
          () => undefined,
        );
        return updated;
      });

      return true;
    } catch (error) {
      console.warn("Failed to load folder:", path, error);
      return false;
    }
  };

  useEffect(() => {
    invoke<RecentProject[] | null>("load_recent_projects")
      .then((loaded) => {
        setRecentProjects(normalizeRecentProjects(loaded ?? []));
      })
      .catch(() => {
        setRecentProjects([]);
      });
  }, []);

  const pickFolder = async () => {
    const selected = await invoke<string | null>("pick_directory");
    if (selected) {
      await loadFolder(selected);
    }
  };

  const openCreatedProject = async (projectPath: string) => {
    await loadFolder(projectPath);
  };

  const toggleFolder = async (path: string) => {
    setExpandedPaths((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });

    if (!childrenByPath[path]) {
      const result = await invoke<FileEntry[]>("list_dir", { path });
      setChildrenByPath((prev) => ({ ...prev, [path]: result }));
    }
  };

  const openFile = async (path: string, name: string) => {
    const existing = openFiles.find((f) => f.path === path);
    if (existing) {
      setActiveFilePath(path);
      return;
    }
    const text = await invoke<string>("read_file", { path });
    setOpenFiles((prev) => [
      ...prev,
      { path, name, content: text, savedContent: text },
    ]);
    setActiveFilePath(path);
  };

  const openAiignore = async () => {
    if (!currentPath) return;
    const aiignorePath = `${currentPath}/.aiignore`;
    try {
      await openFile(aiignorePath, ".aiignore");
    } catch {
      // Doesn't exist yet — seed it with a sensible starter template, then open it.
      const starter =
        "# Files listed here are excluded from AI chat/context in Robotics Studio.\n" +
        "# One pattern per line. Supports: *.ext, dirname/, exact/relative/path\n\n" +
        "*.env\n" +
        "secrets.yaml\n" +
        "*.pem\n" +
        "*.key\n";
      await invoke("write_file", { path: aiignorePath, contents: starter });
      await openFile(aiignorePath, ".aiignore");
    }
  };

  const closeTab = (path: string, e?: React.MouseEvent) => {
    e?.stopPropagation();
    setOpenFiles((prev) => {
      const idx = prev.findIndex((f) => f.path === path);
      const next = prev.filter((f) => f.path !== path);
      if (activeFilePath === path) {
        const fallback =
          next[idx] ?? next[idx - 1] ?? next[next.length - 1] ?? null;
        setActiveFilePath(fallback ? fallback.path : null);
      }
      return next;
    });
  };

  const updateActiveContent = (val: string) => {
    if (!activeFilePath) return;
    setOpenFiles((prev) =>
      prev.map((f) => (f.path === activeFilePath ? { ...f, content: val } : f)),
    );
  };

  const saveFile = async () => {
    if (!activeFile) return;
    await invoke("write_file", {
      path: activeFile.path,
      contents: activeFile.content,
    });
    setOpenFiles((prev) =>
      prev.map((f) =>
        f.path === activeFile.path ? { ...f, savedContent: f.content } : f,
      ),
    );
  };

  const saveGeneratedCode = async (code: string, isXml: boolean) => {
    const selected = await invoke<string | null>("pick_save_path", { isXml });
    if (selected) {
      await invoke("write_file", { path: selected, contents: code });
    }
  };

  // ---------- ROS workspace selection ----------
  const refreshRosPackages = useCallback(
    async (allowUninitializedWorkspace = false) => {
      const requestId = ++rosPackagesRequest.current;
      if (!rosWorkspacePath && !allowUninitializedWorkspace) {
        setRosPackagesLoading(false);
        setRosPackages([]);
        setRosPackagesError(null);
        return;
      }
      setRosPackagesLoading(true);
      setRosPackagesError(null);
      try {
        const packages = await invoke<RosPackageInfo[]>("list_ros_packages");
        if (requestId === rosPackagesRequest.current) {
          setRosPackages(packages);
        }
      } catch (error) {
        if (requestId === rosPackagesRequest.current) {
          setRosPackages([]);
          setRosPackagesError(String(error));
        }
      } finally {
        if (requestId === rosPackagesRequest.current) {
          setRosPackagesLoading(false);
        }
      }
    },
    [rosWorkspacePath],
  );

  useEffect(() => {
    void refreshRosPackages();
  }, [refreshRosPackages]);

  const inspectRosPackage = async (path: string) => {
    if (expandedRosPackage === path) {
      setExpandedRosPackage(null);
      setRosPackageContentsError(null);
      return;
    }
    setExpandedRosPackage(path);
    setRosPackageContentsError(null);
    try {
      const contents = await invoke<FileEntry[]>("list_dir", { path });
      setChildrenByPath((previous) => ({ ...previous, [path]: contents }));
    } catch (error) {
      setRosPackageContentsError(String(error));
    }
  };

  const pickRosWorkspace = async () => {
    try {
      const selected = await invoke<string | null>("pick_directory");
      if (selected) {
        const activeWorkspace = await invoke<string>("set_ros_workspace", {
          workspaceRoot: selected,
        });
        setRosWorkspacePath(activeWorkspace);
        rosWorkspaceReady.current = false;
        setRosConnected(false);
        setSafetyState("DISCONNECTED");
        setSafetyError(null);
        setNodeStatus({});
        setAvailableRobots([]);
        setWorldPath(null);
        setEventHealth(null);
        setEventRecovery(null);
        setWorkspaceError(null);
        await initializeRosEnvironment();
      }
    } catch (error) {
      setWorkspaceError(`Could not switch ROS workspace: ${String(error)}`);
    }
  };

  // ---------- Build actions ----------
  const triggerBuild = async () => {
    setBuildError(null);
    setBuildOutput([]);
    setBuilding(true);
    setBottomCollapsed(false);
    setActiveTab("build");
    try {
      await invoke("run_colcon_build_streaming");
      await refreshRosPackages();
    } catch (error) {
      setBuilding(false);
      setBuildOutput((prev) => appendLog(prev, `ERROR: ${String(error)}`));
    }
  };

  const cancelBuild = async () => {
    try {
      await invoke("cancel_colcon_build");
      setBuildError(null);
    } catch (error) {
      setBuildError(`Build cancellation failed: ${String(error)}`);
    }
  };

  const initializeRosEnvironment = async () => {
    setInitializing(true);
    setSetupOutput(["Starting..."]);
    setCurrentSetupLine("Starting...");
    try {
      await invoke("initialize_ros_environment");
      await refreshRosPackages(true);
    } catch (err) {
      setSetupOutput((prev) => appendLog(prev, `❌ ${err}`));
      setCurrentSetupLine(`❌ ${err}`);
      setInitializing(false);
    }
  };

  // ---------- Sim actions ----------
  const startSim = async () => {
    setBottomCollapsed(false);
    setActiveTab("sim");
    try {
      const result = await invoke<string>("start_gazebo_sim", {
        worldPath: worldPath,
      });
      console.log("Simulation started:\n", result);
    } catch (err) {
      console.error("Simulation failed:\n", err);
    }
  };

  const stopSim = async () => {
    try {
      const result = await invoke<string>("stop_gazebo_sim");
      console.log("Simulation stopped:\n", result);
      await emit("clear-robots");
    } catch (err) {
      console.error("Simulation failed:\n", err);
    }
  };

  const resetSim = async () => {
    try {
      const result = await invoke<string>("reset_gazebo_sim");
      console.log("Simulation reset:\n", result);
    } catch (err) {
      console.error("Reset failed:\n", err);
    }
  };

  const sendTwist = async (
    topicName: string,
    linearX: number,
    angularZ: number,
  ) => {
    setRobotCommandError(null);
    try {
      await invoke("publish_twist", {
        command: { topicName, linearX, angularZ },
      });
    } catch (err) {
      setRobotCommandError(String(err));
    }
  };

  const requestEmergencyStop = async (activate: boolean) => {
    if (safetyPending) return;
    if (
      !activate &&
      !window.confirm(
        "Reset the latched emergency stop? Motion will remain stopped until a new command is issued.",
      )
    ) {
      return;
    }
    setSafetyPending(true);
    setSafetyError(null);
    try {
      const state = await invoke<{
        state: "READY" | "ACTIVE" | "RESET_PENDING" | "FAULT" | "DISCONNECTED";
        detail: string;
      }>("set_emergency_stop", { activate });
      setSafetyState(state.state);
      if (state.state !== (activate ? "ACTIVE" : "READY")) {
        setSafetyError(
          state.detail || "Backend did not confirm the requested safety state.",
        );
      }
    } catch (error) {
      if (activate) {
        setSafetyState("DISCONNECTED");
      }
      setSafetyError(
        `Emergency-stop request was not confirmed: ${String(error)}`,
      );
    } finally {
      setSafetyPending(false);
    }
  };

  // ---------- AI chat ----------
  const sendChatMessage = async (
    overrideMessage?: string,
    mode?: string,
    tfContext?: string,
  ) => {
    const question = overrideMessage ?? chatInput;
    if (!question.trim()) return;
    setChatHistory((prev) => [...prev, { role: "user", text: question }]);
    setChatInput("");
    setAiLoading(true);

    try {
      const response = await invoke<string>("ask_ai", {
        input: {
          userMessage: question,
          openFileContent: activeFile?.content || null,
          openFilePath: activeFile?.path || null,
          workspaceRoot: currentPath ?? null,
          recentRosEvents: rosEvents,
          recentBuildOutput: buildOutput,
          tfContext: tfContext ?? null,
          mode: mode ?? null,
        },
      });
      setChatHistory((prev) => [
        ...prev,
        { role: "assistant", text: response },
      ]);
    } catch (err) {
      const msg = String(err).includes("No AI provider configured")
        ? "No AI provider set up yet. Click ⚙️ Settings above to add your API key."
        : `Error: ${err}`;
      setChatHistory((prev) => [...prev, { role: "assistant", text: msg }]);
    } finally {
      setAiLoading(false);
    }
  };

  const quickAction = (mode: string, prompt: string) => {
    sendChatMessage(prompt, mode);
  };

  const explainTfFrame = (tf: {
    parent_frame: string;
    child_frame: string;
    x: number;
    y: number;
    z: number;
    qx: number;
    qy: number;
    qz: number;
    qw: number;
    validationIssues?: string[];
  }) => {
    const tfContext =
      `parent_frame: ${tf.parent_frame}\n` +
      `child_frame: ${tf.child_frame}\n` +
      `translation: x=${tf.x.toFixed(4)}, y=${tf.y.toFixed(4)}, z=${tf.z.toFixed(4)}\n` +
      `rotation (quaternion): x=${tf.qx.toFixed(4)}, y=${tf.qy.toFixed(4)}, ` +
      `z=${tf.qz.toFixed(4)}, w=${tf.qw.toFixed(4)}`;

    const validationContext =
      tf.validationIssues && tf.validationIssues.length > 0
        ? `\n\nTF validation warning: ${tf.validationIssues.join("; ")}. ` +
          "This transform failed validation and may be malformed, uninitialized, or numerically unstable. " +
          "Explain the likely issue and how it could affect the TF tree, rather than treating it as a normal valid transform."
        : "";

    setChatCollapsed(false);
    sendChatMessage(
      `Explain the TF frame relationship "${tf.parent_frame} → ${tf.child_frame}".`,
      "explain_tf",
      `${tfContext}${validationContext}`,
    );
  };

  // ---------- Keyboard shortcuts ----------
  // Load keybindings on mount
  useEffect(() => {
    invoke<KeyBinding[] | null>("load_keybindings").then((loaded) => {
      if (loaded) {
        setKeymap(loaded);
      }
    });
  }, []);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      // Find matching keybinding
      for (const binding of keymap) {
        const parsed = parseKeybinding(binding.keys);
        if (keyEventMatches(e, parsed)) {
          e.preventDefault();
          // Dispatch to appropriate action
          switch (binding.id) {
            case "save":
              saveFile();
              break;
            case "toggle-sidebar":
              setSidebarCollapsed((c) => !c);
              break;
            case "toggle-bottom-panel":
              setBottomCollapsed((c) => !c);
              break;
            case "close-tab":
              if (activeFilePath) {
                closeTab(activeFilePath);
              }
              break;
            case "build":
              triggerBuild();
              break;
            case "new-project":
              setNewProjectOpen(true);
              break;
          }
          return;
        }
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [keymap, activeFile, activeFilePath]);

  // ---------- Event listeners (ROS stream + build stream) ----------
  useEffect(() => {
    invoke("start_ros_stream");

    const unlistenEvent = listen<string>("ros-event", (event) => {
      setRosEvents((prev) => [...prev.slice(-49), event.payload]);
      setRosConnected(true);
      authRecoveryStarted.current = false;

      // Extract any cmd_vel topics from TopicSnapshot events, so the
      // robot control dropdown reflects whatever's actually in the
      // currently-loaded world, not a hardcoded pair.
      const cmdVelMatches = [
        ...event.payload.matchAll(/name: "([^"]+\/cmd_vel)"/g),
      ];
      if (cmdVelMatches.length > 0) {
        const topics = cmdVelMatches.map((m) => m[1]);
        setAvailableRobots(topics);
      }
    });

    const unlistenNodeSnapshot = listen<string[]>("node-snapshot", (event) => {
      const currentNodes = new Set(event.payload);
      setNodeStatus((prev) => {
        const next: Record<string, "alive" | "crashed"> = {};
        // Mark everything currently reported as alive
        currentNodes.forEach((name) => {
          next[name] = "alive";
        });
        // Anything we previously knew about that's no longer present: crashed
        Object.keys(prev).forEach((name) => {
          if (!currentNodes.has(name)) next[name] = "crashed";
        });
        return next;
      });
    });

    const unlistenError = listen<string>("ros-error", (event) => {
      console.error("ROS error:", event.payload);
      setRosConnected(false);
      setRosGraph(null);
      setSafetyState((state) =>
        state === "ACTIVE" || state === "FAULT" ? state : "DISCONNECTED",
      );
    });

    const unlistenAuthError = listen<string>("ros-auth-error", (event) => {
      setRosConnected(false);
      setRosGraph(null);
      setSafetyState((state) =>
        state === "ACTIVE" || state === "FAULT" ? state : "DISCONNECTED",
      );
      setRosEvents((prev) => [
        ...prev.slice(-49),
        `Authentication: ${event.payload}`,
      ]);
      if (rosWorkspaceReady.current && !authRecoveryStarted.current) {
        authRecoveryStarted.current = true;
        void initializeRosEnvironment();
      }
    });

    const unlistenConnecting = listen<void>("ros-connecting", () => {
      setRosConnected(false);
      setRosGraph(null);
      setSafetyState((state) =>
        state === "ACTIVE" || state === "FAULT" ? state : "DISCONNECTED",
      );
    });
    const unlistenSafety = listen<{
      state: "READY" | "ACTIVE" | "RESET_PENDING" | "FAULT" | "DISCONNECTED";
      detail: string;
    }>("safety-state", (event) => {
      setSafetyState(event.payload.state);
      setSafetyError(
        event.payload.state === "FAULT" ? event.payload.detail : null,
      );
    });
    const unlistenEventHealth = listen<{
      queueCapacity: number;
      subscriberCount: number;
      lastSequence: number;
      droppedTelemetry: number;
      criticalOverflows: number;
      maxBacklog: number;
      persistenceHealthy: boolean;
      persistenceError: string;
      oldestAvailableSequence: number;
      retainedCriticalEvents: number;
    }>("event-health", (event) => {
      setEventHealth(event.payload);
    });
    const unlistenEventRecovery = listen<{
      gapDetected: boolean;
      oldestAvailableSequence: number;
      latestSequence: number;
      snapshotIncluded: boolean;
      retainedCriticalEvents: number;
    }>("event-recovery", (event) => {
      setEventRecovery(event.payload);
    });
    const unlistenRosGraph = listen<RosGraphSnapshot>(
      "ros-graph-snapshot",
      (event) => setRosGraph(event.payload),
    );

    const unlistenBuildOutput = listen<LogEvent>("build-output", (event) => {
      const { kind, text } = event.payload;
      if (kind === "replace") {
        // Update the last line in place
        setBuildOutput((prev) => {
          const updated = [...prev];
          if (updated.length > 0) {
            updated[updated.length - 1] = text;
          } else {
            updated.push(text);
          }
          return updated;
        });
      } else {
        // Append a new line
        setBuildOutput((prev) => appendLog(prev, text));
      }
    });
    const unlistenBuildError = listen<string>("build-error", (event) => {
      setBuildOutput((prev) => appendLog(prev, `ERROR: ${event.payload}`));
    });
    const unlistenBuildFinished = listen<boolean>("build-finished", (event) => {
      setBuilding(false);
      setBuildOutput((prev) =>
        appendLog(
          prev,
          event.payload ? "[✓ Build succeeded]" : "[✗ Build failed]",
        ),
      );
    });

    return () => {
      unlistenEvent.then((f) => f());
      unlistenError.then((f) => f());
      unlistenAuthError.then((f) => f());
      unlistenNodeSnapshot.then((f) => f());
      unlistenConnecting.then((f) => f());
      unlistenSafety.then((f) => f());
      unlistenEventHealth.then((f) => f());
      unlistenEventRecovery.then((f) => f());
      unlistenRosGraph.then((f) => f());
      unlistenBuildOutput.then((f) => f());
      unlistenBuildError.then((f) => f());
      unlistenBuildFinished.then((f) => f());
    };
  }, []);

  useEffect(() => {
    const unlistenProgress = listen<LogEvent>("init-progress", (event) => {
      const { kind, text } = event.payload;
      if (kind === "replace") {
        // Update the last line in place
        setSetupOutput((prev) => {
          const updated = [...prev];
          if (updated.length > 0) {
            updated[updated.length - 1] = text;
          } else {
            updated.push(text);
          }
          return updated;
        });
      } else {
        // Append a new line
        setSetupOutput((prev) => appendLog(prev, text));
      }
      // Always update the toast's current line
      setCurrentSetupLine(text);
    });
    const unlistenFinished = listen<string>("init-finished", (event) => {
      rosWorkspaceReady.current = true;
      setRosWorkspacePath(event.payload);
      setSetupOutput((prev) => appendLog(prev, "✅ ROS environment ready"));
      setCurrentSetupLine("✅ ROS environment ready");
      setInitializing(false);
      void invoke("start_ros_stream").catch((error) => {
        setRosEvents((prev) => [
          ...prev.slice(-49),
          `ROS stream failed: ${error}`,
        ]);
      });
    });
    const unlistenError = listen<string>("init-error", (event) => {
      setSetupOutput((prev) => appendLog(prev, `❌ ${event.payload}`));
      setCurrentSetupLine(`❌ ${event.payload}`);
      setInitializing(false);
    });

    return () => {
      unlistenProgress.then((f) => f());
      unlistenFinished.then((f) => f());
      unlistenError.then((f) => f());
    };
  }, []);

  const dirtyCount = openFiles.filter(
    (f) => f.content !== f.savedContent,
  ).length;

  // ---------- Layout ----------
  return (
    <div
      style={{
        height: "100vh",
        width: "100vw",
        display: "flex",
        flexDirection: "column",
        fontFamily:
          "-apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif",
        background: "#1e1e1e",
        color: "#ccc",
        overflow: "hidden",
      }}
    >
      <style>{`
        .ide-btn {
          background: #2d2d30; color: #ddd; border: 1px solid #3c3c3c;
          border-radius: 4px; padding: 4px 10px; font-size: var(--font-sm); cursor: pointer;
          transition: background 0.12s, border-color 0.12s;
        }
        .ide-btn:hover { background: #3a3a3d; border-color: #4fc3f7; }
        .ide-btn:disabled { opacity: 0.4; cursor: default; }
        .ide-btn:disabled:hover { background: #2d2d30; border-color: #3c3c3c; }
        .ide-btn.primary { border-color: #4fc3f7; color: #4fc3f7; }
        .ide-btn.danger:hover { border-color: #e08080; color: #e08080; }
        .activity-icon {
          width: var(--activity-bar-width); height: auto; aspect-ratio: 1;
          display: flex; align-items: center; justify-content: center; 
          font-size: var(--activity-icon-size); cursor: pointer;
          color: #858585; border-left: 2px solid transparent; position: relative;
        }
        .activity-icon:hover { color: #fff; }
        .activity-icon.active { color: #fff; border-left: 2px solid #4fc3f7; background: #1e1e1e; }
        .file-row {
          padding: var(--spacing-xs) var(--spacing-base) var(--spacing-xs) var(--spacing-lg);
          font-size: var(--font-md); cursor: pointer;
          display: flex; align-items: center; gap: var(--spacing-sm); white-space: nowrap;
          border-left: 2px solid transparent;
        }
        .file-row:hover { background: #2a2d2e; }
        .file-row.active { background: #37373d; border-left: 2px solid #4fc3f7; }
        .tab {
          display: flex; align-items: center; gap: var(--spacing-sm);
          padding: 0 var(--spacing-md) 0 var(--spacing-lg);
          height: var(--tab-height); font-size: var(--tab-font-size); cursor: pointer;
          white-space: nowrap; border-right: 1px solid #1e1e1e;
          color: #969696; position: relative; flex-shrink: 0;
        }
        .tab.active { background: #1e1e1e; color: #fff; }
        .tab:hover { color: #fff; }
        .tab-close {
          width: var(--icon-sm); height: var(--icon-sm); border-radius: 3px; 
          display: flex; align-items: center; justify-content: center; 
          opacity: 0.6; flex-shrink: 0;
        }
        .tab-close:hover { background: #4a4a4d; opacity: 1; }
        .resizer-v { width: var(--resizer-size); cursor: col-resize; flex-shrink: 0; background: transparent; }
        .resizer-v:hover, .resizer-v:active { background: #4fc3f7; }
        .resizer-h { height: var(--resizer-size); cursor: row-resize; flex-shrink: 0; background: transparent; }
        .resizer-h:hover, .resizer-h:active { background: #4fc3f7; }
        .bottom-tab {
          padding: var(--spacing-sm) var(--spacing-md); font-size: var(--font-sm);
          cursor: pointer; user-select: none; border-right: 1px solid #333;
          display: flex; align-items: center; gap: var(--spacing-sm);
        }
        .bottom-tab.active { background: #1e1e1e; color: #fff; }
        .bottom-tab:not(.active) { color: #888; }
        .bottom-tab:not(.active):hover { color: #ccc; }
        .status-item { display: flex; align-items: center; gap: 4px; padding: 0 8px; height: 100%; }
        .status-item.clickable { cursor: pointer; }
        .status-item.clickable:hover { background: rgba(255,255,255,0.12); }
      `}</style>

      {/* Top toolbar */}
      {settingsOpen && (
        <Settings
          onClose={() => setSettingsOpen(false)}
          onKeybindingsChange={setKeymap}
        />
      )}
      {newProjectOpen && (
        <NewProjectModal
          onClose={() => setNewProjectOpen(false)}
          onProjectCreated={openCreatedProject}
        />
      )}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "6px 10px",
          background: "#252526",
          borderBottom: "1px solid #000",
          flexShrink: 0,
        }}
      >
        <span
          style={{
            fontSize: 11,
            padding: "2px 8px",
            borderRadius: 3,
            background: rosConnected ? "#1b3a1b" : "#3a1b1b",
            color: rosConnected ? "#7cd992" : "#e08080",
            flexShrink: 0,
          }}
        >
          {rosConnected ? "● ROS Connected" : "○ ROS Disconnected"}
        </span>
        <button className="ide-btn" onClick={pickFolder}>
          <Folder size={14} style={{ marginRight: 4 }} /> Open Folder
        </button>
        <button className="ide-btn" onClick={() => setNewProjectOpen(true)}>
          <Sparkles size={14} style={{ marginRight: 4 }} /> New Project from
          Template
        </button>
        <button
          className="ide-btn"
          onClick={openAiignore}
          disabled={!currentPath}
          title={
            currentPath
              ? "Open or create .aiignore for this workspace"
              : "Open a folder first"
          }
        >
          <Ban size={14} style={{ marginRight: 4 }} /> .aiignore
        </button>
        <button className="ide-btn" onClick={pickRosWorkspace}>
          <Bot size={14} style={{ marginRight: 4 }} />{" "}
          {rosWorkspacePath ? "Workspace Set" : "Select ROS Workspace"}
        </button>
        <button
          className="ide-btn primary"
          onClick={triggerBuild}
          disabled={building}
        >
          {building ? (
            <>
              <Hammer size={14} style={{ marginRight: 4, display: "inline" }} />{" "}
              Building…
            </>
          ) : (
            <>
              <Hammer size={14} style={{ marginRight: 4, display: "inline" }} />{" "}
              Build
            </>
          )}
        </button>
        {building && (
          <button className="ide-btn" onClick={cancelBuild}>
            Cancel Build
          </button>
        )}
        <select
          value={worldPath ?? "default"}
          onChange={(e) => {
            if (e.target.value === "custom") {
              pickCustomWorld();
            } else {
              setWorldPath(null);
            }
          }}
          className="ide-btn"
        >
          <option value="default">Diff Drive Demo (default)</option>
          <option value="custom">Custom World...</option>
        </select>
        <button className="ide-btn" onClick={startSim}>
          <Play size={14} style={{ marginRight: 4 }} /> Start Sim
        </button>
        <button className="ide-btn danger" onClick={stopSim}>
          <Square size={14} style={{ marginRight: 4 }} /> Stop Sim
        </button>
        <button className="ide-btn" onClick={resetSim}>
          <RotateCw size={14} style={{ marginRight: 4 }} /> Reset Sim
        </button>
        <div style={{ flex: 1 }} />
        <button
          className="ide-btn"
          onClick={saveFile}
          disabled={
            !activeFile || activeFile.content === activeFile.savedContent
          }
        >
          <Save size={14} style={{ marginRight: 4 }} /> Save
        </button>
      </div>

      {/* Main area: activity bar + sidebar + editor + AI chat */}
      <div style={{ flex: 1, display: "flex", minHeight: 0 }}>
        {/* Activity bar */}
        <div
          style={{
            width: "var(--activity-bar-width)",
            background: "#252526",
            borderRight: "1px solid #000",
            display: "flex",
            flexDirection: "column",
            flexShrink: 0,
          }}
        >
          <div
            className={`activity-icon ${
              activityView === "explorer" && !sidebarCollapsed ? "active" : ""
            }`}
            title="Explorer (⌘B)"
            onClick={() => {
              if (activityView === "explorer") setSidebarCollapsed((c) => !c);
              setActivityView("explorer");
              setSidebarCollapsed(false);
            }}
          >
            <Folder
              size={18}
              style={{ width: "var(--icon-md)", height: "var(--icon-md)" }}
            />
          </div>
          <div className="activity-icon" title="Search">
            <FolderOpen
              size={18}
              style={{ width: "var(--icon-md)", height: "var(--icon-md)" }}
            />
          </div>
          <div
            className={`activity-icon ${
              activityView === "ros" && !sidebarCollapsed ? "active" : ""
            }`}
            title="ROS Nodes"
            onClick={() => {
              if (activityView === "ros") setSidebarCollapsed((c) => !c);
              setActivityView("ros");
              setSidebarCollapsed(false);
            }}
          >
            <Bot
              size={18}
              style={{ width: "var(--icon-md)", height: "var(--icon-md)" }}
            />
          </div>
          <div style={{ flex: 1 }} />
          <div
            className={`activity-icon ${!chatCollapsed ? "active" : ""}`}
            title="AI Assistant"
            onClick={() => setChatCollapsed((c) => !c)}
          >
            <Sparkles
              size={18}
              style={{ width: "var(--icon-md)", height: "var(--icon-md)" }}
            />
          </div>
        </div>

        {/* Sidebar */}
        {!sidebarCollapsed && (
          <>
            <div
              style={{
                width: sidebarWidth,
                background: "#252526",
                color: "#ccc",
                overflowY: "auto",
                flexShrink: 0,
              }}
            >
              <div
                style={{
                  padding: "8px 14px 4px",
                  fontSize: 11,
                  opacity: 0.6,
                  textTransform: "uppercase",
                  letterSpacing: 0.5,
                  fontWeight: 600,
                }}
              >
                {activityView === "explorer" ? "Explorer" : "ROS Graph"}
              </div>

              {activityView === "explorer" && (
                <>
                  {currentPath ? (
                    <div
                      style={{
                        padding: "2px 14px 8px",
                        fontSize: 11,
                        opacity: 0.5,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                      }}
                      title={currentPath}
                    >
                      {currentPath}
                    </div>
                  ) : (
                    <div
                      style={{
                        padding: "8px 14px",
                        fontSize: 12,
                        opacity: 0.5,
                      }}
                    >
                      <div>No folder open. Use Open Folder above.</div>
                      {recentProjects.length > 0 && (
                        <div style={{ marginTop: 12 }}>
                          <div
                            style={{
                              fontSize: 11,
                              textTransform: "uppercase",
                              letterSpacing: 0.5,
                              opacity: 0.7,
                              marginBottom: 6,
                            }}
                          >
                            Recent Projects
                          </div>
                          {recentProjects.map((project) => (
                            <div
                              key={project.path}
                              onClick={async () => {
                                const ok = await loadFolder(project.path);
                                if (!ok) {
                                  setRecentProjects((prev) => {
                                    const next = prev.filter(
                                      (item) => item.path !== project.path,
                                    );
                                    void invoke("save_recent_projects", {
                                      projects: next,
                                    }).catch(() => undefined);
                                    return next;
                                  });
                                }
                              }}
                              title={project.path}
                              style={{
                                padding: "6px 8px",
                                borderRadius: 4,
                                cursor: "pointer",
                                marginBottom: 4,
                                background: "rgba(255,255,255,0.02)",
                              }}
                              onMouseEnter={(e) => {
                                e.currentTarget.style.background =
                                  "rgba(79, 195, 247, 0.08)";
                              }}
                              onMouseLeave={(e) => {
                                e.currentTarget.style.background =
                                  "rgba(255,255,255,0.02)";
                              }}
                            >
                              <div style={{ color: "#eee", fontSize: 12 }}>
                                {project.name}
                              </div>
                              <div
                                style={{
                                  color: "#ccc",
                                  fontSize: 10,
                                  overflow: "hidden",
                                  textOverflow: "ellipsis",
                                  whiteSpace: "nowrap",
                                  marginTop: 2,
                                }}
                              >
                                {project.path}
                              </div>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  )}
                  <FileTree
                    entries={entries}
                    depth={0}
                    expandedPaths={expandedPaths}
                    childrenByPath={childrenByPath}
                    activeFilePath={activeFilePath}
                    onToggleFolder={toggleFolder}
                    onOpenFile={openFile}
                  />
                </>
              )}

              {activityView === "ros" && (
                <div style={{ padding: "4px 14px", fontSize: 12 }}>
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 8,
                      marginBottom: 8,
                    }}
                  >
                    <div style={{ opacity: 0.7, flex: 1, minWidth: 0 }}>
                      Workspace:{" "}
                      <span
                        title={rosWorkspacePath ?? undefined}
                        style={{
                          color: rosWorkspacePath ? "#7cd992" : "#e08080",
                          overflowWrap: "anywhere",
                        }}
                      >
                        {rosWorkspacePath ? "selected" : "not set"}
                      </span>
                    </div>
                    <button
                      className="ide-btn"
                      type="button"
                      onClick={() => void refreshRosPackages()}
                      disabled={rosPackagesLoading || !rosWorkspacePath}
                      aria-label="Refresh ROS packages"
                      title="Refresh ROS packages"
                    >
                      <RotateCw
                        size={13}
                        style={{
                          animation: rosPackagesLoading
                            ? "spin 1s linear infinite"
                            : undefined,
                        }}
                      />
                    </button>
                  </div>

                  {rosPackagesLoading && (
                    <div
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: 8,
                        opacity: 0.7,
                      }}
                    >
                      <LoaderCircle size={14} className="spin" />
                      Loading packages...
                    </div>
                  )}

                  {!rosPackagesLoading && rosPackagesError && (
                    <div style={{ color: "#f0a3a3", whiteSpace: "pre-wrap" }}>
                      {rosPackagesError}
                    </div>
                  )}

                  {!rosPackagesLoading &&
                    !rosPackagesError &&
                    rosPackages.length === 0 && (
                      <div style={{ opacity: 0.6 }}>
                        No packages found in the selected workspace. Build the
                        workspace after adding packages, then refresh.
                      </div>
                    )}

                  {!rosPackagesLoading &&
                    !rosPackagesError &&
                    rosPackages.length > 0 && (
                      <>
                        <input
                          type="search"
                          aria-label="Filter ROS packages"
                          value={rosPackageFilter}
                          onChange={(event) =>
                            setRosPackageFilter(event.target.value)
                          }
                          placeholder="Filter packages..."
                          style={{
                            boxSizing: "border-box",
                            width: "100%",
                            marginBottom: 8,
                            padding: "6px 8px",
                            color: "#ddd",
                            background: "#1e1e1e",
                            border: "1px solid #444",
                            borderRadius: 4,
                          }}
                        />
                        {filteredRosPackages.length === 0 ? (
                          <div style={{ opacity: 0.6 }}>
                            No packages match this filter.
                          </div>
                        ) : (
                          <div
                            style={{
                              display: "flex",
                              flexDirection: "column",
                              gap: 8,
                            }}
                          >
                            {filteredRosPackages.map((pkg) => (
                          <div
                            key={pkg.name}
                            style={{
                              background: "rgba(255,255,255,0.02)",
                              border: "1px solid rgba(255,255,255,0.08)",
                              borderRadius: 6,
                              padding: 8,
                            }}
                          >
                            <div
                              style={{
                                fontWeight: 600,
                                color: "#e8e8e8",
                                marginBottom: 4,
                              }}
                            >
                              {pkg.name}
                            </div>
                            {pkg.description && (
                              <div
                                style={{
                                  opacity: 0.75,
                                  marginBottom: 4,
                                  overflowWrap: "anywhere",
                                }}
                              >
                                {pkg.description}
                              </div>
                            )}
                            <div
                              style={{
                                opacity: 0.7,
                                wordBreak: "break-word",
                                marginBottom: 4,
                              }}
                            >
                              {pkg.path}
                            </div>
                            <div
                              style={{
                                display: "flex",
                                flexWrap: "wrap",
                                gap: 6,
                              }}
                            >
                              {pkg.package_type && (
                                <span
                                  style={{
                                    background: "rgba(79,195,247,0.12)",
                                    borderRadius: 999,
                                    padding: "2px 6px",
                                  }}
                                >
                                  {pkg.package_type}
                                </span>
                              )}
                              {pkg.version && (
                                <span
                                  style={{
                                    background: "rgba(124,217,146,0.12)",
                                    borderRadius: 999,
                                    padding: "2px 6px",
                                  }}
                                >
                                  v{pkg.version}
                                </span>
                              )}
                            </div>
                            {pkg.metadata_error && (
                              <div
                                role="status"
                                style={{
                                  color: "#e0b060",
                                  marginTop: 6,
                                  overflowWrap: "anywhere",
                                }}
                              >
                                Package metadata is incomplete:{" "}
                                {pkg.metadata_error}
                              </div>
                            )}
                            {pkg.dependencies.length > 0 && (
                              <div style={{ marginTop: 6 }}>
                                <div style={{ opacity: 0.6, marginBottom: 2 }}>
                                  Dependencies
                                </div>
                                <div
                                  style={{
                                    display: "flex",
                                    flexWrap: "wrap",
                                    gap: 4,
                                  }}
                                >
                                  {pkg.dependencies.slice(0, 8).map((dep) => (
                                    <span
                                      key={`${pkg.name}-${dep}`}
                                      style={{
                                        background: "rgba(255,255,255,0.04)",
                                        borderRadius: 999,
                                        padding: "2px 6px",
                                      }}
                                    >
                                      {dep}
                                    </span>
                                  ))}
                                </div>
                              </div>
                            )}
                            {(pkg.executables.length > 0 ||
                              pkg.launch_files.length > 0 ||
                              pkg.config_files.length > 0) && (
                              <div
                                style={{
                                  marginTop: 6,
                                  display: "flex",
                                  flexDirection: "column",
                                  gap: 2,
                                }}
                              >
                                {pkg.executables.length > 0 && (
                                  <div style={{ opacity: 0.6 }}>
                                    Executables: {pkg.executables.join(", ")}
                                  </div>
                                )}
                                {pkg.launch_files.length > 0 && (
                                  <div style={{ opacity: 0.6 }}>
                                    Launch: {pkg.launch_files.join(", ")}
                                  </div>
                                )}
                                {pkg.config_files.length > 0 && (
                                  <div style={{ opacity: 0.6 }}>
                                    Config: {pkg.config_files.join(", ")}
                                  </div>
                                )}
                              </div>
                            )}
                            <button
                              className="ide-btn"
                              type="button"
                              aria-expanded={expandedRosPackage === pkg.path}
                              onClick={() => void inspectRosPackage(pkg.path)}
                              style={{ marginTop: 8 }}
                            >
                              {expandedRosPackage === pkg.path
                                ? "Hide contents"
                                : "Inspect contents"}
                            </button>
                            {expandedRosPackage === pkg.path &&
                              rosPackageContentsError && (
                                <div
                                  role="alert"
                                  style={{
                                    color: "#f0a3a3",
                                    marginTop: 6,
                                    overflowWrap: "anywhere",
                                  }}
                                >
                                  Could not inspect package:{" "}
                                  {rosPackageContentsError}
                                </div>
                              )}
                            {expandedRosPackage === pkg.path &&
                              childrenByPath[pkg.path] && (
                                <div style={{ marginTop: 6 }}>
                                  <FileTree
                                    entries={childrenByPath[pkg.path]}
                                    depth={0}
                                    expandedPaths={expandedPaths}
                                    childrenByPath={childrenByPath}
                                    activeFilePath={activeFilePath}
                                    onToggleFolder={toggleFolder}
                                    onOpenFile={(path, name) => {
                                      void openFile(path, name).catch(
                                        (error) =>
                                          setRosPackageContentsError(
                                            String(error),
                                          ),
                                      );
                                    }}
                                  />
                                </div>
                              )}
                          </div>
                            ))}
                          </div>
                        )}
                      </>
                    )}
                  <RosGraphExplorer graph={rosGraph} connected={rosConnected} />
                </div>
              )}
            </div>
            <div className="resizer-v" onMouseDown={startDrag("sidebar")} />
          </>
        )}

        {/* Editor column */}
        <div
          style={{
            flex: 1,
            display: "flex",
            flexDirection: "column",
            minWidth: 0,
          }}
        >
          {/* Tab bar */}
          <div
            style={{
              display: "flex",
              background: "#252526",
              overflowX: "auto",
              flexShrink: 0,
              borderBottom: "1px solid #000",
            }}
          >
            {openFiles.map((f) => {
              const dirty = f.content !== f.savedContent;
              return (
                <div
                  key={f.path}
                  className={`tab ${activeFilePath === f.path ? "active" : ""}`}
                  onClick={() => setActiveFilePath(f.path)}
                  title={f.path}
                >
                  <FileText size={14} style={{ flexShrink: 0 }} />
                  <span>{f.name}</span>
                  <div
                    className="tab-close"
                    onClick={(e) => closeTab(f.path, e)}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      cursor: "pointer",
                    }}
                  >
                    {dirty ? (
                      <span style={{ fontSize: 8, opacity: 0.8 }}>●</span>
                    ) : (
                      <X size={12} />
                    )}
                  </div>
                </div>
              );
            })}
            {openFiles.length === 0 && (
              <div
                style={{
                  padding: "var(--spacing-base) var(--spacing-lg)",
                  fontSize: "var(--font-sm)",
                  opacity: 0.4,
                }}
              >
                No files open
              </div>
            )}
          </div>

          {/* Editor */}
          <div style={{ flex: 1, minHeight: 0 }}>
            {activeFile ? (
              <Editor
                height="100%"
                path={activeFile.path}
                language={languageFor(activeFile.name)}
                value={activeFile.content}
                onChange={(val) => updateActiveContent(val ?? "")}
                theme="vs-dark"
                options={{
                  minimap: { enabled: true },
                  fontSize: 13,
                  fontFamily:
                    "'Cascadia Code', 'Fira Code', ui-monospace, Consolas, monospace",
                  automaticLayout: true,
                  scrollBeyondLastLine: false,
                }}
              />
            ) : (
              <div
                style={{
                  height: "100%",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  color: "#555",
                  fontSize: 13,
                  flexDirection: "column",
                  gap: 8,
                }}
              >
                <div
                  style={{
                    opacity: 0.5,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                  }}
                >
                  <Bot size={32} />
                </div>
                <div>Select a file from the Explorer to start editing</div>
              </div>
            )}
          </div>
        </div>

        {/* AI chat sidebar */}
        {!chatCollapsed && (
          <>
            <div className="resizer-v" onMouseDown={startDrag("chat")} />
            <div
              style={{
                width: chatWidth,
                background: "#1e1e1e",
                color: "#ccc",
                display: "flex",
                flexDirection: "column",
                flexShrink: 0,
              }}
            >
              <div
                style={{
                  padding: "8px 14px",
                  fontSize: 11,
                  opacity: 0.6,
                  textTransform: "uppercase",
                  letterSpacing: 0.5,
                  fontWeight: 600,
                  borderBottom: "1px solid #000",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                }}
              >
                <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  <Sparkles size={12} />
                  <span>AI Assistant</span>
                </span>
                <span
                  style={{
                    cursor: "pointer",
                    opacity: 0.7,
                    fontSize: 11,
                    display: "flex",
                    alignItems: "center",
                    gap: 4,
                  }}
                  onClick={() => setSettingsOpen(true)}
                >
                  <Settings2 size={12} />
                  <span>Settings</span>
                </span>
                <span
                  style={{
                    cursor: "pointer",
                    opacity: 0.7,
                    display: "flex",
                    alignItems: "center",
                  }}
                  onClick={() => setChatCollapsed(true)}
                >
                  <X size={12} />
                </span>
              </div>
              <div
                style={{
                  flex: 1,
                  overflowY: "auto",
                  padding: 10,
                  fontSize: 13,
                }}
              >
                {chatHistory.length === 0 && (
                  <div style={{ opacity: 0.4, fontSize: 12 }}>
                    Ask about your open file, ROS log, or last build.
                  </div>
                )}
                {chatHistory.map((msg, i) => {
                  const codeMatch = msg.text.match(
                    /```(?:python|py|xml)?\n([\s\S]*?)```/,
                  );
                  const isXml = /```xml/.test(msg.text);
                  return (
                    <div key={i} style={{ marginBottom: 12 }}>
                      <strong
                        style={{
                          color: msg.role === "user" ? "#4fc3f7" : "#81c784",
                          fontSize: 12,
                        }}
                      >
                        {msg.role === "user" ? "You" : "AI"}
                      </strong>
                      <div
                        style={{
                          whiteSpace: "pre-wrap",
                          marginTop: 3,
                          lineHeight: 1.45,
                        }}
                      >
                        {msg.text}
                      </div>
                      {codeMatch && (
                        <button
                          className="ide-btn"
                          style={{ marginTop: 6, fontSize: 11 }}
                          onClick={() => saveGeneratedCode(codeMatch[1], isXml)}
                        >
                          <Save size={12} style={{ marginRight: 4 }} /> Save to
                          file
                        </button>
                      )}
                    </div>
                  );
                })}
                {aiLoading && (
                  <div style={{ opacity: 0.6, fontSize: 12 }}>Thinking…</div>
                )}
              </div>
              <div
                style={{
                  display: "flex",
                  gap: 4,
                  padding: "8px 8px 0",
                  flexWrap: "wrap",
                }}
              >
                <button
                  className="ide-btn"
                  style={{ fontSize: 11 }}
                  onClick={() =>
                    quickAction(
                      "generate_node",
                      "Generate a simple ROS 2 publisher node based on this workspace.",
                    )
                  }
                >
                  + Node
                </button>
                <button
                  className="ide-btn"
                  style={{ fontSize: 11 }}
                  onClick={() =>
                    quickAction(
                      "generate_launch",
                      "Generate a launch file for this workspace's packages.",
                    )
                  }
                >
                  + Launch File
                </button>
                <button
                  className="ide-btn"
                  style={{ fontSize: 11 }}
                  onClick={() =>
                    quickAction(
                      "generate_urdf",
                      "Generate a URDF for a simple robot, using this workspace's frame names if relevant.",
                    )
                  }
                >
                  + URDF
                </button>
                <button
                  className="ide-btn"
                  style={{ fontSize: 11 }}
                  disabled={buildOutput.length === 0}
                  onClick={() =>
                    quickAction(
                      "explain_error",
                      "Explain the most recent build error and how to fix it.",
                    )
                  }
                >
                  Explain Last Build
                </button>
              </div>
              <div
                style={{
                  display: "flex",
                  padding: 8,
                  gap: 6,
                  borderTop: "1px solid #000",
                }}
              >
                <input
                  value={chatInput}
                  onChange={(e) => setChatInput(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && sendChatMessage()}
                  style={{
                    flex: 1,
                    background: "#2d2d30",
                    border: "1px solid #3c3c3c",
                    borderRadius: 4,
                    color: "#eee",
                    padding: "6px 8px",
                    fontSize: 13,
                  }}
                  placeholder="Ask about your workspace…"
                />
                <button
                  className="ide-btn primary"
                  onClick={() => sendChatMessage()}
                >
                  Send
                </button>
              </div>
            </div>
          </>
        )}
      </div>

      {/* Bottom panel: resizable + tabbed */}
      {!bottomCollapsed ? (
        <>
          <div className="resizer-h" onMouseDown={startDrag("bottom")} />
          <div
            style={{
              height: bottomHeight,
              display: "flex",
              flexDirection: "column",
              borderTop: "1px solid #000",
              flexShrink: 0,
            }}
          >
            <div
              style={{
                display: "flex",
                background: "#252526",
                alignItems: "center",
                flexShrink: 0,
              }}
            >
              {(
                [
                  "ros",
                  "build",
                  "environment",
                  "sim",
                  "tf",
                  "dashboard",
                  "problems",
                ] as BottomTab[]
              ).map((tab) => (
                <div
                  key={tab}
                  className={`bottom-tab ${activeTab === tab ? "active" : ""}`}
                  onClick={() => setActiveTab(tab)}
                >
                  {tab === "ros" && (
                    <>
                      ROS Log
                      {rosEvents.length > 0 && (
                        <span style={{ opacity: 0.5 }}>
                          ({rosEvents.length})
                        </span>
                      )}
                    </>
                  )}
                  {tab === "environment" && (
                    <span
                      style={{
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 4,
                      }}
                    >
                      Environment Setup
                      {initializing && (
                        <LoaderCircle
                          size={12}
                          style={{ animation: "spin 1s linear infinite" }}
                        />
                      )}
                    </span>
                  )}
                  {tab === "build" && (
                    <span
                      style={{
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 4,
                      }}
                    >
                      Build Output
                      {building && (
                        <LoaderCircle
                          size={12}
                          style={{ animation: "spin 1s linear infinite" }}
                        />
                      )}
                    </span>
                  )}
                  {tab === "sim" && "3D View"}
                  {tab === "tf" && "TF Tree"}
                  {tab === "dashboard" && "Dashboard"}
                  {tab === "problems" && "Problems"}
                </div>
              ))}
              <div style={{ flex: 1 }} />
              <span
                style={{
                  padding: "0 10px",
                  fontSize: 13,
                  cursor: "pointer",
                  opacity: 0.6,
                  display: "flex",
                  alignItems: "center",
                }}
                title="Close panel (⌘`)"
                onClick={() => setBottomCollapsed(true)}
              >
                <ChevronDown size={14} />
              </span>
            </div>

            <div style={{ flex: 1, minHeight: 0, background: "#111" }}>
              {activeTab === "ros" && (
                <div
                  style={{
                    height: "100%",
                    display: "flex",
                    flexDirection: "column",
                  }}
                >
                  <div
                    style={{
                      padding: 6,
                      borderBottom: "1px solid #222",
                      flexShrink: 0,
                    }}
                  >
                    <input
                      value={logFilter}
                      onChange={(e) => setLogFilter(e.target.value)}
                      placeholder="Filter (e.g. NodeCrashed, vehicle_blue, TopicSnapshot)..."
                      style={{
                        width: "100%",
                        background: "#1a1a1a",
                        border: "1px solid #333",
                        color: "#eee",
                        padding: "4px 8px",
                        fontSize: 12,
                        borderRadius: 3,
                      }}
                    />
                  </div>
                  <div
                    style={{
                      flex: 1,
                      overflowY: "auto",
                      color: "#0f0",
                      fontSize: 11,
                      fontFamily: "monospace",
                      padding: 8,
                    }}
                  >
                    {rosEvents.length === 0 && (
                      <div style={{ color: "#666" }}>No ROS events yet.</div>
                    )}
                    {rosEvents
                      .filter((evt) =>
                        evt.toLowerCase().includes(logFilter.toLowerCase()),
                      )
                      .map((evt, i) => {
                        const isCrash = evt.includes("NodeCrashed");
                        const isError = evt.toLowerCase().includes("error");
                        return (
                          <div
                            key={i}
                            style={{
                              color: isCrash || isError ? "#ff6b6b" : "#0f0",
                            }}
                          >
                            {evt}
                          </div>
                        );
                      })}
                  </div>
                </div>
              )}
              {activeTab === "build" && (
                <div
                  style={{
                    height: "100%",
                    overflowY: "auto",
                    color: "#0f0",
                    fontSize: 11,
                    fontFamily: "monospace",
                    padding: 8,
                  }}
                >
                  {buildOutput.length === 0 && (
                    <div style={{ color: "#666" }}>No build output yet.</div>
                  )}
                  {buildOutput.map((line, i) => (
                    <div
                      key={i}
                      style={{
                        color: line.startsWith("ERROR") ? "#e08080" : undefined,
                      }}
                    >
                      {line}
                    </div>
                  ))}
                </div>
              )}
              {activeTab === "environment" && (
                <div
                  style={{
                    height: "100%",
                    overflowY: "auto",
                    color: "#0f0",
                    fontSize: 11,
                    fontFamily: "monospace",
                    padding: 8,
                  }}
                >
                  {setupOutput.length === 0 && (
                    <div style={{ color: "#666" }}>No setup logs yet.</div>
                  )}
                  {setupOutput.map((line, i) => (
                    <div
                      key={i}
                      style={{
                        color: line.startsWith("❌")
                          ? "#e08080"
                          : line.startsWith("✅")
                            ? "#7cd992"
                            : undefined,
                      }}
                    >
                      {line}
                    </div>
                  ))}
                </div>
              )}
              {activeTab === "dashboard" && (
                <div
                  style={{
                    height: "100%",
                    overflowY: "auto",
                    padding: 12,
                    fontSize: 12,
                  }}
                >
                  <div style={{ marginBottom: 12 }}>
                    <strong>ROS environment</strong>
                    <div
                      style={{
                        marginTop: 4,
                        color: rosConnected ? "#7cd992" : "#e0b060",
                      }}
                    >
                      {initializing
                        ? "Initializing"
                        : rosConnected
                          ? "Connected"
                          : "Disconnected"}
                    </div>
                  </div>
                  <div style={{ marginBottom: 12 }}>
                    <strong>Selected workspace</strong>
                    <div
                      style={{
                        marginTop: 4,
                        color: "#aaa",
                        overflowWrap: "anywhere",
                      }}
                    >
                      {rosWorkspacePath ?? "No ROS workspace selected"}
                    </div>
                  </div>
                  <div style={{ marginBottom: 12 }}>
                    <strong>Build</strong>
                    <div
                      style={{
                        marginTop: 4,
                        color: building ? "#e0b060" : "#aaa",
                      }}
                    >
                      {building ? "Running" : "Idle"}
                    </div>
                  </div>
                  <div style={{ marginBottom: 6 }}>
                    <strong>ROS nodes</strong>
                  </div>
                  {Object.keys(nodeStatus).length === 0 ? (
                    <div style={{ color: "#777" }}>
                      {rosConnected
                        ? "No nodes reported"
                        : "Unavailable while disconnected"}
                    </div>
                  ) : (
                    Object.entries(nodeStatus).map(([name, status]) => (
                      <div
                        key={name}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 8,
                          padding: "4px 0",
                        }}
                      >
                        <span
                          style={{
                            color: status === "alive" ? "#7cd992" : "#e08080",
                          }}
                        >
                          {status === "alive" ? "●" : "○"}
                        </span>
                        <span style={{ fontFamily: "monospace" }}>{name}</span>
                        <span style={{ marginLeft: "auto", opacity: 0.6 }}>
                          {status}
                        </span>
                      </div>
                    ))
                  )}
                  {currentSetupLine && (
                    <div style={{ marginTop: 12, color: "#777" }}>
                      Latest environment message: {currentSetupLine}
                    </div>
                  )}
                </div>
              )}
              {activeTab === "sim" && (
                <div style={{ height: "100%", display: "flex" }}>
                  <div style={{ flex: 1 }}>
                    <RobotView
                      descriptionContent={
                        activeFile?.name.toLowerCase().endsWith(".urdf")
                          ? activeFile.content
                          : null
                      }
                    />
                  </div>
                  <div style={{ width: 200, borderLeft: "1px solid #333" }}>
                    <RobotControls
                      onCommand={sendTwist}
                      availableRobots={availableRobots}
                      safetyState={safetyState}
                      safetyPending={safetyPending}
                      safetyError={safetyError}
                      commandError={robotCommandError}
                      onEmergencyStop={() => void requestEmergencyStop(true)}
                      onResetEmergencyStop={() =>
                        void requestEmergencyStop(false)
                      }
                    />
                  </div>
                </div>
              )}
              {activeTab === "tf" && <TFTree onExplain={explainTfFrame} />}
              {activeTab === "problems" && (
                <div style={{ padding: 8, fontSize: 12 }}>
                  {eventHealth && (
                    <div
                      role="status"
                      style={{
                        padding: "6px 0",
                        color: !eventHealth.persistenceHealthy
                          ? "#e08080"
                          : eventHealth.criticalOverflows > 0
                            ? "#e0b060"
                            : "#7cd992",
                      }}
                    >
                      ROS event queue: {eventHealth.subscriberCount}{" "}
                      subscriber(s), backlog {eventHealth.maxBacklog}/
                      {eventHealth.queueCapacity}, critical overflows{" "}
                      {eventHealth.criticalOverflows}, coalesced/dropped
                      telemetry {eventHealth.droppedTelemetry}; retained
                      critical events {eventHealth.retainedCriticalEvents}.
                    </div>
                  )}
                  {eventHealth && !eventHealth.persistenceHealthy && (
                    <div
                      role="alert"
                      style={{ color: "#e08080", padding: "4px 0" }}
                    >
                      Critical event history is unavailable:{" "}
                      {eventHealth.persistenceError}
                    </div>
                  )}
                  {eventRecovery?.gapDetected && (
                    <div
                      role="status"
                      style={{ color: "#e0b060", padding: "4px 0" }}
                    >
                      ROS event history had a gap.{" "}
                      {eventRecovery.snapshotIncluded
                        ? "Current state snapshots were restored."
                        : "A state snapshot is not available yet."}{" "}
                      Oldest retained sequence:{" "}
                      {eventRecovery.oldestAvailableSequence}; latest sequence:{" "}
                      {eventRecovery.latestSequence}.
                    </div>
                  )}
                  {diagnostics.length === 0 ? (
                    <div style={{ color: "#777" }}>
                      No problems reported by recent environment, build, or ROS
                      output
                    </div>
                  ) : (
                    diagnostics.slice(-50).map((diagnostic, index) => (
                      <div
                        key={`${index}-${diagnostic}`}
                        style={{ color: "#e08080", padding: "4px 0" }}
                      >
                        {diagnostic}
                      </div>
                    ))
                  )}
                </div>
              )}
            </div>
          </div>
        </>
      ) : (
        <div
          style={{
            height: 22,
            background: "#252526",
            borderTop: "1px solid #000",
            display: "flex",
            alignItems: "center",
            paddingLeft: 10,
            fontSize: 11,
            color: "#888",
            cursor: "pointer",
            flexShrink: 0,
            gap: 6,
          }}
          onClick={() => setBottomCollapsed(false)}
        >
          <ChevronUp size={12} />
          <span>
            Show panel (ROS Log · Build Output · Environment · 3D View)
          </span>
        </div>
      )}

      {/* Status bar */}
      <div
        style={{
          height: 22,
          background: rosConnected ? "#0e5a8a" : "#7a2e2e",
          color: "#fff",
          display: "flex",
          alignItems: "center",
          fontSize: 11,
          flexShrink: 0,
        }}
      >
        <div
          className="status-item clickable"
          onClick={() => setBottomCollapsed(false)}
        >
          {rosConnected ? "● ROS Connected" : "○ ROS Disconnected"}
        </div>
        <div className="status-item">
          {rosWorkspacePath
            ? `Workspace: ${rosWorkspacePath}`
            : "No ROS workspace"}
        </div>
        {building && (
          <div
            className="status-item"
            style={{ display: "inline-flex", alignItems: "center", gap: 4 }}
          >
            <LoaderCircle
              size={12}
              style={{ animation: "spin 1s linear infinite" }}
            />
            Building…
          </div>
        )}
        <div style={{ flex: 1 }} />
        {dirtyCount > 0 && (
          <div className="status-item">{dirtyCount} unsaved</div>
        )}
        {activeFile && (
          <div className="status-item">{languageFor(activeFile.name)}</div>
        )}
        {activeFile && <div className="status-item">UTF-8</div>}
        {activeFile && activeFileIgnored && (
          <div
            className="status-item"
            title="This file matches a pattern in .aiignore and is excluded from AI context"
            style={{
              color: "#e0b050",
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
            }}
          >
            <Ban size={12} />
            AI-ignored
          </div>
        )}
        <div
          className="status-item clickable"
          onClick={() => setChatCollapsed((c) => !c)}
          style={{ display: "inline-flex", alignItems: "center", gap: 4 }}
        >
          <Sparkles size={12} />
          Assistant
        </div>
      </div>

      {/* Environment Setup Toast */}
      {initializing && (
        <div
          style={{
            position: "fixed",
            bottom: 40,
            right: 20,
            background: "#1e2e3a",
            border: "1px solid #0078d4",
            borderRadius: 6,
            padding: "12px 16px",
            maxWidth: 380,
            boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
            zIndex: 9998,
            display: "flex",
            alignItems: "center",
            gap: 12,
            fontSize: 12,
            color: "#e3f2fd",
          }}
        >
          <div
            style={{
              display: "inline-flex",
              alignItems: "center",
              justifyContent: "center",
              animation: "spin 1s linear infinite",
              fontSize: 14,
            }}
          >
            <Settings2 size={14} />
          </div>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ marginBottom: 4, fontWeight: 600, color: "#4fc3f7" }}>
              Environment Setup
            </div>
            <div
              style={{ fontSize: 11, color: "#bbb", wordBreak: "break-word" }}
            >
              {currentSetupLine || "Initializing..."}
            </div>
          </div>
          <button
            onClick={() => {
              // Dismiss toast but keep initialization running
              setCurrentSetupLine("");
            }}
            style={{
              background: "transparent",
              border: "none",
              color: "#888",
              cursor: "pointer",
              fontSize: 16,
              padding: "0 4px",
              opacity: 0.6,
              transition: "opacity 0.2s",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.opacity = "1";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.opacity = "0.6";
            }}
          >
            <X size={14} />
          </button>
        </div>
      )}

      <style>{`
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      `}</style>
    </div>
  );
}

export default App;
