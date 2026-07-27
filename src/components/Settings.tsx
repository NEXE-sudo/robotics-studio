import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AIProviderSettings {
  provider: string;
  api_key: string;
  model: string;
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

export default function Settings({ onClose }: { onClose: () => void }) {
  const [provider, setProvider] = useState("groq");
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState(MODELS_BY_PROVIDER["groq"][0]);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    invoke<AIProviderSettings | null>("load_ai_settings").then((existing) => {
      if (existing) {
        setProvider(existing.provider);
        setApiKey(existing.api_key);
        setModel(existing.model);
      }
    });
  }, []);

  const handleProviderChange = (p: string) => {
    setProvider(p);
    setModel(MODELS_BY_PROVIDER[p][0]);
  };

  const handleSave = async () => {
    await invoke("save_ai_settings", {
      settings: { provider, api_key: apiKey, model },
    });
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
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
          padding: 24,
          width: 420,
          color: "#ccc",
        }}
      >
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            marginBottom: 16,
          }}
        >
          <h3 style={{ margin: 0 }}>AI Provider Settings</h3>
          <span style={{ cursor: "pointer", opacity: 0.7 }} onClick={onClose}>
            ✕
          </span>
        </div>

        <label
          style={{
            display: "block",
            fontSize: 12,
            marginBottom: 4,
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
            padding: 6,
            marginBottom: 12,
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
            fontSize: 12,
            marginBottom: 4,
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
            padding: 6,
            marginBottom: 12,
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
            fontSize: 12,
            marginBottom: 4,
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
            padding: 6,
            marginBottom: 16,
            background: "#2d2d30",
            color: "#eee",
            border: "1px solid #3c3c3c",
          }}
        />

        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
          {saved && (
            <span
              style={{ color: "#7cd992", fontSize: 12, alignSelf: "center" }}
            >
              Saved
            </span>
          )}
          <button className="ide-btn primary" onClick={handleSave}>
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
