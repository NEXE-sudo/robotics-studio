import assert from "node:assert/strict";
import test from "node:test";
import {
  appendLog,
  closeFileByPath,
  getNextFileAfterClose,
  normalizeRecentProjects,
  projectDisplayName,
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

test("falls back to a valid timestamp and a path-derived name when recent project metadata is missing or invalid", () => {
  const projects = [
    { path: "/tmp/no-name", name: "", last_opened: "not-a-timestamp" },
    { path: "/tmp/real", name: "", last_opened: "1700000100000" },
    { path: "/tmp/blank-timestamp", name: "blank", last_opened: "   " },
    { path: "   ", name: "   ", last_opened: "   " },
  ];

  const normalized = normalizeRecentProjects(projects);
  assert.equal(projectDisplayName("/tmp/no-name", ""), "no-name");
  assert.equal(projectDisplayName("  ", "   "), "Untitled project");
  assert.equal(normalized[0].name, "real");
  assert.equal(normalized[1].name, "no-name");
  assert.equal(normalized[2].name, "blank");
  assert.ok(Number(normalized[0].last_opened) > 0);
  assert.equal(Number(normalized[2].last_opened), 0);
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

test("closeFileByPath keeps the next active tab in sync with the reduced file list", () => {
  const files = [
    { path: "/a.ts", name: "a.ts" },
    { path: "/b.ts", name: "b.ts" },
    { path: "/c.ts", name: "c.ts" },
  ];

  const result = closeFileByPath(files, "/b.ts", "/b.ts");
  assert.deepEqual(
    result.files.map((file) => file.path),
    ["/a.ts", "/c.ts"],
  );
  assert.equal(result.activePath, "/c.ts");

  const resultTwo = closeFileByPath(files, "/d.ts", "/a.ts");
  assert.equal(resultTwo.activePath, "/d.ts");
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
