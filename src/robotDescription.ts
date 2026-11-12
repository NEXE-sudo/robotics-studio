import * as THREE from "three";
import { ColladaLoader } from "three/examples/jsm/loaders/ColladaLoader.js";
import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js";

export interface RobotOrigin {
  xyz: [number, number, number];
  rpy: [number, number, number];
}

export type RobotGeometry =
  | { kind: "box"; size: [number, number, number] }
  | { kind: "cylinder"; radius: number; length: number }
  | { kind: "sphere"; radius: number }
  | {
      kind: "mesh";
      filename: string;
      scale: [number, number, number];
      format: "stl" | "dae";
      data: string | null;
    }
  | { kind: "unsupported"; description: string };

export interface RobotVisual {
  origin: RobotOrigin;
  geometry: RobotGeometry;
  color: [number, number, number, number] | null;
  materialName?: string | null;
}

export interface RobotInertial {
  origin: RobotOrigin;
  mass: number;
  inertia: [number, number, number, number, number, number];
}

export interface RobotLink {
  name: string;
  visuals: RobotVisual[];
  collisions: RobotVisual[];
  inertial?: RobotInertial | null;
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
  setInertialVisible: (visible: boolean) => void;
  dispose: () => void;
}

function bytesFromBase64(data: string): ArrayBuffer {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes.buffer;
}

function createVisual(visual: RobotVisual, collision = false) {
  const { geometry, color } = visual;
  let contents: THREE.Object3D;
  let shape: THREE.BufferGeometry;
  switch (geometry.kind) {
    case "box":
      shape = new THREE.BoxGeometry(...geometry.size);
      contents = new THREE.Mesh(
        shape,
        new THREE.MeshStandardMaterial({
          color: collision
            ? 0xe34b4b
            : color
              ? new THREE.Color(color[0], color[1], color[2])
              : 0x80aaff,
          opacity: collision ? Math.min(color?.[3] ?? 0.35, 0.35) : (color?.[3] ?? 1),
          transparent: collision || (color?.[3] ?? 1) < 1,
          wireframe: collision,
        }),
      );
      break;
    case "cylinder":
      shape = new THREE.CylinderGeometry(
        geometry.radius,
        geometry.radius,
        geometry.length,
        24,
      );
      contents = new THREE.Mesh(
        shape,
        new THREE.MeshStandardMaterial({
          color: collision
            ? 0xe34b4b
            : color
              ? new THREE.Color(color[0], color[1], color[2])
              : 0x80aaff,
          opacity: collision ? Math.min(color?.[3] ?? 0.35, 0.35) : (color?.[3] ?? 1),
          transparent: collision || (color?.[3] ?? 1) < 1,
          wireframe: collision,
        }),
      );
      (contents as THREE.Mesh).rotation.x = Math.PI / 2;
      break;
    case "sphere":
      shape = new THREE.SphereGeometry(geometry.radius, 24, 16);
      contents = new THREE.Mesh(
        shape,
        new THREE.MeshStandardMaterial({
          color: collision
            ? 0xe34b4b
            : color
              ? new THREE.Color(color[0], color[1], color[2])
              : 0x80aaff,
          opacity: collision ? Math.min(color?.[3] ?? 0.35, 0.35) : (color?.[3] ?? 1),
          transparent: collision || (color?.[3] ?? 1) < 1,
          wireframe: collision,
        }),
      );
      break;
    case "mesh": {
      if (!geometry.data) {
        throw new Error(`Mesh data for '${geometry.filename}' was not loaded`);
      }
      const buffer = bytesFromBase64(geometry.data);
      if (geometry.format === "stl") {
        const meshGeometry = new STLLoader().parse(buffer);
        const material = new THREE.MeshStandardMaterial({
          color: collision
            ? 0xe34b4b
            : color
              ? new THREE.Color(color[0], color[1], color[2])
              : 0x80aaff,
          opacity: collision ? Math.min(color?.[3] ?? 0.35, 0.35) : (color?.[3] ?? 1),
          transparent: collision || (color?.[3] ?? 1) < 1,
          wireframe: collision,
        });
        contents = new THREE.Mesh(meshGeometry, material);
      } else {
        const daeContent = new TextDecoder().decode(buffer);
        const dae = new ColladaLoader().parse(daeContent, "");
        if (!dae) {
          throw new Error(`Could not parse DAE mesh '${geometry.filename}'`);
        }
        contents = dae.scene;
        const upAxis =
          /<up_axis>\s*([XYZ]_UP)\s*<\/up_axis>/i.exec(daeContent)?.[1]?.toUpperCase() ??
          "Y_UP";
        contents.rotation.set(0, 0, 0);
        if (upAxis === "Y_UP") contents.rotation.x = Math.PI / 2;
        if (upAxis === "X_UP") contents.rotation.y = -Math.PI / 2;
        if (color || collision) {
          contents.traverse((child) => {
            if (!(child instanceof THREE.Mesh)) return;
            const existing = Array.isArray(child.material)
              ? child.material
              : [child.material];
            for (const material of existing) {
              if ("color" in material && material.color instanceof THREE.Color) {
                material.color.set(
                  collision
                    ? 0xe34b4b
                    : new THREE.Color(color![0], color![1], color![2]),
                );
              }
              material.opacity = collision
                ? Math.min(color?.[3] ?? 0.35, 0.35)
                : (color?.[3] ?? material.opacity);
              material.transparent = collision || material.opacity < 1;
              material.wireframe = collision;
            }
          });
        }
      }
      contents.scale.set(...geometry.scale);
      break;
    }
    case "unsupported":
      return null;
  }

  contents.traverse((child) => {
    if (child instanceof THREE.Mesh) {
      child.castShadow = true;
      child.receiveShadow = true;
    }
  });

  const visualGroup = new THREE.Group();
  visualGroup.position.set(...visual.origin.xyz);
  visualGroup.rotation.set(...visual.origin.rpy, "ZYX");
  visualGroup.add(contents);
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
  const inertialObjects: THREE.Object3D[] = [];
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
    if (definition.inertial) {
      const inertial = new THREE.Group();
      inertial.name = `${name}:inertial`;
      inertial.position.set(...definition.inertial.origin.xyz);
      inertial.rotation.set(...definition.inertial.origin.rpy, "ZYX");
      const axes = new THREE.AxesHelper(0.12);
      inertial.add(axes);
      const radiusOfGyration =
        definition.inertial.mass > 0
          ? Math.sqrt(
              Math.max(
                0,
                (definition.inertial.inertia[0] +
                  definition.inertial.inertia[3] +
                  definition.inertial.inertia[5]) /
                  (3 * definition.inertial.mass),
              ),
            )
          : 0;
      if (radiusOfGyration > 0) {
        const marker = new THREE.Mesh(
          new THREE.SphereGeometry(1, 12, 8),
          new THREE.MeshBasicMaterial({
            color: 0xffaa44,
            wireframe: true,
            transparent: true,
            opacity: 0.55,
          }),
        );
        marker.scale.setScalar(Math.min(radiusOfGyration, 1));
        inertial.add(marker);
      }
      inertial.visible = false;
      inertialObjects.push(inertial);
      link.add(inertial);
    }

    for (const joint of jointsByParent.get(name) ?? []) {
      const origin = new THREE.Group();
      origin.name = `${joint.name}:origin`;
      origin.position.set(...joint.origin.xyz);
      origin.rotation.set(...joint.origin.rpy, "ZYX");
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
    setInertialVisible(visible) {
      inertialObjects.forEach((object) => {
        object.visible = visible;
      });
    },
    dispose() {
      const geometries = new Set<THREE.BufferGeometry>();
      const materials = new Set<THREE.Material>();
      const textures = new Set<THREE.Texture>();
      group.traverse((child) => {
        if (child instanceof THREE.Mesh) {
          geometries.add(child.geometry);
          const meshMaterials = Array.isArray(child.material)
            ? child.material
            : [child.material];
          for (const material of meshMaterials) {
            materials.add(material);
            for (const value of Object.values(material)) {
              if (value instanceof THREE.Texture) textures.add(value);
            }
          }
        } else if (child instanceof THREE.AxesHelper) {
          geometries.add(child.geometry);
          const axesMaterials = Array.isArray(child.material)
            ? child.material
            : [child.material];
          axesMaterials.forEach((material) => materials.add(material));
        }
      });
      geometries.forEach((geometry) => geometry.dispose());
      materials.forEach((material) => material.dispose());
      textures.forEach((texture) => texture.dispose());
    },
  };
}
