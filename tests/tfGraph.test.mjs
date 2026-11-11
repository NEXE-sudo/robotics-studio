import assert from "node:assert/strict";
import test from "node:test";
import { buildTfGraph } from "../src/tfGraph.ts";

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

test("reports invalid quaternions and non-finite transforms", () => {
  const graph = buildTfGraph(
    [
      transform("map", "zero", { qx: 0, qy: 0, qz: 0, qw: 0 }),
      transform("map", "invalid", { x: Number.NaN }),
    ],
    10000,
  );
  assert.ok(graph.edges[0].issues.includes("Zero quaternion — likely uninitialized"));
  assert.ok(graph.edges[1].issues.includes("Contains NaN or Infinity"));
});
