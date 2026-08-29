import { useEffect, useState, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { Sparkles } from "lucide-react";

interface TfUpdate {
  parent_frame: string;
  child_frame: string;
  x: number;
  y: number;
  z: number;
  qx: number;
  qy: number;
  qz: number;
  qw: number;
}

interface Props {
  onExplain?: (tf: TfUpdate) => void;
}

export default function TFTree({ onExplain }: Props) {
  const [transforms, setTransforms] = useState<Record<string, TfUpdate>>({});
  const isActiveRef = useRef(true);

  useEffect(() => {
    isActiveRef.current = true;
    const unlisten = listen<TfUpdate>("tf-update", (event) => {
      if (!isActiveRef.current) return;
      const key = `${event.payload.parent_frame}->${event.payload.child_frame}`;
      setTransforms((prev) => ({ ...prev, [key]: event.payload }));
    });

    return () => {
      isActiveRef.current = false;
      unlisten.then((f) => f());
    };
  }, []);

  // Group by parent frame, so we can render a simple indented tree
  const byParent: Record<string, TfUpdate[]> = {};
  Object.values(transforms).forEach((t) => {
    if (!byParent[t.parent_frame]) byParent[t.parent_frame] = [];
    byParent[t.parent_frame].push(t);
  });

  const roots = Object.keys(byParent);

  return (
    <div
      style={{
        padding: 12,
        color: "#ccc",
        fontSize: 12,
        fontFamily: "monospace",
        overflowY: "auto",
        height: "100%",
      }}
    >
      {roots.length === 0 && (
        <div style={{ opacity: 0.5 }}>
          No transforms detected yet — start a ROS node that publishes to /tf.
        </div>
      )}
      {roots.map((parent) => (
        <div key={parent} style={{ marginBottom: 10 }}>
          <div style={{ color: "#4fc3f7", fontWeight: 600 }}>{parent}</div>
          {byParent[parent].map((t) => (
            <div key={t.child_frame} style={{ paddingLeft: 20, opacity: 0.9 }}>
              └─ <span style={{ color: "#7cd992" }}>{t.child_frame}</span>
              {onExplain && (
                <span
                  onClick={() => onExplain(t)}
                  title="Explain this frame with AI"
                  style={{
                    marginLeft: 8,
                    fontSize: 10,
                    color: "#4fc3f7",
                    opacity: 0.8,
                    cursor: "pointer",
                    transition: "opacity 0.12s ease",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.opacity = "1";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.opacity = "0.8";
                  }}
                >
                  <Sparkles
                    size={12}
                    style={{ marginRight: 4, display: "inline" }}
                  />{" "}
                  Explain
                </span>
              )}
              <div style={{ paddingLeft: 20, opacity: 0.6, fontSize: 11 }}>
                pos: ({t.x.toFixed(3)}, {t.y.toFixed(3)}, {t.z.toFixed(3)})
              </div>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
