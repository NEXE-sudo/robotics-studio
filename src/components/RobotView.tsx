import { useEffect, useRef, useState } from "react";
import * as THREE from "three";
import { listen } from "@tauri-apps/api/event";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { invoke } from "@tauri-apps/api/core";
import { ArrowUp, ArrowDown, RotateCcw, RotateCw, Square } from "lucide-react";
import { buildRobotInstance } from "../robotDescription";
import type { RobotDescription, RobotInstance } from "../robotDescription";

interface OdometryUpdate {
  node: string;
  x: number;
  y: number;
  z: number;
  qx: number;
  qy: number;
  qz: number;
  qw: number;
}

interface JointStateUpdate {
  names: string[];
  positions: number[];
}

function buildPlaceholderRobot(node: string): RobotInstance {
  const isBlue = node.includes("blue");
  const bodyColor = isBlue ? 0x8080ff : 0x80ff80;
  const wheelColor = 0x333333;
  const group = new THREE.Group();

  const chassis = new THREE.Mesh(
    new THREE.BoxGeometry(2.01142, 0.568726, 1),
    new THREE.MeshStandardMaterial({ color: bodyColor }),
  );
  chassis.position.set(-0.151427, 0.175, 0);
  chassis.castShadow = true;
  chassis.receiveShadow = true;
  group.add(chassis);

  const wheelMaterial = new THREE.MeshStandardMaterial({ color: wheelColor });
  const leftWheel = new THREE.Mesh(
    new THREE.SphereGeometry(0.3, 16, 16),
    wheelMaterial,
  );
  leftWheel.position.set(0.554283, -0.025, -0.625029);
  leftWheel.castShadow = true;
  group.add(leftWheel);

  const rightWheel = new THREE.Mesh(
    new THREE.SphereGeometry(0.3, 16, 16),
    wheelMaterial,
  );
  rightWheel.position.set(0.554282, -0.025, 0.625029);
  rightWheel.castShadow = true;
  group.add(rightWheel);

  const caster = new THREE.Mesh(
    new THREE.SphereGeometry(0.2, 16, 16),
    new THREE.MeshStandardMaterial({ color: wheelColor }),
  );
  caster.position.set(-0.957138, -0.125, 0);
  caster.castShadow = true;
  group.add(caster);

  return {
    group,
    updateJoints: () => undefined,
    setCollisionVisible: () => undefined,
  };
}

export default function RobotView({
  descriptionContent,
}: {
  descriptionContent: string | null;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const robotsRef = useRef<Record<string, RobotInstance>>({});
  const [robotDescription, setRobotDescription] =
    useState<RobotDescription | null>(null);
  const [descriptionError, setDescriptionError] = useState<string | null>(null);
  const [showCollisions, setShowCollisions] = useState(false);
  const collisionVisibleRef = useRef(false);

  useEffect(() => {
    if (!descriptionContent) {
      setRobotDescription(null);
      setDescriptionError(null);
      return;
    }

    let active = true;
    setRobotDescription(null);
    setDescriptionError(null);
    invoke<RobotDescription>("parse_robot_description", {
      content: descriptionContent,
    })
      .then((description) => {
        if (active) setRobotDescription(description);
      })
      .catch((error) => {
        if (active) setDescriptionError(String(error));
      });

    return () => {
      active = false;
    };
  }, [descriptionContent]);

  useEffect(() => {
    collisionVisibleRef.current = showCollisions;
    Object.values(robotsRef.current).forEach((robot) =>
      robot.setCollisionVisible(showCollisions),
    );
  }, [showCollisions]);

  useEffect(() => {
    if (!containerRef.current) return;

    let isActive = true;
    let animationFrameId: number;

    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x1a1a1a);

    const camera = new THREE.PerspectiveCamera(
      60,
      containerRef.current.clientWidth / containerRef.current.clientHeight,
      0.1,
      1000,
    );

    camera.position.set(3, 3, 3);
    camera.lookAt(0, 0, 0);

    const renderer = new THREE.WebGLRenderer({
      antialias: true,
    });

    renderer.setPixelRatio(window.devicePixelRatio);
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;

    renderer.setPixelRatio(window.devicePixelRatio);

    containerRef.current.appendChild(renderer.domElement);

    const resize = () => {
      if (!containerRef.current) return;

      const width = containerRef.current.clientWidth;
      const height = containerRef.current.clientHeight;

      console.log("Resize:", width, height);

      if (width === 0 || height === 0) return;

      camera.aspect = width / height;
      camera.updateProjectionMatrix();

      renderer.setSize(width, height);
    };

    resize();

    const controls = new OrbitControls(camera, renderer.domElement);

    controls.enableDamping = true;
    controls.dampingFactor = 0.1;

    controls.target.set(0, 0, 0);
    controls.update();

    const observer = new ResizeObserver(resize);
    observer.observe(containerRef.current);

    const grid = new THREE.GridHelper(10, 10);
    grid.receiveShadow = true;
    scene.add(grid);
    scene.add(new THREE.AxesHelper(1));

    const light = new THREE.DirectionalLight(0xffffff, 1);
    light.position.set(5, 10, 5);

    light.castShadow = true;

    scene.add(light);
    scene.add(new THREE.AmbientLight(0x404040));

    if (robotDescription) {
      const preview = buildRobotInstance(robotDescription);
      preview.setCollisionVisible(collisionVisibleRef.current);
      scene.add(preview.group);
      robotsRef.current["__description_preview__"] = preview;
    }

    const animate = () => {
      if (!isActive) return;

      animationFrameId = requestAnimationFrame(animate);
      controls.update();
      renderer.render(scene, camera);
    };

    animate();

    const unlistenClear = listen("clear-robots", () => {
      if (!isActive) return;

      Object.values(robotsRef.current).forEach((robot) =>
        scene.remove(robot.group),
      );
      robotsRef.current = {};
      if (robotDescription) {
        const preview = buildRobotInstance(robotDescription);
        preview.setCollisionVisible(collisionVisibleRef.current);
        scene.add(preview.group);
        robotsRef.current["__description_preview__"] = preview;
      }
    });

    const unlistenPromise = listen<OdometryUpdate>(
      "odometry-update",
      (event) => {
        if (!isActive) return;

        const { node, x, y, z, qx, qy, qz, qw } = event.payload;

        let robot = robotsRef.current[node];

        if (!robot) {
          robot =
            robotsRef.current["__description_preview__"] ??
            (robotDescription
              ? buildRobotInstance(robotDescription)
              : buildPlaceholderRobot(node));
          robot.setCollisionVisible(collisionVisibleRef.current);
          if (robotsRef.current["__description_preview__"]) {
            delete robotsRef.current["__description_preview__"];
          } else {
            scene.add(robot.group);
          }
          robotsRef.current[node] = robot;
        }

        robot.group.position.set(x, z, -y);
        robot.group.quaternion.set(qx, qz, -qy, qw);
      },
    );
    const unlistenJointState = listen<JointStateUpdate>(
      "joint-state-update",
      (event) => {
        if (!isActive) return;
        const { names, positions } = event.payload;
        const jointPositions = new Map<string, number>();
        for (let index = 0; index < Math.min(names.length, positions.length); index += 1) {
          jointPositions.set(names[index], positions[index]);
        }
        Object.values(robotsRef.current).forEach((robot) =>
          robot.updateJoints(jointPositions),
        );
      },
    );

    return () => {
      isActive = false;

      observer.disconnect();

      cancelAnimationFrame(animationFrameId);

      unlistenPromise.then((f) => f()).catch(() => {});
      unlistenClear.then((f) => f()).catch(() => {});
      unlistenJointState.then((f) => f()).catch(() => {});

      Object.values(robotsRef.current).forEach((robot) => {
        scene.remove(robot.group);
        robot.group.traverse((child) => {
          if (child instanceof THREE.Mesh) {
            child.geometry.dispose();
            if (Array.isArray(child.material)) {
              child.material.forEach((m) => m.dispose());
            } else {
              child.material.dispose();
            }
          }
        });
      });

      controls.dispose();

      renderer.dispose();

      robotsRef.current = {};

      if (containerRef.current) {
        containerRef.current.innerHTML = "";
      }
    };
  }, [robotDescription]);

  return (
    <div
      ref={containerRef}
      style={{
        width: "100%",
        height: "100%",
        overflow: "hidden",
        position: "relative",
      }}
    >
      {robotDescription && (
        <button
          type="button"
          aria-pressed={showCollisions}
          onClick={() => setShowCollisions((visible) => !visible)}
          style={{
            position: "absolute",
            zIndex: 1,
            left: 8,
            bottom: 8,
          }}
        >
          {showCollisions ? "Hide" : "Show"} collision geometry
        </button>
      )}
      {(descriptionError || robotDescription?.warnings.length) && (
        <div
          role={descriptionError ? "alert" : "status"}
          style={{
            position: "absolute",
            zIndex: 1,
            top: 8,
            left: 8,
            maxWidth: 420,
            padding: 8,
            background: "rgba(25,25,25,0.92)",
            color: descriptionError ? "#ff8b82" : "#e0b060",
            fontSize: 12,
            whiteSpace: "pre-wrap",
          }}
        >
          {descriptionError ?? robotDescription?.warnings.join("\n")}
        </div>
      )}
      {descriptionContent && !robotDescription && !descriptionError && (
        <div
          role="status"
          style={{
            position: "absolute",
            zIndex: 1,
            top: 8,
            left: 8,
            padding: 8,
            background: "rgba(25,25,25,0.92)",
            fontSize: 12,
          }}
        >
          Validating robot description...
        </div>
      )}
      {!descriptionContent && !descriptionError && (
        <div
          role="status"
          style={{
            position: "absolute",
            zIndex: 1,
            top: 8,
            left: 8,
            padding: 8,
            background: "rgba(25,25,25,0.92)",
            fontSize: 12,
          }}
        >
          Open a URDF file to preview its geometry; a placeholder robot is shown.
        </div>
      )}
    </div>
  );
}

export function RobotControls({
  onCommand,
  availableRobots,
  safetyState,
  safetyPending,
  safetyError,
  commandError,
  onEmergencyStop,
  onResetEmergencyStop,
}: {
  onCommand: (topic: string, lin: number, ang: number) => void;
  availableRobots: string[];
  safetyState: "READY" | "ACTIVE" | "RESET_PENDING" | "FAULT" | "DISCONNECTED";
  safetyPending: boolean;
  safetyError: string | null;
  commandError: string | null;
  onEmergencyStop: () => void;
  onResetEmergencyStop: () => void;
}) {
  const [selectedRobot, setSelectedRobot] = useState<string | null>(null);
  const [robotConfig, setRobotConfig] = useState<RobotMotionConfig | null>(null);
  const [configError, setConfigError] = useState<string | null>(null);
  const [configBusy, setConfigBusy] = useState(false);

  useEffect(() => {
    if (!selectedRobot) {
      setRobotConfig(null);
      return;
    }
    let active = true;
    setConfigBusy(true);
    setConfigError(null);
    invoke<RobotMotionConfig>("get_robot_config", { topicName: selectedRobot })
      .then((config) => {
        if (active) setRobotConfig(config);
      })
      .catch((error) => {
        if (active) setConfigError(String(error));
      })
      .finally(() => {
        if (active) setConfigBusy(false);
      });
    return () => {
      active = false;
    };
  }, [selectedRobot]);

  const saveConfig = async (reset = false) => {
    if (!selectedRobot || !robotConfig || configBusy) return;
    setConfigBusy(true);
    setConfigError(null);
    try {
      const saved = reset
        ? await invoke<RobotMotionConfig>("reset_robot_config", {
            topicName: selectedRobot,
          })
        : await invoke<RobotMotionConfig>("save_robot_config", {
            config: robotConfig,
          });
      setRobotConfig(saved);
    } catch (error) {
      setConfigError(String(error));
    } finally {
      setConfigBusy(false);
    }
  };

  // Keep selection valid as the available robot list changes (e.g., after
  // switching worlds) — default to the first available robot if none is
  // selected yet, or if the previously selected one no longer exists.
  useEffect(() => {
    if (availableRobots.length === 0) return;
    if (!selectedRobot || !availableRobots.includes(selectedRobot)) {
      setSelectedRobot(availableRobots[0]);
    }
  }, [availableRobots, selectedRobot]);

  const btnStyle: React.CSSProperties = {
    width: "var(--icon-lg)",
    height: "var(--icon-lg)",
    fontSize: "var(--font-md)",
    margin: "var(--spacing-xs)",
    cursor: "pointer",
  };

  return (
    <div style={{ padding: "var(--spacing-base)", color: "#ccc" }}>
      {availableRobots.length === 0 ? (
        <div
          style={{
            fontSize: "var(--font-xs)",
            opacity: 0.5,
            marginBottom: "var(--spacing-base)",
          }}
        >
          No robots detected — start the simulation.
        </div>
      ) : (
        <select
          value={selectedRobot ?? ""}
          onChange={(e) => setSelectedRobot(e.target.value)}
          style={{ marginBottom: 10 }}
        >
          {availableRobots.map((topic) => {
            // Derive a readable name from the topic path, e.g.
            // /model/vehicle_blue/cmd_vel -> vehicle_blue
            const name = topic.split("/")[2] ?? topic;
            return (
              <option key={topic} value={topic}>
                {name}
              </option>
            );
          })}
        </select>
      )}

      <div
        role="status"
        aria-live="polite"
        style={{
          margin: "8px 0",
          padding: 8,
          color: safetyState === "ACTIVE" || safetyState === "FAULT" ? "#ff8b82" : "#ccc",
          background: safetyState === "ACTIVE" || safetyState === "FAULT" ? "#421f1f" : "#252526",
        }}
      >
        Safety state: {safetyState}
        {safetyState === "ACTIVE" && (
          <div>Latched. Reset is explicit; motion will not resume automatically.</div>
        )}
      </div>
      {safetyError && <div role="alert" style={{ color: "#ff8b82" }}>{safetyError}</div>}
      {commandError && <div role="alert" style={{ color: "#ff8b82" }}>{commandError}</div>}
      {safetyState === "ACTIVE" || safetyState === "FAULT" ? (
        <button
          type="button"
          className="ide-btn primary"
          disabled={safetyPending || safetyState === "FAULT"}
          onClick={onResetEmergencyStop}
          style={{ width: "100%", marginBottom: 8 }}
        >
          {safetyPending ? "Applying..." : "Reset Emergency Stop"}
        </button>
      ) : (
        <button
          type="button"
          className="ide-btn"
          disabled={safetyPending || safetyState !== "READY"}
          onClick={onEmergencyStop}
          style={{
            width: "100%",
            marginBottom: 8,
            color: "#fff",
            background: "#a32323",
          }}
        >
          {safetyPending ? "Applying..." : "EMERGENCY STOP"}
        </button>
      )}

      <div
        style={{
          marginBottom: "var(--spacing-sm)",
          fontSize: "var(--font-xs)",
          opacity: 0.75,
        }}
      >
        {robotConfig
          ? `Limits: ${robotConfig.maxLinearVelocity} m/s linear, ${robotConfig.maxAngularVelocity} rad/s angular; ${robotConfig.commandLeaseMs} ms command lease.`
          : "Per-robot motion limits are enforced by the ROS service."}{" "}
        Software stop only; not safety-rated.
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(3, 48px)",
          justifyContent: "center",
          opacity: selectedRobot && safetyState === "READY" && !safetyPending ? 1 : 0.4,
          pointerEvents: selectedRobot && safetyState === "READY" && !safetyPending ? "auto" : "none",
        }}
      >
        <div />
        <button
          disabled={!selectedRobot || safetyState !== "READY" || safetyPending}
          style={btnStyle}
          onClick={() => onCommand(selectedRobot!, 0.3, 0)}
        >
          <ArrowUp
            style={{ width: "var(--icon-md)", height: "var(--icon-md)" }}
          />
        </button>
        <div />
        <button
          disabled={!selectedRobot || safetyState !== "READY" || safetyPending}
          style={btnStyle}
          onClick={() => onCommand(selectedRobot!, 0, 0.5)}
        >
          <RotateCcw
            style={{ width: "var(--icon-md)", height: "var(--icon-md)" }}
          />
        </button>
        <button
          disabled={!selectedRobot || safetyState !== "READY" || safetyPending}
          style={btnStyle}
          onClick={() => onCommand(selectedRobot!, 0, 0)}
        >
          <Square
            style={{ width: "var(--icon-sm)", height: "var(--icon-sm)" }}
          />
        </button>
        <button
          disabled={!selectedRobot || safetyState !== "READY" || safetyPending}
          style={btnStyle}
          onClick={() => onCommand(selectedRobot!, 0, -0.5)}
        >
          <RotateCw
            style={{ width: "var(--icon-md)", height: "var(--icon-md)" }}
          />
        </button>
        <div />
        <button
          disabled={!selectedRobot || safetyState !== "READY" || safetyPending}
          style={btnStyle}
          onClick={() => onCommand(selectedRobot!, -0.3, 0)}
        >
          <ArrowDown
            style={{ width: "var(--icon-md)", height: "var(--icon-md)" }}
          />
        </button>
        <div />
      </div>
      {selectedRobot && robotConfig && (
        <fieldset
          disabled={configBusy}
          style={{ marginTop: 12, border: "1px solid #444", padding: 8 }}
        >
          <legend>Robot motion limits</legend>
          {([
            ["maxLinearVelocity", "Linear velocity (m/s)"],
            ["maxAngularVelocity", "Angular velocity (rad/s)"],
            ["maxLinearAcceleration", "Linear acceleration (m/s²)"],
            ["maxAngularAcceleration", "Angular acceleration (rad/s²)"],
            ["commandLeaseMs", "Command lease (ms)"],
            ["commandTimeoutMs", "Command timeout (ms)"],
          ] as const).map(([field, label]) => (
            <label key={field} style={{ display: "block", fontSize: 11, margin: "6px 0" }}>
              {label}
              <input
                type="number"
                min={field.endsWith("Ms") ? 100 : 0.01}
                step={field.endsWith("Ms") ? 50 : 0.05}
                value={robotConfig[field]}
                onChange={(event) => {
                  const value = Number(event.target.value);
                  setRobotConfig((current) => current && { ...current, [field]: value });
                }}
                style={{ width: "100%" }}
              />
            </label>
          ))}
          <label style={{ display: "block", fontSize: 11 }}>
            Motion mode
            <select
              value={robotConfig.permittedMotionModes.includes("full_twist") ? "full_twist" : "planar"}
              onChange={(event) =>
                setRobotConfig((current) => current && {
                  ...current,
                  permittedMotionModes: [event.target.value],
                })
              }
              style={{ width: "100%" }}
            >
              <option value="planar">Planar</option>
              <option value="full_twist">Full Twist</option>
            </select>
          </label>
          {configError && <div role="alert" style={{ color: "#ff8b82" }}>{configError}</div>}
          <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
            <button type="button" className="ide-btn primary" disabled={configBusy} onClick={() => void saveConfig()}>
              Save limits
            </button>
            <button type="button" className="ide-btn" disabled={configBusy} onClick={() => void saveConfig(true)}>
              Safe defaults
            </button>
          </div>
        </fieldset>
      )}
    </div>
  );
}

interface RobotMotionConfig {
  topicName: string;
  maxLinearVelocity: number;
  maxAngularVelocity: number;
  maxLinearAcceleration: number;
  maxAngularAcceleration: number;
  commandLeaseMs: number;
  commandTimeoutMs: number;
  permittedMotionModes: string[];
}
