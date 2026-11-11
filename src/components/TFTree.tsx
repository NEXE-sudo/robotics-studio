import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { AlertTriangle, Sparkles } from "lucide-react";
import {
  buildTfGraph,
  tfEdgeKey,
  type TfTransform,
  type TfTreeNode,
} from "../tfGraph";

type TfUpdateWithValidation = TfTransform & {
  validationIssues: string[];
};

interface TfEvent extends Omit<TfTransform, "receivedAtMs"> {}

interface Props {
  onExplain?: (tf: TfUpdateWithValidation) => void;
}

function TreeBranch({
  node,
  depth,
  selectedEdgeKey,
  onSelect,
}: {
  node: TfTreeNode;
  depth: number;
  selectedEdgeKey: string | null;
  onSelect: (key: string) => void;
}) {
  const issues = node.edge?.issues ?? [];
  return (
    <div key={node.edge?.key ?? node.frame}>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 5,
          paddingLeft: depth * 14,
          minHeight: 24,
        }}
      >
        {depth > 0 && <span style={{ opacity: 0.45 }}>└─</span>}
        {node.edge ? (
          <button
            type="button"
            onClick={() => onSelect(node.edge!.key)}
            aria-pressed={selectedEdgeKey === node.edge.key}
            style={{
              textAlign: "left",
              padding: "2px 4px",
              border: 0,
              borderRadius: 3,
              color: issues.length ? "#e0b060" : "#7cd992",
              background:
                selectedEdgeKey === node.edge.key
                  ? "rgba(79,195,247,0.12)"
                  : "transparent",
              fontFamily: "inherit",
              fontSize: "inherit",
              cursor: "pointer",
            }}
          >
            {node.frame}
          </button>
        ) : (
          <span style={{ color: "#4fc3f7", fontWeight: 600 }}>
            {node.frame}
          </span>
        )}
        {issues.length > 0 && (
          <span title={issues.join("; ")}>
            <AlertTriangle
              aria-label={issues.join("; ")}
              size={13}
              color="#e0b060"
            />
          </span>
        )}
      </div>
      {node.children.map((child) => (
        <TreeBranch
          key={child.edge?.key ?? child.frame}
          node={child}
          depth={depth + 1}
          selectedEdgeKey={selectedEdgeKey}
          onSelect={onSelect}
        />
      ))}
    </div>
  );
}

function quaternionEulerDegrees(transform: TfTransform) {
  const { qx, qy, qz, qw } = transform;
  if (![qx, qy, qz, qw].every(Number.isFinite)) return null;
  const norm = Math.hypot(qx, qy, qz, qw);
  if (norm < 1e-12) return null;
  const x = qx / norm;
  const y = qy / norm;
  const z = qz / norm;
  const w = qw / norm;
  const sinRoll = 2 * (w * x + y * z);
  const cosRoll = 1 - 2 * (x * x + y * y);
  const sinPitch = 2 * (w * y - z * x);
  const sinYaw = 2 * (w * z + x * y);
  const cosYaw = 1 - 2 * (y * y + z * z);
  const degrees = 180 / Math.PI;
  return {
    roll: Math.atan2(sinRoll, cosRoll) * degrees,
    pitch: Math.asin(Math.max(-1, Math.min(1, sinPitch))) * degrees,
    yaw: Math.atan2(sinYaw, cosYaw) * degrees,
  };
}

export default function TFTree({ onExplain }: Props) {
  const [transforms, setTransforms] = useState<Record<string, TfTransform>>(
    {},
  );
  const transformsRef = useRef<Record<string, TfTransform>>({});
  const [selectedEdgeKey, setSelectedEdgeKey] = useState<string | null>(null);
  const [nowMs, setNowMs] = useState(() => Date.now());

  useEffect(() => {
    let active = true;
    let updateFrame: number | null = null;
    const listener = listen<TfEvent>("tf-update", (event) => {
      if (!active) return;
      const transform: TfTransform = {
        ...event.payload,
        receivedAtMs: Date.now(),
      };
      const key = tfEdgeKey(transform.parent_frame, transform.child_frame);
      transformsRef.current[key] = transform;
      if (updateFrame === null) {
        updateFrame = window.requestAnimationFrame(() => {
          updateFrame = null;
          if (active) setTransforms({ ...transformsRef.current });
        });
      }
    });
    const timer = window.setInterval(() => setNowMs(Date.now()), 1000);

    return () => {
      active = false;
      window.clearInterval(timer);
      if (updateFrame !== null) {
        window.cancelAnimationFrame(updateFrame);
      }
      listener.then((unlisten) => unlisten()).catch(() => undefined);
    };
  }, []);

  const graph = useMemo(
    () => buildTfGraph(Object.values(transforms), nowMs),
    [transforms, nowMs],
  );
  const selected =
    graph.edges.find((edge) => edge.key === selectedEdgeKey) ?? graph.edges[0];
  const euler = selected ? quaternionEulerDegrees(selected) : null;

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
      <div style={{ opacity: 0.65, marginBottom: 8 }}>
        {graph.edges.length} transform(s) · {graph.roots.length} root frame(s)
      </div>
      {graph.warnings.map((warning) => (
        <div
          key={warning}
          role="status"
          style={{
            display: "flex",
            alignItems: "center",
            gap: 6,
            color: "#e0b060",
            marginBottom: 6,
          }}
        >
          <AlertTriangle size={13} />
          {warning}
        </div>
      ))}
      {graph.edges.length === 0 && (
        <div style={{ opacity: 0.5 }}>
          No transforms detected yet — start a ROS node that publishes to /tf
          or /tf_static.
        </div>
      )}
      {graph.roots.map((root) => (
        <TreeBranch
          key={root.frame}
          node={root}
          depth={0}
          selectedEdgeKey={selected?.key ?? null}
          onSelect={setSelectedEdgeKey}
        />
      ))}
      {graph.unattachedFrames.length > 0 && (
        <div style={{ marginTop: 10 }}>
          <div style={{ color: "#e0b060", marginBottom: 4 }}>
            Cyclic or unreachable frames
          </div>
          {graph.unattachedFrames.map((frame) => (
            <div key={frame} style={{ color: "#e0b060", paddingLeft: 8 }}>
              {frame}
            </div>
          ))}
        </div>
      )}
      {selected && (
        <section
          aria-label="Transform inspector"
          style={{
            marginTop: 14,
            padding: 10,
            border: "1px solid rgba(255,255,255,0.12)",
            borderRadius: 5,
            background: "rgba(255,255,255,0.025)",
          }}
        >
          <div style={{ fontWeight: 600, color: "#4fc3f7", marginBottom: 6 }}>
            {selected.parent_frame} → {selected.child_frame}
          </div>
          <div style={{ opacity: 0.65, marginBottom: 6 }}>
            Source: {selected.topic_name || "unknown topic"}
            {selected.topic_name.endsWith("/tf_static")
              ? " · static"
              : ` · updated ${Math.max(
                  0,
                  Math.floor((nowMs - selected.receivedAtMs) / 1000),
                )}s ago`}
          </div>
          <div>
            Translation (m): ({selected.x.toFixed(4)}, {selected.y.toFixed(4)},{" "}
            {selected.z.toFixed(4)})
          </div>
          <div>
            Quaternion (x, y, z, w): ({selected.qx.toFixed(4)},{" "}
            {selected.qy.toFixed(4)}, {selected.qz.toFixed(4)},{" "}
            {selected.qw.toFixed(4)})
          </div>
          {euler && (
            <div style={{ marginTop: 4 }}>
              RPY (deg): ({euler.roll.toFixed(2)}, {euler.pitch.toFixed(2)},{" "}
              {euler.yaw.toFixed(2)})
            </div>
          )}
          {selected.issues.map((issue) => (
            <div
              key={issue}
              role="alert"
              style={{ color: "#e0b060", marginTop: 5 }}
            >
              {issue}
            </div>
          ))}
          {onExplain && (
            <button
              type="button"
              className="ide-btn"
              onClick={() =>
                onExplain({
                  ...selected,
                  validationIssues: selected.issues,
                })
              }
              style={{ marginTop: 8 }}
            >
              <Sparkles size={13} style={{ marginRight: 4 }} />
              Explain transform
            </button>
          )}
        </section>
      )}
    </div>
  );
}
