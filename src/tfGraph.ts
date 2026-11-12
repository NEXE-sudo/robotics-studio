export interface TfTransform {
  parent_frame: string;
  child_frame: string;
  topic_name: string;
  x: number;
  y: number;
  z: number;
  qx: number;
  qy: number;
  qz: number;
  qw: number;
  receivedAtMs: number;
  source_timestamp_ms: number;
}

export interface TfGraphEdge extends TfTransform {
  key: string;
  issues: string[];
}

export interface TfTreeNode {
  frame: string;
  edge?: TfGraphEdge;
  children: TfTreeNode[];
}

export interface TfGraph {
  roots: TfTreeNode[];
  unattachedFrames: string[];
  edges: TfGraphEdge[];
  warnings: string[];
}

export interface TfComposedPose {
  rootFrame: string;
  frame: string;
  x: number;
  y: number;
  z: number;
  qx: number;
  qy: number;
  qz: number;
  qw: number;
  stale: boolean;
}

export type TfComposition =
  | { ok: true; pose: TfComposedPose }
  | { ok: false; reason: string };

const QUATERNION_TOLERANCE = 0.01;
const TRANSLATION_WARNING_THRESHOLD = 1000;
const SOURCE_DISAPPEARED_AFTER_MS = 30_000;

export function tfEdgeKey(parentFrame: string, childFrame: string) {
  return JSON.stringify([parentFrame, childFrame]);
}

export function mergeTfTransform(
  existing: Record<string, TfTransform>,
  incoming: TfTransform,
  staleAfterMs = 5000,
): Record<string, TfTransform> {
  const updated = { ...existing };
  for (const [key, transform] of Object.entries(updated)) {
    if (
      transform.child_frame === incoming.child_frame &&
      transform.parent_frame !== incoming.parent_frame &&
      !transform.topic_name.endsWith("/tf_static") &&
      incoming.receivedAtMs - transform.receivedAtMs > staleAfterMs
    ) {
      delete updated[key];
    }
  }
  updated[tfEdgeKey(incoming.parent_frame, incoming.child_frame)] = incoming;
  return updated;
}

function normalizeQuaternion(
  quaternion: [number, number, number, number],
): [number, number, number, number] {
  const norm = Math.hypot(...quaternion);
  return [
    quaternion[0] / norm,
    quaternion[1] / norm,
    quaternion[2] / norm,
    quaternion[3] / norm,
  ];
}

function multiplyQuaternions(
  first: [number, number, number, number],
  second: [number, number, number, number],
): [number, number, number, number] {
  const [x1, y1, z1, w1] = first;
  const [x2, y2, z2, w2] = second;
  return [
    w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
    w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
    w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
    w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
  ];
}

function rotateVector(
  quaternion: [number, number, number, number],
  vector: [number, number, number],
): [number, number, number] {
  const [x, y, z, w] = quaternion;
  const [vx, vy, vz] = vector;
  const tx = 2 * (y * vz - z * vy);
  const ty = 2 * (z * vx - x * vz);
  const tz = 2 * (x * vy - y * vx);
  return [
    vx + w * tx + y * tz - z * ty,
    vy + w * ty + z * tx - x * tz,
    vz + w * tz + x * ty - y * tx,
  ];
}

export function composeTfTransform(
  edges: TfGraphEdge[],
  targetFrame: string,
): TfComposition {
  if (!targetFrame) return { ok: false, reason: "Target frame is empty" };
  const path: TfGraphEdge[] = [];
  const visited = new Set<string>();
  const parentsByChild = new Map<string, TfGraphEdge[]>();
  for (const edge of edges) {
    const parents = parentsByChild.get(edge.child_frame) ?? [];
    parents.push(edge);
    parentsByChild.set(edge.child_frame, parents);
  }
  let frame = targetFrame;

  while (true) {
    if (visited.has(frame)) {
      return { ok: false, reason: "Transform path contains a cycle" };
    }
    visited.add(frame);
    const parents = parentsByChild.get(frame) ?? [];
    if (parents.length > 1) {
      return { ok: false, reason: `Frame ${frame} has multiple parents` };
    }
    if (parents.length === 0) break;

    const edge = parents[0];
    const blockingIssue = edge.issues.find(
      (issue) =>
        issue !== "Transform is stale" &&
        issue !== "No transform update received for 30 seconds" &&
        issue !== "Large translation magnitude",
    );
    if (blockingIssue) {
      return { ok: false, reason: `${edge.child_frame}: ${blockingIssue}` };
    }
    path.push(edge);
    frame = edge.parent_frame;
  }

  let translation: [number, number, number] = [0, 0, 0];
  let rotation: [number, number, number, number] = [0, 0, 0, 1];
  let stale = false;
  for (const edge of path.reverse()) {
    const edgeRotation = normalizeQuaternion([
      edge.qx,
      edge.qy,
      edge.qz,
      edge.qw,
    ]);
    const rotatedTranslation = rotateVector(rotation, [
      edge.x,
      edge.y,
      edge.z,
    ]);
    translation = [
      translation[0] + rotatedTranslation[0],
      translation[1] + rotatedTranslation[1],
      translation[2] + rotatedTranslation[2],
    ];
    rotation = normalizeQuaternion(multiplyQuaternions(rotation, edgeRotation));
    stale ||= edge.issues.includes("Transform is stale") ||
      edge.issues.includes("No transform update received for 30 seconds");
  }

  return {
    ok: true,
    pose: {
      rootFrame: frame,
      frame: targetFrame,
      x: translation[0],
      y: translation[1],
      z: translation[2],
      qx: rotation[0],
      qy: rotation[1],
      qz: rotation[2],
      qw: rotation[3],
      stale,
    },
  };
}

export function buildTfGraph(
  transforms: TfTransform[],
  nowMs = Date.now(),
  staleAfterMs = 5000,
): TfGraph {
  const edges = transforms.map((transform): TfGraphEdge => {
    const key = tfEdgeKey(transform.parent_frame, transform.child_frame);
    const issues: string[] = [];
    const values = [
      transform.x,
      transform.y,
      transform.z,
      transform.qx,
      transform.qy,
      transform.qz,
      transform.qw,
    ];

    if (!transform.parent_frame || !transform.child_frame) {
      issues.push("Parent or child frame is empty");
    }
    if (values.some((value) => !Number.isFinite(value))) {
      issues.push("Contains NaN or Infinity");
    }
    if (
      !Number.isFinite(transform.receivedAtMs) ||
      !Number.isFinite(transform.source_timestamp_ms)
    ) {
      issues.push("Contains an invalid timestamp");
    }

    const quaternionNorm = Math.hypot(
      transform.qx,
      transform.qy,
      transform.qz,
      transform.qw,
    );
    if (quaternionNorm < 1e-12) {
      issues.push("Zero quaternion — likely uninitialized");
    } else if (Math.abs(quaternionNorm - 1) > QUATERNION_TOLERANCE) {
      issues.push("Non-normalized quaternion");
    }

    if (
      Math.abs(transform.x) > TRANSLATION_WARNING_THRESHOLD ||
      Math.abs(transform.y) > TRANSLATION_WARNING_THRESHOLD ||
      Math.abs(transform.z) > TRANSLATION_WARNING_THRESHOLD
    ) {
      issues.push("Large translation magnitude");
    }

    if (!transform.topic_name.endsWith("/tf_static")) {
      const age = nowMs - transform.receivedAtMs;
      if (age > staleAfterMs) issues.push("Transform is stale");
      if (age > SOURCE_DISAPPEARED_AFTER_MS) {
        issues.push("No transform update received for 30 seconds");
      }
    }

    return { ...transform, key, issues };
  });

  const frameNames = new Set<string>();
  const parentsByChild = new Map<string, Set<string>>();
  const childrenByParent = new Map<string, TfGraphEdge[]>();
  for (const edge of edges) {
    if (!edge.parent_frame || !edge.child_frame) continue;
    frameNames.add(edge.parent_frame);
    frameNames.add(edge.child_frame);
    const parents = parentsByChild.get(edge.child_frame) ?? new Set<string>();
    parents.add(edge.parent_frame);
    parentsByChild.set(edge.child_frame, parents);
    const children = childrenByParent.get(edge.parent_frame) ?? [];
    children.push(edge);
    childrenByParent.set(edge.parent_frame, children);
  }

  for (const edge of edges) {
    const parents = parentsByChild.get(edge.child_frame);
    if (parents && parents.size > 1) {
      edge.issues.push("Frame has multiple parents");
    }
  }

  const visitState = new Map<string, "visiting" | "visited">();
  const stack: string[] = [];
  const cycleEdges = new Set<string>();
  const cycleFrames = new Set<string>();
  const visit = (frame: string) => {
    visitState.set(frame, "visiting");
    stack.push(frame);
    for (const edge of childrenByParent.get(frame) ?? []) {
      if (visitState.get(edge.child_frame) === "visiting") {
        cycleEdges.add(edge.key);
        const cycleStart = stack.lastIndexOf(edge.child_frame);
        for (const cycleFrame of stack.slice(cycleStart)) {
          cycleFrames.add(cycleFrame);
        }
      } else if (!visitState.has(edge.child_frame)) {
        visit(edge.child_frame);
      }
    }
    stack.pop();
    visitState.set(frame, "visited");
  };
  for (const frame of [...frameNames].sort()) {
    if (!visitState.has(frame)) visit(frame);
  }
  for (const edge of edges) {
    if (cycleEdges.has(edge.key)) {
      edge.issues.push("Cycle detected in transform graph");
    }
  }

  const roots = [...frameNames]
    .filter((frame) => !parentsByChild.has(frame))
    .sort();
  const visitedFrames = new Set<string>();
  const buildTree = (frame: string, ancestors: Set<string>): TfTreeNode => {
    visitedFrames.add(frame);
    const nextAncestors = new Set(ancestors);
    nextAncestors.add(frame);
    const children = (childrenByParent.get(frame) ?? [])
      .filter((edge) => !nextAncestors.has(edge.child_frame))
      .map((edge) => ({
        frame: edge.child_frame,
        edge,
        children: buildTree(edge.child_frame, nextAncestors).children,
      }));
    return { frame, children };
  };
  const treeRoots = roots.map((root) => buildTree(root, new Set()));
  const unattachedFrames = [...frameNames]
    .filter((frame) => !visitedFrames.has(frame))
    .sort();
  const warnings: string[] = [];
  if (treeRoots.length > 1) {
    warnings.push(`TF contains ${treeRoots.length} disconnected root frames`);
  }
  if (cycleFrames.size > 0) {
    warnings.push(
      `TF contains a cycle involving: ${[...cycleFrames].sort().join(", ")}`,
    );
  }
  if (edges.some((edge) => edge.issues.includes("Frame has multiple parents"))) {
    warnings.push("One or more frames have multiple parents");
  }

  return { roots: treeRoots, unattachedFrames, edges, warnings };
}
