import React from "react";
import { X } from "lucide-react";

export interface Toast {
  id: string;
  type: "success" | "error" | "info" | "warning";
  message: string;
  details?: string;
}

interface ToastContainerProps {
  toasts: Toast[];
  onDismiss: (id: string) => void;
}

const ToastContainer: React.FC<ToastContainerProps> = ({
  toasts,
  onDismiss,
}) => {
  if (toasts.length === 0) return null;

  const getTypeStyle = (type: Toast["type"]) => {
    switch (type) {
      case "success":
        return {
          background: "#1e3a29",
          borderLeft: "4px solid #4caf50",
          color: "#e8f5e9",
        };
      case "error":
        return {
          background: "#3a1e1e",
          borderLeft: "4px solid #f44336",
          color: "#ffebee",
        };
      case "warning":
        return {
          background: "#3a321e",
          borderLeft: "4px solid #ff9800",
          color: "#fffde7",
        };
      case "info":
      default:
        return {
          background: "#1e2e3a",
          borderLeft: "4px solid #2196f3",
          color: "#e3f2fd",
        };
    }
  };

  return (
    <div
      style={{
        position: "fixed",
        bottom: 40,
        right: 20,
        zIndex: 9999,
        display: "flex",
        flexDirection: "column",
        gap: 8,
        maxWidth: 380,
        width: "100%",
        pointerEvents: "none",
      }}
    >
      {toasts.map((t) => (
        <div
          key={t.id}
          style={{
            ...getTypeStyle(t.type),
            padding: "var(--spacing-base) var(--spacing-md)",
            borderRadius: 4,
            boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
            fontSize: "var(--font-sm)",
            pointerEvents: "auto",
            display: "flex",
            flexDirection: "column",
            gap: 4,
            animation: "fadeIn 0.2s ease-in-out",
          }}
        >
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
            }}
          >
            <span style={{ fontWeight: 600 }}>{t.message}</span>
            <button
              onClick={() => onDismiss(t.id)}
              style={{
                background: "transparent",
                border: "none",
                color: "inherit",
                cursor: "pointer",
                padding: "0 4px",
                display: "flex",
                alignItems: "center",
                opacity: 0.7,
              }}
            >
              <X
                style={{ width: "var(--icon-sm)", height: "var(--icon-sm)" }}
              />
            </button>
          </div>
          {t.details && (
            <div
              style={{
                fontSize: "var(--font-xs)",
                opacity: 0.85,
                fontFamily: "monospace",
                wordBreak: "break-all",
              }}
            >
              {t.details}
            </div>
          )}
        </div>
      ))}
    </div>
  );
};

export default ToastContainer;
