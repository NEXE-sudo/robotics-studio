import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { X } from "lucide-react";

interface Props {
  onClose: () => void;
  onProjectCreated: (projectPath: string) => void;
}

export default function NewProjectModal({ onClose, onProjectCreated }: Props) {
  const [templates, setTemplates] = useState<string[]>([]);
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null);
  const [destinationDir, setDestinationDir] = useState("");
  const [projectName, setProjectName] = useState("");
  const [loadingTemplates, setLoadingTemplates] = useState(true);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<string[]>("list_templates")
      .then((names) => {
        setTemplates(names);
        if (names.length > 0) setSelectedTemplate(names[0]);
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoadingTemplates(false));
  }, []);

  const pickDestination = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") setDestinationDir(selected);
  };

  const canCreate =
    !!selectedTemplate &&
    !!destinationDir &&
    projectName.trim().length > 0 &&
    !creating;

  const handleCreate = async () => {
    if (!selectedTemplate) return;
    setCreating(true);
    setError(null);
    try {
      const projectPath = await invoke<string>("create_project_from_template", {
        templateName: selectedTemplate,
        destinationDir,
        projectName: projectName.trim(),
      });
      onProjectCreated(projectPath);
      await revealItemInDir(projectPath);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  };

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
          padding: "var(--spacing-lg)",
          width: 460,
          color: "#ccc",
        }}
      >
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            marginBottom: "var(--spacing-md)",
          }}
        >
          <h3 style={{ margin: 0 }}>New Project from Template</h3>
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

        <label
          style={{
            display: "block",
            fontSize: "var(--font-xs)",
            marginBottom: "var(--spacing-xs)",
            opacity: 0.8,
          }}
        >
          Template
        </label>
        {loadingTemplates ? (
          <div
            style={{
              fontSize: "var(--font-xs)",
              opacity: 0.6,
              marginBottom: "var(--spacing-base)",
            }}
          >
            Loading templates…
          </div>
        ) : templates.length === 0 ? (
          <div
            style={{
              fontSize: "var(--font-xs)",
              opacity: 0.6,
              marginBottom: "var(--spacing-base)",
            }}
          >
            No templates bundled.
          </div>
        ) : (
          <select
            value={selectedTemplate ?? ""}
            onChange={(e) => setSelectedTemplate(e.target.value)}
            style={{
              width: "100%",
              padding: "var(--spacing-sm)",
              marginBottom: "var(--spacing-base)",
              background: "#2d2d30",
              color: "#eee",
              border: "1px solid #3c3c3c",
            }}
          >
            {templates.map((t) => (
              <option key={t} value={t}>
                {t}
              </option>
            ))}
          </select>
        )}

        <label
          style={{
            display: "block",
            fontSize: "var(--font-xs)",
            marginBottom: "var(--spacing-xs)",
            opacity: 0.8,
          }}
        >
          Project name
        </label>
        <input
          value={projectName}
          onChange={(e) => setProjectName(e.target.value)}
          placeholder="my_ros2_project"
          style={{
            width: "100%",
            padding: "var(--spacing-sm)",
            marginBottom: "var(--spacing-base)",
            background: "#2d2d30",
            color: "#eee",
            border: "1px solid #3c3c3c",
            boxSizing: "border-box",
          }}
        />

        <label
          style={{
            display: "block",
            fontSize: "var(--font-xs)",
            marginBottom: "var(--spacing-xs)",
            opacity: 0.8,
          }}
        >
          Destination folder
        </label>
        <div
          style={{
            display: "flex",
            gap: "var(--spacing-sm)",
            marginBottom: "var(--spacing-md)",
          }}
        >
          <input
            value={destinationDir}
            readOnly
            placeholder="Choose a folder…"
            style={{
              flex: 1,
              padding: "var(--spacing-sm)",
              background: "#2d2d30",
              color: "#eee",
              border: "1px solid #3c3c3c",
            }}
          />
          <button className="ide-btn" onClick={pickDestination}>
            Browse…
          </button>
        </div>

        {error && (
          <div
            style={{
              color: "#e08080",
              fontSize: "var(--font-xs)",
              marginBottom: "var(--spacing-base)",
            }}
          >
            {error}
          </div>
        )}

        <div
          style={{
            display: "flex",
            justifyContent: "flex-end",
            gap: "var(--spacing-sm)",
          }}
        >
          <button className="ide-btn" onClick={onClose} disabled={creating}>
            Cancel
          </button>
          <button
            className="ide-btn primary"
            onClick={handleCreate}
            disabled={!canCreate}
          >
            {creating ? "Creating…" : "Create Project"}
          </button>
        </div>
      </div>
    </div>
  );
}
