import { useState, useEffect, useRef, useCallback } from "react";
import Editor from "@monaco-editor/react";
import { invoke } from "@tauri-apps/api/core";
import RobotView, { RobotControls } from "./components/RobotView";
import { listen, emit } from "@tauri-apps/api/event";
import TFTree from "./components/TFTree";
import Settings from "./components/Settings";
import NewProjectModal from "./components/NewProjectModal";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  KeyBinding,
  DEFAULT_KEYBINDINGS,
  parseKeybinding,
  keyEventMatches,
} from "./keybindings";

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

interface LogEvent {
  kind: "append" | "replace";
  text: string;
}

const MAX_RECENT_PROJECTS = 8;

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
              style={{ paddingLeft: 16 + depth * 14 }}
              onClick={() =>
                entry.is_dir
                  ? onToggleFolder(entry.path)
                  : onOpenFile(entry.path, entry.name)
              }
            >
              <span>{entry.is_dir ? (isExpanded ? "📂" : "📁") : "📄"}</span>
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
  // --- File browser state ---
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [currentPath, setCurrentPath] = useState<string | null>(null);

  // --- Multi-tab editor state ---
  const [openFiles, setOpenFiles] = useState<OpenFile[]>([]);
  const [activeFilePath, setActiveFilePath] = useState<string | null>(null);
  const activeFile = openFiles.find((f) => f.path === activeFilePath) ?? null;

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
    const selected = await open({
      filters: [{ name: "SDF World", extensions: ["sdf"] }],
      multiple: false,
    });
    if (typeof selected === "string") {
      setWorldPath(selected);
    }
  };

  // --- ROS event log ---
  const [rosEvents, setRosEvents] = useState<string[]>([]);
  const [rosConnected, setRosConnected] = useState(false);
  const [initializing, setInitializing] = useState(false);
  const [setupOutput, setSetupOutput] = useState<string[]>([]); // Environment setup logs
  const [currentSetupLine, setCurrentSetupLine] = useState<string>(""); // Current line for toast

  // --- Build output ---
  const [buildOutput, setBuildOutput] = useState<string[]>([]);
  const [building, setBuilding] = useState(false);

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
  const [sidebarWidth, setSidebarWidth] = useState(240);
  const [chatWidth, setChatWidth] = useState(320);
  const [bottomHeight, setBottomHeight] = useState(220);
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
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
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
    const selected = await save({
      defaultPath: currentPath ?? undefined,
      filters: isXml
        ? [{ name: "URDF/XML", extensions: ["urdf", "xacro", "xml"] }]
        : [{ name: "Python", extensions: ["py"] }],
    });
    if (typeof selected === "string") {
      await invoke("write_file", { path: selected, contents: code });
    }
  };

  // ---------- ROS workspace selection ----------
  const pickRosWorkspace = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      setRosWorkspacePath(selected);
    }
  };

  // ---------- Build actions ----------
  const triggerBuild = async () => {
    setBuildOutput([]);
    setBuilding(true);
    setBottomCollapsed(false);
    setActiveTab("build");
    try {
      await invoke("run_colcon_build_streaming");
    } finally {
      setBuilding(false);
    }
  };

  const initializeRosEnvironment = async () => {
    setInitializing(true);
    setSetupOutput(["Starting..."]);
    setCurrentSetupLine("Starting...");
    setBottomCollapsed(false);
    setActiveTab("environment");
    try {
      await invoke("initialize_ros_environment");
    } catch (err) {
      setSetupOutput((prev) => [...prev, `❌ ${err}`]);
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
    try {
      await invoke("publish_twist", { topicName, linearX, angularZ });
    } catch (err) {
      console.error("Twist command failed:", err);
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
        userMessage: question,
        openFileContent: activeFile?.content || null,
        openFilePath: activeFile?.path || null,
        workspaceRoot: currentPath ?? null,
        recentRosEvents: rosEvents,
        recentBuildOutput: buildOutput,
        tfContext: tfContext ?? null,
        mode: mode ?? null,
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
  }) => {
    const tfContext =
      `parent_frame: ${tf.parent_frame}\n` +
      `child_frame: ${tf.child_frame}\n` +
      `translation: x=${tf.x.toFixed(4)}, y=${tf.y.toFixed(4)}, z=${tf.z.toFixed(4)}\n` +
      `rotation (quaternion): x=${tf.qx.toFixed(4)}, y=${tf.qy.toFixed(4)}, ` +
      `z=${tf.qz.toFixed(4)}, w=${tf.qw.toFixed(4)}`;
    setChatCollapsed(false);
    sendChatMessage(
      `Explain the TF frame relationship "${tf.parent_frame} → ${tf.child_frame}".`,
      "explain_tf",
      tfContext,
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
    });

    const unlistenConnecting = listen<void>("ros-connecting", () => {
      setRosConnected(false);
    });

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
        setBuildOutput((prev) => [...prev, text]);
      }
    });
    const unlistenBuildError = listen<string>("build-error", (event) => {
      setBuildOutput((prev) => [...prev, `ERROR: ${event.payload}`]);
    });
    const unlistenBuildFinished = listen<boolean>("build-finished", (event) => {
      setBuilding(false);
      setBuildOutput((prev) => [
        ...prev,
        event.payload ? "✅ Build succeeded" : "❌ Build failed",
      ]);
    });

    return () => {
      unlistenEvent.then((f) => f());
      unlistenError.then((f) => f());
      unlistenNodeSnapshot.then((f) => f());
      unlistenConnecting.then((f) => f());
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
        setSetupOutput((prev) => [...prev.slice(-99), text]);
      }
      // Always update the toast's current line
      setCurrentSetupLine(text);
    });
    const unlistenFinished = listen<string>("init-finished", (event) => {
      setRosWorkspacePath(event.payload);
      setSetupOutput((prev) => [...prev, "✅ ROS environment ready"]);
      setCurrentSetupLine("✅ ROS environment ready");
      setInitializing(false);
    });
    const unlistenError = listen<string>("init-error", (event) => {
      setSetupOutput((prev) => [...prev, `❌ ${event.payload}`]);
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
        * { box-sizing: border-box; }
        ::-webkit-scrollbar { width: 10px; height: 10px; }
        ::-webkit-scrollbar-thumb { background: #3e3e42; border-radius: 5px; }
        ::-webkit-scrollbar-thumb:hover { background: #55555a; }
        ::-webkit-scrollbar-track { background: transparent; }
        .ide-btn {
          background: #2d2d30; color: #ddd; border: 1px solid #3c3c3c;
          border-radius: 4px; padding: 4px 10px; font-size: 12px; cursor: pointer;
          transition: background 0.12s, border-color 0.12s;
        }
        .ide-btn:hover { background: #3a3a3d; border-color: #4fc3f7; }
        .ide-btn:disabled { opacity: 0.4; cursor: default; }
        .ide-btn:disabled:hover { background: #2d2d30; border-color: #3c3c3c; }
        .ide-btn.primary { border-color: #4fc3f7; color: #4fc3f7; }
        .ide-btn.danger:hover { border-color: #e08080; color: #e08080; }
        .activity-icon {
          width: 48px; height: 44px; display: flex; align-items: center;
          justify-content: center; font-size: 18px; cursor: pointer;
          color: #858585; border-left: 2px solid transparent; position: relative;
        }
        .activity-icon:hover { color: #fff; }
        .activity-icon.active { color: #fff; border-left: 2px solid #4fc3f7; background: #1e1e1e; }
        .file-row {
          padding: 3px 10px 3px 16px; font-size: 13px; cursor: pointer;
          display: flex; align-items: center; gap: 6px; white-space: nowrap;
          border-left: 2px solid transparent;
        }
        .file-row:hover { background: #2a2d2e; }
        .file-row.active { background: #37373d; border-left: 2px solid #4fc3f7; }
        .tab {
          display: flex; align-items: center; gap: 6px; padding: 0 8px 0 12px;
          height: 34px; font-size: 12.5px; cursor: pointer; white-space: nowrap;
          border-right: 1px solid #1e1e1e; color: #969696; position: relative; flex-shrink: 0;
        }
        .tab.active { background: #1e1e1e; color: #fff; }
        .tab:hover { color: #fff; }
        .tab-close {
          width: 16px; height: 16px; border-radius: 3px; display: flex;
          align-items: center; justify-content: center; font-size: 12px; opacity: 0.6;
        }
        .tab-close:hover { background: #4a4a4d; opacity: 1; }
        .resizer-v { width: 4px; cursor: col-resize; flex-shrink: 0; background: transparent; }
        .resizer-v:hover, .resizer-v:active { background: #4fc3f7; }
        .resizer-h { height: 4px; cursor: row-resize; flex-shrink: 0; background: transparent; }
        .resizer-h:hover, .resizer-h:active { background: #4fc3f7; }
        .bottom-tab {
          padding: 6px 14px; font-size: 12px; cursor: pointer; user-select: none;
          border-right: 1px solid #333; display: flex; align-items: center; gap: 6px;
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
          📁 Open Folder
        </button>
        <button className="ide-btn" onClick={() => setNewProjectOpen(true)}>
          ✨ New Project from Template
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
          🚫 .aiignore
        </button>
        <button className="ide-btn" onClick={pickRosWorkspace}>
          🤖 {rosWorkspacePath ? "Workspace Set" : "Select ROS Workspace"}
        </button>
        <button onClick={initializeRosEnvironment} disabled={initializing}>
          {initializing ? "⏳ Setting up..." : "🚀 Initialize ROS Environment"}
        </button>
        <div
          style={{ width: 1, height: 20, background: "#444", margin: "0 4px" }}
        />
        <button
          className="ide-btn primary"
          onClick={triggerBuild}
          disabled={building}
        >
          {building ? "⏳ Building…" : "🔨 Build"}
        </button>
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
          ▶️ Start Sim
        </button>
        <button className="ide-btn danger" onClick={stopSim}>
          ⏹ Stop Sim
        </button>
        <button className="ide-btn" onClick={resetSim}>
          🔄 Reset Sim
        </button>
        <div style={{ flex: 1 }} />
        <button
          className="ide-btn"
          onClick={saveFile}
          disabled={
            !activeFile || activeFile.content === activeFile.savedContent
          }
        >
          💾 Save
        </button>
      </div>

      {/* Main area: activity bar + sidebar + editor + AI chat */}
      <div style={{ flex: 1, display: "flex", minHeight: 0 }}>
        {/* Activity bar */}
        <div
          style={{
            width: 48,
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
            📁
          </div>
          <div className="activity-icon" title="Search">
            🔍
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
            🤖
          </div>
          <div style={{ flex: 1 }} />
          <div
            className={`activity-icon ${!chatCollapsed ? "active" : ""}`}
            title="AI Assistant"
            onClick={() => setChatCollapsed((c) => !c)}
          >
            ✨
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
                                  color: "#a0a0a0",
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
                  <div style={{ opacity: 0.7, marginBottom: 6 }}>
                    Workspace:{" "}
                    <span
                      style={{
                        color: rosWorkspacePath ? "#7cd992" : "#e08080",
                      }}
                    >
                      {rosWorkspacePath ? "set" : "not set"}
                    </span>
                  </div>
                  <div style={{ opacity: 0.5, fontSize: 11 }}>
                    Live node/topic graph coming soon — inspect activity in the
                    ROS Log panel below.
                  </div>
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
                  <span>📄</span>
                  <span>{f.name}</span>
                  <span
                    className="tab-close"
                    onClick={(e) => closeTab(f.path, e)}
                  >
                    {dirty ? "●" : "✕"}
                  </span>
                </div>
              );
            })}
            {openFiles.length === 0 && (
              <div style={{ padding: "8px 14px", fontSize: 12, opacity: 0.4 }}>
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
                <div style={{ fontSize: 32, opacity: 0.5 }}>🤖</div>
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
                <span>✨ AI Assistant</span>
                <span
                  style={{ cursor: "pointer", opacity: 0.7, fontSize: 11 }}
                  onClick={() => setSettingsOpen(true)}
                >
                  ⚙️ Settings
                </span>
                <span
                  style={{ cursor: "pointer", opacity: 0.7 }}
                  onClick={() => setChatCollapsed(true)}
                >
                  ✕
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
                          💾 Save to file
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
                    <>Environment Setup{initializing && <span>⏳</span>}</>
                  )}
                  {activeTab === "dashboard" && (
                    <div style={{ padding: 12, fontSize: 12 }}>
                      <div
                        style={{
                          opacity: 0.6,
                          marginBottom: 8,
                          textTransform: "uppercase",
                          fontSize: 11,
                        }}
                      >
                        Node Health
                      </div>
                      {Object.keys(nodeStatus).length === 0 && (
                        <div style={{ color: "#666" }}>
                          No nodes detected yet.
                        </div>
                      )}
                      {Object.entries(nodeStatus).map(([name, status]) => (
                        <div
                          key={name}
                          style={{
                            display: "flex",
                            alignItems: "center",
                            gap: 8,
                            padding: "4px 0",
                            borderBottom: "1px solid #222",
                          }}
                        >
                          <span
                            style={{
                              color: status === "alive" ? "#7cd992" : "#e08080",
                            }}
                          >
                            {status === "alive" ? "●" : "○"}
                          </span>
                          <span style={{ fontFamily: "monospace" }}>
                            {name}
                          </span>
                          <span
                            style={{
                              opacity: 0.5,
                              marginLeft: "auto",
                              fontSize: 11,
                            }}
                          >
                            {status}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                  {tab === "build" && (
                    <>Build Output{building && <span>⏳</span>}</>
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
                }}
                title="Close panel (⌘`)"
                onClick={() => setBottomCollapsed(true)}
              >
                ⌄
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
              {activeTab === "sim" && (
                <div style={{ height: "100%", display: "flex" }}>
                  <div style={{ flex: 1 }}>
                    <RobotView />
                  </div>
                  <div style={{ width: 200, borderLeft: "1px solid #333" }}>
                    <RobotControls
                      onCommand={sendTwist}
                      availableRobots={availableRobots}
                    />
                  </div>
                </div>
              )}
              {activeTab === "tf" && <TFTree onExplain={explainTfFrame} />}
              {activeTab === "problems" && (
                <div style={{ padding: 8, fontSize: 12, color: "#666" }}>
                  No problems detected.
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
          }}
          onClick={() => setBottomCollapsed(false)}
        >
          ⌃ Show panel (ROS Log · Build Output · Environment · 3D View)
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
        {building && <div className="status-item">⏳ Building…</div>}
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
            style={{ color: "#e0b050" }}
          >
            🚫 AI-ignored
          </div>
        )}
        <div
          className="status-item clickable"
          onClick={() => setChatCollapsed((c) => !c)}
        >
          ✨ Assistant
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
              display: "inline-block",
              animation: "spin 1s linear infinite",
              fontSize: 14,
            }}
          >
            ⚙️
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
            ✕
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
