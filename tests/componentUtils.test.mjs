import assert from "node:assert/strict";
import test from "node:test";
import {
  canLoadRobotDescription,
  defaultServiceRequest,
  describeEventRecovery,
  eventRecoveryActionLabel,
  nodePath,
} from "../src/componentUtils.ts";

test("robot descriptions only load when content, path, and workspace are all present", () => {
  assert.equal(canLoadRobotDescription("<robot />", "/tmp/robot.urdf", "/tmp/ws"), true);
  assert.equal(canLoadRobotDescription(null, "/tmp/robot.urdf", "/tmp/ws"), false);
  assert.equal(canLoadRobotDescription("<robot />", null, "/tmp/ws"), false);
  assert.equal(canLoadRobotDescription("<robot />", "/tmp/robot.urdf", null), false);
});

test("node paths stay rooted and namespace-safe", () => {
  assert.equal(nodePath("wheel", "/"), "/wheel");
  assert.equal(nodePath("wheel", "/robot"), "/robot/wheel");
  assert.equal(nodePath("wheel", "/robot/"), "/robot/wheel");
});

test("common read-only service requests receive a safe default request body", () => {
  assert.equal(defaultServiceRequest("rcl_interfaces/srv/GetParameters"), '{"names":[]}');
  assert.equal(defaultServiceRequest("rcl_interfaces/srv/ListParameters"), '{"prefixes":[],"depth":0}');
  assert.equal(defaultServiceRequest("std_srvs/srv/Empty"), "{}");
});

test("event recovery summaries explain missing telemetry windows in the UI", () => {
  const recovery = {
    gapDetected: true,
    oldestAvailableSequence: 42,
    latestSequence: 97,
    snapshotIncluded: true,
  };

  assert.equal(
    eventRecoveryActionLabel(recovery),
    "Acknowledge recovery gap · appears from 42 to 97",
  );
  assert.match(
    describeEventRecovery(recovery),
    /Current state snapshots were restored\./,
  );
  assert.equal(describeEventRecovery({ ...recovery, gapDetected: false }), null);
});
