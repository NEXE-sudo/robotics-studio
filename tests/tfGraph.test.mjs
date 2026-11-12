import assert from "node:assert/strict";
import test from "node:test";
import {
  buildTfGraph,
  composeTfTransform,
  mergeTfTransform,
} from "../src/tfGraph.ts";

function transform(parent, child, overrides = {}) {
  return {
    parent_frame: parent,
    child_frame: child,
    topic_name: "/tf",
    x: 0,
    y: 0,
    z: 0,
    qx: 0,
    qy: 0,
    qz: 0,
    qw: 1,
    receivedAtMs: 10000,
    source_timestamp_ms: 9000,
    ...overrides,
  };
}

test("builds a rooted parent-child hierarchy", () => {
  const graph = buildTfGraph(
    [transform("map", "base_link"), transform("base_link", "laser")],
    10000,
  );
  assert.equal(graph.roots.length, 1);
  assert.equal(graph.roots[0].frame, "map");
  assert.equal(graph.roots[0].children[0].frame, "base_link");
  assert.equal(graph.roots[0].children[0].children[0].frame, "laser");
  assert.deepEqual(graph.warnings, []);
});

test("reports multiple disconnected roots and multiple parents", () => {
  const graph = buildTfGraph(
    [
      transform("map_a", "base_link"),
      transform("map_b", "base_link"),
      transform("map_c", "camera"),
    ],
    10000,
  );
  assert.equal(graph.roots.length, 3);
  assert.ok(graph.warnings.some((warning) => warning.includes("disconnected")));
  assert.ok(graph.edges[0].issues.includes("Frame has multiple parents"));
  assert.ok(graph.edges[1].issues.includes("Frame has multiple parents"));
});

test("detects cycles and keeps cyclic frames visible as unattached", () => {
  const graph = buildTfGraph(
    [transform("a", "b"), transform("b", "c"), transform("c", "a")],
    10000,
  );
  assert.ok(graph.warnings.some((warning) => warning.includes("cycle")));
  assert.ok(graph.edges.some((edge) => edge.issues.includes("Cycle detected in transform graph")));
  assert.deepEqual(graph.unattachedFrames, ["a", "b", "c"]);
});

test("marks old dynamic transforms stale but exempts static transforms", () => {
  const graph = buildTfGraph(
    [
      transform("map", "dynamic", { receivedAtMs: 0 }),
      transform("map", "fixed", {
        topic_name: "/tf_static",
        receivedAtMs: 0,
      }),
    ],
    10000,
  );
  assert.ok(graph.edges[0].issues.includes("Transform is stale"));
  assert.ok(!graph.edges[1].issues.includes("Transform is stale"));
});

test("uses receive time for staleness and preserves the ROS source timestamp", () => {
  const graph = buildTfGraph(
    [
      transform("map", "replayed", {
        receivedAtMs: 0,
        source_timestamp_ms: 9000,
      }),
    ],
    10000,
  );
  assert.ok(graph.edges[0].issues.includes("Transform is stale"));
  assert.equal(graph.edges[0].source_timestamp_ms, 9000);
});

test("flags future source timestamps and invalid receive stamps during reconnect", () => {
  const graph = buildTfGraph(
    [
      transform("map", "future", {
        receivedAtMs: 1000,
        source_timestamp_ms: 20000,
      }),
      transform("map", "invalid", {
        receivedAtMs: Number.NaN,
        source_timestamp_ms: 9000,
      }),
    ],
    10000,
  );
  assert.ok(graph.edges[0].issues.includes("Source timestamp is in the future"));
  assert.ok(graph.edges[1].issues.includes("Contains an invalid receive timestamp"));
});

test("marks dynamic frames with no recent updates as disappeared", () => {
  const graph = buildTfGraph(
    [transform("map", "lost_frame", { receivedAtMs: 0 })],
    40000,
  );
  assert.ok(graph.edges[0].issues.includes("Transform is stale"));
  assert.ok(
    graph.edges[0].issues.includes(
      "No transform update received for 30 seconds",
    ),
  );
});

test("replaces a stale dynamic parent when a child changes parent", () => {
  const previous = transform("map", "base_link", { receivedAtMs: 1000 });
  const changed = transform("odom", "base_link", { receivedAtMs: 7000 });
  const merged = mergeTfTransform(
    { [JSON.stringify(["map", "base_link"])]: previous },
    changed,
  );
  assert.deepEqual(Object.values(merged), [changed]);

  const conflict = mergeTfTransform(
    { [JSON.stringify(["map", "base_link"])]: previous },
    transform("odom", "base_link", { receivedAtMs: 2000 }),
  );
  assert.equal(Object.keys(conflict).length, 2);
});

test("composes chained transforms with parent rotation", () => {
  const halfSqrt = Math.SQRT1_2;
  const graph = buildTfGraph(
    [
      transform("map", "base_link", {
        x: 1,
        qz: halfSqrt,
        qw: halfSqrt,
      }),
      transform("base_link", "laser", { x: 1 }),
    ],
    10000,
  );
  const composition = composeTfTransform(graph.edges, "laser");
  assert.equal(composition.ok, true);
  if (!composition.ok) return;
  assert.equal(composition.pose.rootFrame, "map");
  assert.ok(Math.abs(composition.pose.x - 1) < 1e-9);
  assert.ok(Math.abs(composition.pose.y - 1) < 1e-9);
  assert.ok(Math.abs(composition.pose.qz - halfSqrt) < 1e-9);
  assert.ok(Math.abs(composition.pose.qw - halfSqrt) < 1e-9);
});

test("refuses composed poses with ambiguous or invalid edges", () => {
  const ambiguous = buildTfGraph(
    [transform("map_a", "base"), transform("map_b", "base")],
    10000,
  );
  const ambiguousResult = composeTfTransform(ambiguous.edges, "base");
  assert.equal(ambiguousResult.ok, false);

  const invalid = buildTfGraph(
    [transform("map", "base", { qx: 0, qy: 0, qz: 0, qw: 0 })],
    10000,
  );
  const invalidResult = composeTfTransform(invalid.edges, "base");
  assert.equal(invalidResult.ok, false);
});

test("reports invalid quaternions and non-finite transforms", () => {
  const graph = buildTfGraph(
    [
      transform("map", "zero", { qx: 0, qy: 0, qz: 0, qw: 0 }),
      transform("map", "invalid", { x: Number.NaN }),
      transform("map", "bad_timestamp", { receivedAtMs: Number.NaN }),
    ],
    10000,
  );
  assert.ok(graph.edges[0].issues.includes("Zero quaternion — likely uninitialized"));
  assert.ok(graph.edges[1].issues.includes("Contains NaN or Infinity"));
  assert.ok(graph.edges[2].issues.includes("Contains an invalid timestamp"));
});
