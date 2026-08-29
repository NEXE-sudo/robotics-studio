import React from "react";
import { Bot, Folder } from "lucide-react";

interface EmptyStateProps {
  onOpenFolder: () => void;
  onSelectWorkspace: () => void;
  rosWorkspacePath: string | null;
}

const EmptyState: React.FC<EmptyStateProps> = ({
  onOpenFolder,
  onSelectWorkspace,
  rosWorkspacePath,
}) => {
  return (
    <div
      style={{
        flex: 1,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        background: "#1e1e1e",
        color: "#ccc",
        padding: 40,
        textAlign: "center",
        userSelect: "none",
      }}
    >
      <div
        style={{
          marginBottom: 16,
          opacity: 0.9,
          display: "flex",
          justifyContent: "center",
        }}
      >
        <Bot size={48} />
      </div>
      <h2
        style={{
          margin: "0 0 8px 0",
          fontSize: 22,
          fontWeight: 500,
          color: "#fff",
        }}
      >
        Robotics Studio
      </h2>
      <p
        style={{
          margin: "0 0 28px 0",
          fontSize: 13,
          color: "#888",
          maxWidth: 460,
        }}
      >
        Next-Generation Robotics Development Environment for ROS 2, Gazebo
        Simulation, and Real-Time Introspection.
      </p>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(2, 1fr)",
          gap: 16,
          maxWidth: 520,
          width: "100%",
          marginBottom: 36,
        }}
      >
        <div
          onClick={onOpenFolder}
          style={{
            background: "#252526",
            border: "1px solid #3c3c3c",
            borderRadius: 6,
            padding: 16,
            cursor: "pointer",
            textAlign: "left",
            transition: "all 0.15s ease",
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.borderColor = "#4fc3f7";
            e.currentTarget.style.background = "#2d2d30";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.borderColor = "#3c3c3c";
            e.currentTarget.style.background = "#252526";
          }}
        >
          <div
            style={{
              fontSize: 18,
              marginBottom: 6,
              display: "flex",
              alignItems: "center",
              gap: 8,
            }}
          >
            <Folder size={18} style={{ flexShrink: 0 }} />
            <span>Open Folder</span>
          </div>
          <div style={{ fontSize: 12, color: "#888" }}>
            Browse and edit packages, launch files, and URDF models.
          </div>
        </div>

        <div
          onClick={onSelectWorkspace}
          style={{
            background: "#252526",
            border: "1px solid #3c3c3c",
            borderRadius: 6,
            padding: 16,
            cursor: "pointer",
            textAlign: "left",
            transition: "all 0.15s ease",
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.borderColor = "#4fc3f7";
            e.currentTarget.style.background = "#2d2d30";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.borderColor = "#3c3c3c";
            e.currentTarget.style.background = "#252526";
          }}
        >
          <div
            style={{
              fontSize: 18,
              marginBottom: 6,
              display: "flex",
              alignItems: "center",
              gap: 8,
            }}
          >
            <Bot size={18} style={{ flexShrink: 0 }} />
            <span>
              {rosWorkspacePath ? "Change Workspace" : "Select ROS Workspace"}
            </span>
          </div>
          <div style={{ fontSize: 12, color: "#888" }}>
            {rosWorkspacePath
              ? `Active: ${rosWorkspacePath.split("/").pop()}`
              : "Connect your workspace to enable builds & Gazebo simulations."}
          </div>
        </div>
      </div>

      <div style={{ fontSize: 12, color: "#666", display: "flex", gap: 20 }}>
        <span>
          <kbd style={kbdStyle}>⌘S</kbd> Save File
        </span>
        <span>
          <kbd style={kbdStyle}>⌘B</kbd> Toggle Sidebar
        </span>
        <span>
          <kbd style={kbdStyle}>⌘`</kbd> Toggle Bottom Panel
        </span>
        <span>
          <kbd style={kbdStyle}>⌘W</kbd> Close Tab
        </span>
      </div>
    </div>
  );
};

const kbdStyle: React.CSSProperties = {
  background: "#2d2d30",
  border: "1px solid #444",
  borderRadius: 3,
  padding: "1px 5px",
  fontSize: 11,
  color: "#aaa",
};

export default EmptyState;
