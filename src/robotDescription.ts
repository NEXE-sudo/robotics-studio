import * as THREE from "three";

export interface RobotOrigin {
  xyz: [number, number, number];
  rpy: [number, number, number];
}

export type RobotGeometry =
  | { kind: "box"; size: [number, number, number] }
  | { kind: "cylinder"; radius: number; length: number }
  | { kind: "sphere"; radius: number }
  | { kind: "unsupported"; description: string };

export interface RobotVisual {
  origin: RobotOrigin;
  geometry: RobotGeometry;
  color: [number, number, number, number] | null;
}

export interface RobotLink {
  name: string;
  visuals: RobotVisual[];
  collisions: RobotVisual[];
}

export interface RobotJointLimit {
  lower: number | null;
  upper: number | null;
  effort: number | null;
  velocity: number | null;
}

export interface RobotMimic {
  joint: string;
  multiplier: number;
  offset: number;
}

export interface RobotJoint {
  name: string;
  jointType: string;
  parent: string;
  child: string;
  origin: RobotOrigin;
  axis: [number, number, number];
  limit: RobotJointLimit | null;
  mimic: RobotMimic | null;
}

export interface RobotDescription {
  name: string;
  rootLink: string;
  links: RobotLink[];
  joints: RobotJoint[];
  warnings: string[];
}

export interface RobotInstance {
  group: THREE.Group;
  updateJoints: (positions: ReadonlyMap<string, number>) => void;
  setCollisionVisible: (visible: boolean) => void;
}

function createVisual(visual: RobotVisual, collision = false) {
  const { geometry, color } = visual;
  let shape: THREE.BufferGeometry;
  switch (geometry.kind) {
    case "box":
      shape = new THREE.BoxGeometry(...geometry.size);
      break;
    case "cylinder":
      shape = new THREE.CylinderGeometry(
        geometry.radius,
        geometry.radius,
        geometry.length,
        24,
      );
      break;
    case "sphere":
      shape = new THREE.SphereGeometry(geometry.radius, 24, 16);
      break;
    case "unsupported":
      return null;
  }

  const opacity = collision
    ? Math.min(color?.[3] ?? 0.35, 0.35)
    : color?.[3] ?? 1;
  const material = new THREE.MeshStandardMaterial({
    color: collision
      ? new THREE.Color(0xe34b4b)
      : color
      ? new THREE.Color(color[0], color[1], color[2])
      : new THREE.Color(0x80aaff),
    opacity,
    transparent: collision || opacity < 1,
    wireframe: collision,
  });
  const mesh = new THREE.Mesh(shape, material);
  if (geometry.kind === "cylinder") mesh.rotation.x = Math.PI / 2;
  mesh.castShadow = true;
  mesh.receiveShadow = true;

  const visualGroup = new THREE.Group();
  visualGroup.position.set(...visual.origin.xyz);
  visualGroup.rotation.set(...visual.origin.rpy, "XYZ");
  visualGroup.add(mesh);
  return visualGroup;
}

export function buildRobotInstance(
  description: RobotDescription,
): RobotInstance {
  const links = new Map(description.links.map((link) => [link.name, link]));
  const jointsByParent = new Map<string, RobotJoint[]>();
  for (const joint of description.joints) {
    const siblings = jointsByParent.get(joint.parent) ?? [];
    siblings.push(joint);
    jointsByParent.set(joint.parent, siblings);
  }

  const jointTransforms = new Map<
    string,
    { joint: RobotJoint; motion: THREE.Group }
  >();
  const collisionObjects: THREE.Object3D[] = [];
  const limitPosition = (joint: RobotJoint, position: number) =>
    Math.max(
      joint.limit?.lower ?? Number.NEGATIVE_INFINITY,
      Math.min(joint.limit?.upper ?? Number.POSITIVE_INFINITY, position),
    );

  const buildLink = (name: string): THREE.Group => {
    const definition = links.get(name);
    if (!definition) {
      throw new Error(`Robot description refers to missing link '${name}'`);
    }
    const link = new THREE.Group();
    link.name = name;
    for (const visual of definition.visuals) {
      const mesh = createVisual(visual);
      if (mesh) link.add(mesh);
    }
    for (const collision of definition.collisions) {
      const mesh = createVisual(collision, true);
      if (mesh) {
        mesh.visible = false;
        collisionObjects.push(mesh);
        link.add(mesh);
      }
    }

    for (const joint of jointsByParent.get(name) ?? []) {
      const origin = new THREE.Group();
      origin.name = `${joint.name}:origin`;
      origin.position.set(...joint.origin.xyz);
      origin.rotation.set(...joint.origin.rpy, "XYZ");
      const motion = new THREE.Group();
      motion.name = `${joint.name}:motion`;
      origin.add(motion);
      motion.add(buildLink(joint.child));
      link.add(origin);
      jointTransforms.set(joint.name, { joint, motion });
    }
    return link;
  };

  const group = new THREE.Group();
  group.name = description.name;
  const rosCoordinates = new THREE.Group();
  rosCoordinates.name = "ros-coordinate-frame";
  rosCoordinates.rotation.x = -Math.PI / 2;
  rosCoordinates.add(buildLink(description.rootLink));
  group.add(rosCoordinates);

  return {
    group,
    updateJoints(positions) {
      const resolvedPositions = new Map(positions);
      for (const joint of description.joints) {
        if (joint.mimic) {
          resolvedPositions.delete(joint.name);
        } else {
          const position = resolvedPositions.get(joint.name);
          if (position !== undefined && Number.isFinite(position)) {
            resolvedPositions.set(joint.name, limitPosition(joint, position));
          }
        }
      }
      for (let pass = 0; pass < description.joints.length; pass += 1) {
        let changed = false;
        for (const joint of description.joints) {
          if (!joint.mimic || resolvedPositions.has(joint.name)) continue;
          const sourcePosition = resolvedPositions.get(joint.mimic.joint);
          if (sourcePosition === undefined || !Number.isFinite(sourcePosition)) {
            continue;
          }
          resolvedPositions.set(
            joint.name,
            limitPosition(
              joint,
              sourcePosition * joint.mimic.multiplier + joint.mimic.offset,
            ),
          );
          changed = true;
        }
        if (!changed) break;
      }

      for (const [name, rawPosition] of resolvedPositions) {
        const position = Number.isFinite(rawPosition) ? rawPosition : undefined;
        if (position === undefined) continue;
        const transform = jointTransforms.get(name);
        if (!transform) continue;
        const { joint, motion } = transform;
        if (joint.jointType === "revolute" || joint.jointType === "continuous") {
          motion.quaternion.setFromAxisAngle(
            new THREE.Vector3(...joint.axis),
            position,
          );
        } else if (joint.jointType === "prismatic") {
          motion.position.set(
            joint.axis[0] * position,
            joint.axis[1] * position,
            joint.axis[2] * position,
          );
        }
      }
    },
    setCollisionVisible(visible) {
      collisionObjects.forEach((object) => {
        object.visible = visible;
      });
    },
  };
}
