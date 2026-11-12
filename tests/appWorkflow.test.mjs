import assert from "node:assert/strict";
import test from "node:test";
import {
  appendLog,
  getNextFileAfterClose,
  normalizeRecentProjects,
} from "../src/appWorkflow.ts";

test("keeps the newest recent project entry for each workspace path", () => {
  const projects = [
    { path: "/tmp/a", name: "alpha", last_opened: "1700000000000" },
    { path: "/tmp/b", name: "beta", last_opened: "1700000300000" },
    { path: "/tmp/a", name: "alpha-renamed", last_opened: "1700000400000" },
    { path: "/tmp/c", name: "gamma", last_opened: "1700000200000" },
  ];

  const normalized = normalizeRecentProjects(projects);
  assert.deepEqual(
    normalized.map((project) => project.path),
    ["/tmp/a", "/tmp/b", "/tmp/c"],
  );
  assert.equal(normalized[0].name, "alpha-renamed");
});

test("selects the adjacent tab after closing the active file", () => {
  const files = [
    { path: "/a.ts", name: "a.ts" },
    { path: "/b.ts", name: "b.ts" },
    { path: "/c.ts", name: "c.ts" },
  ];

  assert.equal(getNextFileAfterClose(files, "/b.ts", "/b.ts"), "/c.ts");
  assert.equal(getNextFileAfterClose(files, "/a.ts", "/a.ts"), "/b.ts");
  assert.equal(getNextFileAfterClose(files, "/c.ts", "/a.ts"), "/c.ts");
  assert.equal(getNextFileAfterClose(files, "/d.ts", "/a.ts"), "/d.ts");
});

test("caps log output by line count and total character budget", () => {
  const lines = Array.from({ length: 50 }, (_, index) => `line-${index}`);
  const result = appendLog(lines, "tail");
  assert.ok(result.length <= 1000);
  assert.equal(result[result.length - 1], "tail");

  const oversized = ["x".repeat(1500), "y".repeat(1500), "z".repeat(1500)];
  const trimmed = appendLog(oversized, "w");
  assert.ok(trimmed.every((entry) => entry.length <= 2000000));
  assert.equal(trimmed[trimmed.length - 1], "w");
});
