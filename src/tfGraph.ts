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

const QUATERNION_TOLERANCE = 0.01;
const TRANSLATION_WARNING_THRESHOLD = 1000;

export function tfEdgeKey(parentFrame: string, childFrame: string) {
  return JSON.stringify([parentFrame, childFrame]);
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

    if (
      !transform.topic_name.endsWith("/tf_static") &&
      nowMs - transform.receivedAtMs > staleAfterMs
    ) {
      issues.push("Transform is stale");
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
