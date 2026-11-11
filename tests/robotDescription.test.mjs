import assert from "node:assert/strict";
import test from "node:test";
import { buildRobotInstance } from "../src/robotDescription.ts";

const origin = { xyz: [0, 0, 0], rpy: [0, 0, 0] };

test("builds primitive robot links and applies revolute and prismatic joint states", () => {
  const instance = buildRobotInstance({
    name: "test_robot",
    rootLink: "base_link",
    warnings: [],
    links: [
      {
        name: "base_link",
        visuals: [
          {
            origin,
            geometry: { kind: "box", size: [1, 0.5, 0.25] },
            color: [0.2, 0.4, 0.8, 1],
          },
        ],
        collisions: [],
      },
      { name: "arm_link", visuals: [], collisions: [] },
      { name: "slider_link", visuals: [], collisions: [] },
    ],
    joints: [
      {
        name: "shoulder",
        jointType: "revolute",
        parent: "base_link",
        child: "arm_link",
        origin: { xyz: [0, 0, 0.5], rpy: [0, 0, 0] },
        axis: [0, 0, 1],
        limit: null,
        mimic: null,
      },
      {
        name: "slider",
        jointType: "prismatic",
        parent: "arm_link",
        child: "slider_link",
        origin,
        axis: [1, 0, 0],
        limit: null,
        mimic: null,
      },
    ],
  });

  const arm = instance.group.getObjectByName("arm_link");
  const shoulder = instance.group.getObjectByName("shoulder:motion");
  const slider = instance.group.getObjectByName("slider:motion");
  assert.ok(arm);
  assert.ok(shoulder);
  assert.ok(slider);

  instance.updateJoints(
    new Map([
      ["shoulder", Math.PI / 2],
      ["slider", 0.25],
    ]),
  );

  assert.ok(Math.abs(shoulder.rotation.z - Math.PI / 2) < 1e-8);
  assert.equal(slider.position.x, 0.25);
  assert.equal(instance.group.getObjectByName("base_link").children.length, 2);
});

test("skips unsupported mesh visuals without failing the whole model", () => {
  const instance = buildRobotInstance({
    name: "mesh_robot",
    rootLink: "base_link",
    warnings: ["Mesh geometry 'package://robot/meshes/base.stl' is not rendered"],
    links: [
      {
        name: "base_link",
        visuals: [
          {
            origin,
            geometry: {
              kind: "unsupported",
              description: "package://robot/meshes/base.stl",
            },
            color: null,
          },
        ],
        collisions: [],
      },
    ],
    joints: [],
  });

  assert.ok(instance.group.getObjectByName("base_link"));
  assert.equal(instance.group.getObjectByName("base_link").children.length, 0);
});

test("clamps joint limits, resolves mimic joints, and toggles collision geometry", () => {
  const instance = buildRobotInstance({
    name: "bounded_robot",
    rootLink: "base",
    warnings: [],
    links: [
      { name: "base", visuals: [], collisions: [] },
      { name: "driver_link", visuals: [], collisions: [] },
      {
        name: "follower_link",
        visuals: [],
        collisions: [
          {
            origin,
            geometry: { kind: "box", size: [1, 1, 1] },
            color: null,
          },
        ],
      },
    ],
    joints: [
      {
        name: "driver",
        jointType: "revolute",
        parent: "base",
        child: "driver_link",
        origin,
        axis: [0, 0, 1],
        limit: { lower: -1, upper: 1, effort: 2, velocity: 3 },
        mimic: null,
      },
      {
        name: "follower",
        jointType: "revolute",
        parent: "driver_link",
        child: "follower_link",
        origin,
        axis: [0, 0, 1],
        limit: null,
        mimic: { joint: "driver", multiplier: -2, offset: 0.25 },
      },
    ],
  });
  const driver = instance.group.getObjectByName("driver:motion");
  const follower = instance.group.getObjectByName("follower:motion");
  const collision = instance.group.getObjectByName("follower_link").children[0];
  assert.ok(driver);
  assert.ok(follower);
  assert.equal(collision.visible, false);

  instance.updateJoints(new Map([["driver", 2]]));
  assert.ok(Math.abs(driver.rotation.z - 1) < 1e-8);
  assert.ok(Math.abs(follower.rotation.z - (-1.75)) < 1e-8);
  instance.setCollisionVisible(true);
  assert.equal(collision.visible, true);
});
