import { useEffect, useRef, useState } from "react";
import * as THREE from "three";
import { listen } from "@tauri-apps/api/event";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";

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

export default function RobotView() {
  const containerRef = useRef<HTMLDivElement>(null);
  const robotsRef = useRef<Record<string, THREE.Mesh>>({});

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

    const animate = () => {
      if (!isActive) return;

      animationFrameId = requestAnimationFrame(animate);
      controls.update();
      renderer.render(scene, camera);
    };

    animate();

    const unlistenClear = listen("clear-robots", () => {
      if (!isActive) return;

      Object.values(robotsRef.current).forEach((mesh) => scene.remove(mesh));
      robotsRef.current = {};
    });

    const unlistenPromise = listen<OdometryUpdate>(
      "odometry-update",
      (event) => {
        if (!isActive) return;

        const { node, x, y, z, qx, qy, qz, qw } = event.payload;

        let mesh = robotsRef.current[node];

        if (!mesh) {
          const geometry = new THREE.BoxGeometry(0.3, 0.2, 0.15);
          const color = node.includes("blue") ? 0x3388ff : 0x33ff88;
          const material = new THREE.MeshStandardMaterial({ color });

          mesh = new THREE.Mesh(geometry, material);

          mesh.castShadow = true;
          mesh.receiveShadow = true;

          scene.add(mesh);
          robotsRef.current[node] = mesh;
        }

        mesh.position.set(x, z, -y);
        mesh.quaternion.set(qx, qz, -qy, qw);
      },
    );

    return () => {
      isActive = false;

      observer.disconnect();

      cancelAnimationFrame(animationFrameId);

      unlistenPromise.then((f) => f()).catch(() => {});
      unlistenClear.then((f) => f()).catch(() => {});

      Object.values(robotsRef.current).forEach((mesh) => {
        scene.remove(mesh);

        mesh.geometry.dispose();

        if (Array.isArray(mesh.material)) {
          mesh.material.forEach((m) => m.dispose());
        } else {
          mesh.material.dispose();
        }
      });

      controls.dispose();

      renderer.dispose();

      robotsRef.current = {};

      if (containerRef.current) {
        containerRef.current.innerHTML = "";
      }
    };
  }, []);

  return (
    <div
      ref={containerRef}
      style={{
        width: "100%",
        height: "100%",
        overflow: "hidden",
      }}
    />
  );
}

export function RobotControls({
  onCommand,
}: {
  onCommand: (topic: string, lin: number, ang: number) => void;
}) {
  const [selectedRobot, setSelectedRobot] = useState(
    "/model/vehicle_blue/cmd_vel",
  );

  const btnStyle: React.CSSProperties = {
    width: 48,
    height: 48,
    fontSize: 18,
    margin: 2,
    cursor: "pointer",
  };

  return (
    <div style={{ padding: 12, color: "#ccc" }}>
      <select
        value={selectedRobot}
        onChange={(e) => setSelectedRobot(e.target.value)}
        style={{ marginBottom: 10 }}
      >
        <option value="/model/vehicle_blue/cmd_vel">Blue Robot</option>
        <option value="/model/vehicle_green/cmd_vel">Green Robot</option>
      </select>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(3, 48px)",
          justifyContent: "center",
        }}
      >
        <div />
        <button
          style={btnStyle}
          onClick={() => onCommand(selectedRobot, 0.3, 0)}
        >
          ↑
        </button>
        <div />
        <button
          style={btnStyle}
          onClick={() => onCommand(selectedRobot, 0, 0.5)}
        >
          ↺
        </button>
        <button style={btnStyle} onClick={() => onCommand(selectedRobot, 0, 0)}>
          ■
        </button>
        <button
          style={btnStyle}
          onClick={() => onCommand(selectedRobot, 0, -0.5)}
        >
          ↻
        </button>
        <div />
        <button
          style={btnStyle}
          onClick={() => onCommand(selectedRobot, -0.3, 0)}
        >
          ↓
        </button>
        <div />
      </div>
    </div>
  );
}
