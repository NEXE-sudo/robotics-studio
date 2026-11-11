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
}

export interface RobotJoint {
  name: string;
  jointType: string;
  parent: string;
  child: string;
  origin: RobotOrigin;
  axis: [number, number, number];
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
}

function createVisual(visual: RobotVisual) {
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

  const material = new THREE.MeshStandardMaterial({
    color: color
      ? new THREE.Color(color[0], color[1], color[2])
      : new THREE.Color(0x80aaff),
    opacity: color?.[3] ?? 1,
    transparent: (color?.[3] ?? 1) < 1,
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
      for (const [name, position] of positions) {
        if (!Number.isFinite(position)) continue;
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
  };
}
