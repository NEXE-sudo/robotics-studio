import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { X, Lightbulb } from "lucide-react";
import {
  KeyBinding,
  DEFAULT_KEYBINDINGS,
  normalizeVscodeKeybinding,
} from "../keybindings";

interface AIProviderSettings {
  provider: string;
  api_key: string;
  model: string;
}

interface SystemSpecs {
  ram_gb: number;
  cpu_cores: number;
  gpu_name: string | null;
  gpu_vram_gb: number | null;
}

interface VsCodeKeybinding {
  key: string;
  command: string;
  when?: string;
}

const MODELS_BY_PROVIDER: Record<string, string[]> = {
  groq: [
    "llama-3.3-70b-versatile",
    "llama-3.1-8b-instant",
    "mixtral-8x7b-32768",
  ],
  anthropic: ["claude-sonnet-4-5", "claude-opus-4-1", "claude-haiku-4-5"],
  openai: ["gpt-4o", "gpt-4o-mini", "gpt-4-turbo"],
};

const VSCODE_COMMAND_MAPPING: Record<string, string> = {
  "workbench.action.files.save": "save",
  "workbench.action.toggleSidebarVisibility": "toggle-sidebar",
  "workbench.action.terminal.toggleTerminal": "toggle-bottom-panel",
  "workbench.action.closeActiveEditor": "close-tab",
};

export default function Settings({
  onClose,
  onKeybindingsChange,
}: {
  onClose: () => void;
  onKeybindingsChange?: (keybindings: KeyBinding[]) => void;
}) {
  // AI Provider tab state
  const [provider, setProvider] = useState("groq");
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState(MODELS_BY_PROVIDER["groq"][0]);
  const [aiSaved, setAiSaved] = useState(false);

  // Tab state
  const [activeTab, setActiveTab] = useState<"ai" | "shortcuts" | "system">(
    "ai",
  );

  // Keyboard shortcuts tab state
  const [keybindings, setKeybindings] =
    useState<KeyBinding[]>(DEFAULT_KEYBINDINGS);
  const [listeningId, setListeningId] = useState<string | null>(null);
  const [shortcutsSaved, setShortcutsSaved] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);
  const [importPreview, setImportPreview] = useState<
    { id: string; oldKeys: string; newKeys: string }[] | null
  >(null);

  // System specs tab state
  const [systemSpecs, setSystemSpecs] = useState<SystemSpecs | null>(null);
  const [systemLoading, setSystemLoading] = useState(false);
  const [systemError, setSystemError] = useState<string | null>(null);

  // Load AI settings on mount
  useEffect(() => {
    invoke<AIProviderSettings | null>("load_ai_settings").then((existing) => {
      if (existing) {
        setProvider(existing.provider);
        setApiKey(existing.api_key);
        setModel(existing.model);
      }
    });
  }, []);

  // Load keybindings on mount
  useEffect(() => {
    invoke<KeyBinding[] | null>("load_keybindings").then((loaded) => {
      if (loaded) {
        setKeybindings(loaded);
      }
    });
  }, []);

  // Load system specs when System tab is opened
  useEffect(() => {
    if (activeTab === "system" && !systemSpecs && !systemLoading) {
      setSystemLoading(true);
      invoke<SystemSpecs>("get_system_specs")
        .then((specs) => {
          setSystemSpecs(specs);
          setSystemError(null);
        })
        .catch((err) => {
          setSystemError("Couldn't detect system specs");
          console.error(err);
        })
        .finally(() => {
          setSystemLoading(false);
        });
    }
  }, [activeTab, systemSpecs, systemLoading]);

  // AI Provider tab handlers
  const handleProviderChange = (p: string) => {
    setProvider(p);
    setModel(MODELS_BY_PROVIDER[p][0]);
  };

  const handleSaveAi = async () => {
    await invoke("save_ai_settings", {
      settings: { provider, api_key: apiKey, model },
    });
    setAiSaved(true);
    setTimeout(() => setAiSaved(false), 2000);
  };

  // Keyboard shortcuts handlers
  const handleSaveShortcuts = async () => {
    await invoke("save_keybindings", { keybindings });
    if (onKeybindingsChange) {
      onKeybindingsChange(keybindings);
    }
    setShortcutsSaved(true);
    setTimeout(() => setShortcutsSaved(false), 2000);
  };

  const handleResetShortcuts = () => {
    setKeybindings(DEFAULT_KEYBINDINGS);
  };

  const handleListenForKeybinding = (id: string) => {
    setListeningId(id);

    const listener = (e: KeyboardEvent) => {
      e.preventDefault();
      const parts: string[] = [];
      if (e.metaKey) parts.push("Cmd");
      if (e.ctrlKey && !e.metaKey) parts.push("Ctrl");
      if (e.shiftKey) parts.push("Shift");
      if (e.altKey) parts.push("Alt");

      const key =
        e.key === "`" ? "`" : e.key.length === 1 ? e.key.toUpperCase() : e.key;
      if (!["Meta", "Control", "Shift", "Alt"].includes(e.key)) {
        parts.push(key);
      }

      // Only save keybinding if a non-modifier key was pressed
      if (parts.length > 0 && !["Cmd", "Ctrl", "Shift", "Alt"].includes(key)) {
        const newKeys = parts.join("+");
        setKeybindings((prev) =>
          prev.map((kb) => (kb.id === id ? { ...kb, keys: newKeys } : kb)),
        );
        setListeningId(null);
        window.removeEventListener("keydown", listener);
      }
    };

    window.addEventListener("keydown", listener);
  };

  const handleImportVsCode = async () => {
    try {
      const path = await open({
        filters: [{ name: "JSON", extensions: ["json"] }],
        multiple: false,
      });

      if (!path) return;

      const content = await invoke<string>("read_file", { path });
      const vsCodeBindings: VsCodeKeybinding[] = JSON.parse(content);

      if (!Array.isArray(vsCodeBindings)) {
        setImportError("File must contain a JSON array of keybindings");
        return;
      }

      const preview: { id: string; oldKeys: string; newKeys: string }[] = [];
      let skipped = 0;

      for (const binding of vsCodeBindings) {
        const ourId = VSCODE_COMMAND_MAPPING[binding.command];
        if (!ourId) {
          skipped++;
          continue;
        }

        const newKeys = normalizeVscodeKeybinding(binding.key);
        const existing = keybindings.find((kb) => kb.id === ourId);
        if (existing && existing.keys !== newKeys) {
          preview.push({
            id: ourId,
            oldKeys: existing.keys,
            newKeys,
          });
        }
      }

      if (preview.length === 0 && skipped === vsCodeBindings.length) {
        setImportError("No recognized keybindings found in this file");
        return;
      }

      setImportPreview(preview);
      setImportError(null);
    } catch (err) {
      setImportError(
        `Failed to import: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  };

  const handleApplyImport = () => {
    if (!importPreview) return;

    const newKeybindings = keybindings.map((kb) => {
      const preview = importPreview.find((p) => p.id === kb.id);
      return preview ? { ...kb, keys: preview.newKeys } : kb;
    });

    setKeybindings(newKeybindings);
    setImportPreview(null);
  };

  const handleCancelImport = () => {
    setImportPreview(null);
  };

  // Get model suggestion based on RAM
  const getModelSuggestion = (ramGb: number): string => {
    if (ramGb < 8) {
      return "Not enough headroom for local models — cloud providers (Groq/Anthropic/OpenAI) are recommended.";
    }
    if (ramGb < 16) {
      return "Quantized 7-8B parameter models (e.g. Llama 3 8B, Mistral 7B) should run reasonably.";
    }
    if (ramGb < 32) {
      return "13-14B parameter models are a good fit.";
    }
    return "30B+ parameter models or larger quantizations are feasible.";
  };

  const tabStyle = (isActive: boolean) => ({
    padding: "var(--spacing-sm) var(--spacing-md)",
    cursor: "pointer",
    borderBottom: isActive ? "2px solid #4fc3f7" : "none",
    color: isActive ? "#4fc3f7" : "#ccc",
    fontSize: "var(--font-sm)",
    fontWeight: isActive ? 600 : 400,
    transition: "color 0.12s ease, opacity 0.12s ease",
  });

  return (
    <div
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: "rgba(0,0,0,0.5)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 1000,
      }}
    >
      <div
        style={{
          background: "#252526",
          border: "1px solid #3c3c3c",
          borderRadius: 8,
          width: 600,
          maxHeight: "80vh",
          display: "flex",
          flexDirection: "column",
          color: "#ccc",
        }}
      >
        {/* Header */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            padding: "var(--spacing-md) var(--spacing-lg)",
            borderBottom: "1px solid #3c3c3c",
          }}
        >
          <h3 style={{ margin: 0 }}>Settings</h3>
          <X
            style={{
              width: "var(--icon-md)",
              height: "var(--icon-md)",
              cursor: "pointer",
              opacity: 0.7,
              flexShrink: 0,
            }}
            onClick={onClose}
          />
        </div>

        {/* Tab bar */}
        <div
          style={{
            display: "flex",
            borderBottom: "1px solid #3c3c3c",
            paddingLeft: "var(--spacing-lg)",
          }}
        >
          <div
            style={tabStyle(activeTab === "ai")}
            onClick={() => setActiveTab("ai")}
          >
            AI Provider
          </div>
          <div
            style={tabStyle(activeTab === "shortcuts")}
            onClick={() => setActiveTab("shortcuts")}
          >
            Keyboard Shortcuts
          </div>
          <div
            style={tabStyle(activeTab === "system")}
            onClick={() => setActiveTab("system")}
          >
            System & Models
          </div>
        </div>

        {/* Content */}
        <div
          style={{
            flex: 1,
            padding: "var(--spacing-lg)",
            overflowY: "auto",
          }}
        >
          {activeTab === "ai" && (
            <div>
              <label
                style={{
                  display: "block",
                  fontSize: "var(--font-xs)",
                  marginBottom: "var(--spacing-xs)",
                  opacity: 0.8,
                }}
              >
                Provider
              </label>
              <select
                value={provider}
                onChange={(e) => handleProviderChange(e.target.value)}
                style={{
                  width: "100%",
                  padding: "var(--spacing-sm)",
                  marginBottom: "var(--spacing-base)",
                  background: "#2d2d30",
                  color: "#eee",
                  border: "1px solid #3c3c3c",
                }}
              >
                <option value="groq">Groq</option>
                <option value="anthropic">Anthropic (Claude)</option>
                <option value="openai">OpenAI</option>
              </select>

              <label
                style={{
                  display: "block",
                  fontSize: "var(--font-xs)",
                  marginBottom: "var(--spacing-xs)",
                  opacity: 0.8,
                }}
              >
                Model
              </label>
              <select
                value={model}
                onChange={(e) => setModel(e.target.value)}
                style={{
                  width: "100%",
                  padding: "var(--spacing-sm)",
                  marginBottom: "var(--spacing-base)",
                  background: "#2d2d30",
                  color: "#eee",
                  border: "1px solid #3c3c3c",
                }}
              >
                {MODELS_BY_PROVIDER[provider].map((m) => (
                  <option key={m} value={m}>
                    {m}
                  </option>
                ))}
              </select>

              <label
                style={{
                  display: "block",
                  fontSize: "var(--font-xs)",
                  marginBottom: "var(--spacing-xs)",
                  opacity: 0.8,
                }}
              >
                API Key
              </label>
              <input
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="sk-..."
                style={{
                  width: "100%",
                  padding: "var(--spacing-sm)",
                  marginBottom: "var(--spacing-md)",
                  background: "#2d2d30",
                  color: "#eee",
                  border: "1px solid #3c3c3c",
                }}
              />

              <div
                style={{
                  display: "flex",
                  justifyContent: "flex-end",
                  gap: "var(--spacing-sm)",
                }}
              >
                {aiSaved && (
                  <span
                    style={{
                      color: "#7cd992",
                      fontSize: "var(--font-xs)",
                      alignSelf: "center",
                    }}
                  >
                    Saved
                  </span>
                )}
                <button className="ide-btn primary" onClick={handleSaveAi}>
                  Save
                </button>
              </div>
            </div>
          )}

          {activeTab === "shortcuts" && (
            <div>
              <div
                style={{
                  marginBottom: "var(--spacing-md)",
                  maxHeight: 300,
                  overflowY: "auto",
                  border: "1px solid #3c3c3c",
                  borderRadius: 4,
                  padding: "var(--spacing-sm)",
                }}
              >
                {keybindings.map((kb) => (
                  <div
                    key={kb.id}
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      padding: "8px 0",
                      borderBottom: "1px solid #3c3c3c",
                    }}
                  >
                    <span style={{ fontSize: "var(--font-xs)", opacity: 0.9 }}>
                      {kb.description}
                    </span>
                    <div
                      style={{
                        padding: "2px var(--spacing-sm)",
                        background:
                          listeningId === kb.id ? "#3c3c3c" : "#2d2d30",
                        border: "1px solid #3c3c3c",
                        borderRadius: 3,
                        fontSize: "var(--font-xs)",
                        cursor: "pointer",
                        fontFamily: "monospace",
                        minWidth: 80,
                        textAlign: "center",
                        color: listeningId === kb.id ? "#4fc3f7" : "#ccc",
                        transition:
                          "background 0.12s ease, border-color 0.12s ease, opacity 0.12s ease",
                      }}
                      onMouseEnter={(e) => {
                        if (listeningId !== kb.id) {
                          e.currentTarget.style.background = "#3c3c3c";
                          e.currentTarget.style.borderColor = "#4fc3f7";
                        }
                      }}
                      onMouseLeave={(e) => {
                        if (listeningId !== kb.id) {
                          e.currentTarget.style.background = "#2d2d30";
                          e.currentTarget.style.borderColor = "#3c3c3c";
                        }
                      }}
                      onClick={() =>
                        listeningId !== kb.id &&
                        handleListenForKeybinding(kb.id)
                      }
                    >
                      {listeningId === kb.id ? "Press keys..." : kb.keys}
                    </div>
                  </div>
                ))}
              </div>

              {importError && (
                <div
                  style={{
                    color: "#e08080",
                    fontSize: "var(--font-xs)",
                    marginBottom: "var(--spacing-base)",
                    padding: "var(--spacing-sm)",
                    background: "rgba(224, 128, 128, 0.1)",
                    borderRadius: 4,
                  }}
                >
                  {importError}
                </div>
              )}

              {importPreview && (
                <div
                  style={{
                    marginBottom: "var(--spacing-base)",
                    padding: "var(--spacing-sm)",
                    background: "rgba(79, 195, 247, 0.1)",
                    border: "1px solid #4fc3f7",
                    borderRadius: 4,
                  }}
                >
                  <div
                    style={{
                      fontSize: "var(--font-xs)",
                      marginBottom: "var(--spacing-sm)",
                    }}
                  >
                    <strong>Import preview:</strong> {importPreview.length}{" "}
                    keybinding{importPreview.length !== 1 ? "s" : ""} will be
                    updated:
                  </div>
                  {importPreview.map((p) => (
                    <div
                      key={p.id}
                      style={{
                        fontSize: "var(--font-xs)",
                        marginBottom: "var(--spacing-xs)",
                        opacity: 0.8,
                      }}
                    >
                      {p.id}: {p.oldKeys} → {p.newKeys}
                    </div>
                  ))}
                  <div
                    style={{
                      display: "flex",
                      gap: "var(--spacing-sm)",
                      marginTop: "var(--spacing-sm)",
                    }}
                  >
                    <button
                      className="ide-btn primary"
                      style={{
                        fontSize: "var(--font-xs)",
                        padding: "4px var(--spacing-sm)",
                      }}
                      onClick={handleApplyImport}
                    >
                      Apply
                    </button>
                    <button
                      className="ide-btn"
                      style={{
                        fontSize: "var(--font-xs)",
                        padding: "4px var(--spacing-sm)",
                      }}
                      onClick={handleCancelImport}
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              )}

              <div
                style={{
                  display: "flex",
                  gap: "var(--spacing-sm)",
                  justifyContent: "flex-end",
                }}
              >
                {shortcutsSaved && (
                  <span
                    style={{
                      color: "#7cd992",
                      fontSize: "var(--font-xs)",
                      alignSelf: "center",
                    }}
                  >
                    Saved
                  </span>
                )}
                <button
                  className="ide-btn"
                  onClick={handleResetShortcuts}
                  style={{ fontSize: "var(--font-xs)" }}
                >
                  Reset to Defaults
                </button>
                <button
                  className="ide-btn"
                  onClick={handleImportVsCode}
                  style={{ fontSize: "var(--font-xs)" }}
                >
                  Import from VS Code
                </button>
                <button
                  className="ide-btn primary"
                  onClick={handleSaveShortcuts}
                  style={{ fontSize: "var(--font-xs)" }}
                >
                  Save Shortcuts
                </button>
              </div>
            </div>
          )}

          {activeTab === "system" && (
            <div>
              {systemLoading && (
                <div
                  style={{
                    color: "#999",
                    fontSize: "var(--font-xs)",
                    textAlign: "center",
                  }}
                >
                  Detecting system specs...
                </div>
              )}

              {systemError && (
                <div style={{ color: "#e08080", fontSize: "var(--font-xs)" }}>
                  {systemError}
                </div>
              )}

              {systemSpecs && (
                <div>
                  <div style={{ marginBottom: "var(--spacing-md)" }}>
                    <div
                      style={{
                        fontSize: "var(--font-xs)",
                        marginBottom: "var(--spacing-xs)",
                      }}
                    >
                      <strong>RAM:</strong> {systemSpecs.ram_gb} GB
                    </div>
                    <div
                      style={{
                        fontSize: "var(--font-xs)",
                        marginBottom: "var(--spacing-xs)",
                      }}
                    >
                      <strong>CPU cores:</strong> {systemSpecs.cpu_cores}
                    </div>
                    <div
                      style={{
                        fontSize: "var(--font-xs)",
                        marginBottom: "var(--spacing-xs)",
                      }}
                    >
                      <strong>GPU:</strong>{" "}
                      {systemSpecs.gpu_name || "Not detected"}
                    </div>
                    {systemSpecs.gpu_vram_gb && (
                      <div
                        style={{
                          fontSize: "var(--font-xs)",
                          marginBottom: "var(--spacing-xs)",
                        }}
                      >
                        <strong>GPU VRAM:</strong> {systemSpecs.gpu_vram_gb} GB
                      </div>
                    )}
                  </div>

                  <div
                    style={{
                      padding: "var(--spacing-base)",
                      background: "#2d2d30",
                      borderRadius: 4,
                      border: "1px solid #3c3c3c",
                      marginTop: "var(--spacing-md)",
                    }}
                  >
                    <div
                      style={{
                        fontSize: "var(--font-xs)",
                        fontWeight: 600,
                        marginBottom: "var(--spacing-sm)",
                      }}
                    >
                      Offline model suggestions:
                    </div>
                    <div
                      style={{
                        fontSize: "var(--font-xs)",
                        lineHeight: 1.5,
                        opacity: 0.9,
                      }}
                    >
                      {getModelSuggestion(systemSpecs.ram_gb)}
                      {systemSpecs.gpu_vram_gb &&
                        systemSpecs.gpu_vram_gb >= 8 && (
                          <div
                            style={{
                              marginTop: 8,
                              opacity: 0.8,
                              display: "flex",
                              alignItems: "center",
                              gap: 6,
                            }}
                          >
                            <Lightbulb
                              style={{
                                width: "var(--icon-sm)",
                                height: "var(--icon-sm)",
                                flexShrink: 0,
                              }}
                            />
                            <span>
                              GPU-accelerated inference will be significantly
                              faster than CPU-only.
                            </span>
                          </div>
                        )}
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
