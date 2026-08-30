import { useEffect, useState, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { AlertTriangle, Sparkles } from "lucide-react";

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

type TfUpdateWithValidation = TfUpdate & {
  validationIssues?: string[];
};

interface Props {
  onExplain?: (tf: TfUpdateWithValidation) => void;
}

const QUATERNION_TOLERANCE = 0.01;
const TRANSLATION_WARNING_THRESHOLD = 1000;

function validateTfUpdate(tf: TfUpdate): string[] {
  const issues: string[] = [];
  const values = [tf.x, tf.y, tf.z, tf.qx, tf.qy, tf.qz, tf.qw];

  if (values.some((value) => !Number.isFinite(value))) {
    issues.push("Contains NaN/Infinity");
  }

  const quaternionNorm =
    tf.qx * tf.qx + tf.qy * tf.qy + tf.qz * tf.qz + tf.qw * tf.qw;

  if (Math.abs(quaternionNorm) < 1e-12) {
    issues.push("Zero quaternion — likely uninitialized");
  } else if (Math.abs(quaternionNorm - 1) > QUATERNION_TOLERANCE) {
    issues.push("Non-normalized quaternion");
  }

  if (
    Math.abs(tf.x) > TRANSLATION_WARNING_THRESHOLD ||
    Math.abs(tf.y) > TRANSLATION_WARNING_THRESHOLD ||
    Math.abs(tf.z) > TRANSLATION_WARNING_THRESHOLD
  ) {
    issues.push("Large translation magnitude");
  }

  return issues;
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
  const byParent: Record<string, TfUpdateWithValidation[]> = {};
  Object.values(transforms).forEach((t) => {
    const withValidation: TfUpdateWithValidation = {
      ...t,
      validationIssues: validateTfUpdate(t),
    };

    if (!byParent[t.parent_frame]) byParent[t.parent_frame] = [];
    byParent[t.parent_frame].push(withValidation);
  });

  const roots = Object.keys(byParent);

  return (
    <div
      style={{
        padding: "var(--spacing-base)",
        color: "#ccc",
        fontSize: "var(--font-xs)",
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
          {byParent[parent].map((t) => {
            const issues = t.validationIssues ?? [];
            const issueSummary = issues.length > 0 ? issues.join(" • ") : "";

            return (
              <div
                key={t.child_frame}
                style={{ paddingLeft: 20, opacity: 0.9 }}
              >
                └─ <span style={{ color: "#7cd992" }}>{t.child_frame}</span>
                {issues.length > 0 && (
                  <span
                    title={issueSummary}
                    style={{
                      marginLeft: 8,
                      color: "#e08080",
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 4,
                      cursor: "help",
                    }}
                  >
                    <AlertTriangle
                      style={{
                        width: "var(--icon-sm)",
                        height: "var(--icon-sm)",
                        verticalAlign: "middle",
                      }}
                    />
                    <span style={{ opacity: 0.9 }}>{issues[0]}</span>
                  </span>
                )}
                {onExplain && (
                  <span
                    onClick={() =>
                      onExplain({ ...t, validationIssues: issues })
                    }
                    title="Explain this frame with AI"
                    style={{
                      marginLeft: "var(--spacing-sm)",
                      fontSize: "var(--font-xs)",
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
                      style={{
                        width: "var(--icon-sm)",
                        height: "var(--icon-sm)",
                        marginRight: 4,
                        display: "inline",
                      }}
                    />{" "}
                    Explain
                  </span>
                )}
                <div
                  style={{
                    paddingLeft: 20,
                    opacity: 0.6,
                    fontSize: "var(--font-xs)",
                  }}
                >
                  pos: ({t.x.toFixed(3)}, {t.y.toFixed(3)}, {t.z.toFixed(3)})
                </div>
              </div>
            );
          })}
        </div>
      ))}
    </div>
  );
}
