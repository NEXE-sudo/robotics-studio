import assert from "node:assert/strict";
import test from "node:test";
import {
  canLoadRobotDescription,
  defaultServiceRequest,
  describeEventRecovery,
  eventRecoveryActionLabel,
  nodePath,
  shouldResetEventRecoveryAcknowledgement,
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
    retainedCriticalEvents: 6,
  };

  assert.equal(
    eventRecoveryActionLabel(recovery),
    "Acknowledge recovery gap · appears from 42 to 97 · retained critical events 6",
  );
  assert.match(
    describeEventRecovery(recovery),
    /Retained critical events: 6\./,
  );
  assert.equal(describeEventRecovery({ ...recovery, gapDetected: false }), null);
});

test("event recovery acknowledgements reset only for a new or resolved gap", () => {
  const recovery = {
    gapDetected: true,
    oldestAvailableSequence: 42,
    latestSequence: 97,
    snapshotIncluded: true,
    retainedCriticalEvents: 6,
  };

  assert.equal(shouldResetEventRecoveryAcknowledgement(null, recovery), true);
  assert.equal(
    shouldResetEventRecoveryAcknowledgement(recovery, {
      ...recovery,
      latestSequence: 98,
    }),
    true,
  );
  assert.equal(
    shouldResetEventRecoveryAcknowledgement(recovery, {
      ...recovery,
      retainedCriticalEvents: 6,
    }),
    false,
  );
  assert.equal(
    shouldResetEventRecoveryAcknowledgement(recovery, {
      ...recovery,
      gapDetected: false,
    }),
    true,
  );
});
