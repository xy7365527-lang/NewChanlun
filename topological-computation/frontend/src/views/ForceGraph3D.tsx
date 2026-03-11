/**
 * ForceGraph3D.tsx
 *
 * Three.js 原生 3D 力导向图。
 * 用户可旋转/缩放查看不同角度，减少边交叉。
 *
 * 技术栈：three.js 原生 + useRef + requestAnimationFrame
 * （@react-three/fiber 需要 React 19，当前是 React 18，故用原生方案）
 *
 * 特性：
 * - 节点球体用 f 值着色（fZero/fLow/fMid/fHigh）
 * - settled 节点：蓝色半透明外壳（不只是光环）
 * - 当前穿越位置：白色发光球体 + 脉冲动画
 * - 鼠标拖拽旋转，滚轮缩放
 * - 点击节点 → onSelectNode 回调
 */

import { useEffect, useRef, useState } from "react";
import * as THREE from "three";
import { T, FONT, fToColor } from "../tokens";
import type { TopologyNode, TopologyLink, TopologyResponse } from "../types";

// ── 类型 ────────────────────────────────────────────────────────

interface Props {
  data: TopologyResponse | null;
  traversalPosition?: string;
  focusConcept?: string | null;
  onSelectNode?: (node: TopologyNode) => void;
}

// ── 常量 ────────────────────────────────────────────────────────

const DAMPING = 0.88;          // 弹力模拟阻尼
const REPULSION = 18000;       // 斥力强度
const SPRING_K = 0.003;        // 弹簧系数
const SPRING_REST = 80;        // 弹簧自然长度
const MAX_SPEED = 8;           // 最大速度
const SIM_STEPS = 120;         // 布局迭代次数（初始冷却）

// ── 3D 力导向布局（纯 JS，无 d3） ───────────────────────────────

interface Vec3 { x: number; y: number; z: number }

function initLayout(nodes: TopologyNode[], size = 300): Vec3[] {
  return nodes.map(() => ({
    x: (Math.random() - 0.5) * size,
    y: (Math.random() - 0.5) * size,
    z: (Math.random() - 0.5) * size,
  }));
}

function runLayoutStep(
  positions: Vec3[],
  velocities: Vec3[],
  nodes: TopologyNode[],
  links: TopologyLink[],
  alpha: number
): void {
  const N = nodes.length;
  if (N === 0) return;

  // 斥力：O(N²)，对小图（≤500）可接受
  for (let i = 0; i < N; i++) {
    for (let j = i + 1; j < N; j++) {
      const dx = positions[j].x - positions[i].x;
      const dy = positions[j].y - positions[i].y;
      const dz = positions[j].z - positions[i].z;
      const d2 = dx * dx + dy * dy + dz * dz + 1;
      const force = REPULSION * alpha / d2;
      const fx = force * dx;
      const fy = force * dy;
      const fz = force * dz;
      velocities[i].x -= fx;
      velocities[i].y -= fy;
      velocities[i].z -= fz;
      velocities[j].x += fx;
      velocities[j].y += fy;
      velocities[j].z += fz;
    }
  }

  // 弹力（边）
  const idxMap = new Map(nodes.map((n, i) => [n.id, i]));
  for (const link of links) {
    const srcId = typeof link.source === "string" ? link.source : (link.source as TopologyNode).id;
    const tgtId = typeof link.target === "string" ? link.target : (link.target as TopologyNode).id;
    const i = idxMap.get(srcId);
    const j = idxMap.get(tgtId);
    if (i === undefined || j === undefined) continue;
    const dx = positions[j].x - positions[i].x;
    const dy = positions[j].y - positions[i].y;
    const dz = positions[j].z - positions[i].z;
    const d = Math.sqrt(dx * dx + dy * dy + dz * dz) + 0.01;
    const force = SPRING_K * (d - SPRING_REST) * alpha;
    const fx = force * dx / d;
    const fy = force * dy / d;
    const fz = force * dz / d;
    velocities[i].x += fx;
    velocities[i].y += fy;
    velocities[i].z += fz;
    velocities[j].x -= fx;
    velocities[j].y -= fy;
    velocities[j].z -= fz;
  }

  // 更新位置 + 阻尼 + 限速
  for (let i = 0; i < N; i++) {
    velocities[i].x *= DAMPING;
    velocities[i].y *= DAMPING;
    velocities[i].z *= DAMPING;
    const speed = Math.sqrt(
      velocities[i].x ** 2 + velocities[i].y ** 2 + velocities[i].z ** 2
    );
    if (speed > MAX_SPEED) {
      const s = MAX_SPEED / speed;
      velocities[i].x *= s;
      velocities[i].y *= s;
      velocities[i].z *= s;
    }
    positions[i].x += velocities[i].x;
    positions[i].y += velocities[i].y;
    positions[i].z += velocities[i].z;
  }
}

// ── 组件 ────────────────────────────────────────────────────────

// Maximum node count for O(N^2) force layout — above this the view is unusable
const MAX_FORCE_NODES = 2000;

export function ForceGraph3D({
  data, traversalPosition, focusConcept, onSelectNode,
}: Props) {
  const mountRef = useRef<HTMLDivElement>(null);
  const [hoveredNode, setHoveredNode] = useState<TopologyNode | null>(null);

  // Scale guard: O(N^2) force layout is not viable above MAX_FORCE_NODES
  if (data && data.nodes.length > MAX_FORCE_NODES) {
    return (
      <div style={{
        width: "100%", height: "100%",
        display: "flex", alignItems: "center", justifyContent: "center",
        flexDirection: "column", gap: 12,
        fontFamily: "'DM Mono', monospace",
        color: "#6a6a8a",
        background: "#0a0a0f",
      }}>
        <div style={{ fontSize: 13 }}>
          3D Force — {data.nodes.length.toLocaleString()} nodes
        </div>
        <div style={{ fontSize: 10, color: "#3a3a5a", maxWidth: 320, textAlign: "center" }}>
          O(N^2) force layout cannot handle {data.nodes.length.toLocaleString()} nodes.
          Use Galaxy View for full-scale rendering.
        </div>
      </div>
    );
  }

  useEffect(() => {
    const mount = mountRef.current;
    if (!mount || !data || data.nodes.length === 0) return;

    // 等待容器有实际尺寸（flex 布局完成后）
    const W = mount.clientWidth || mount.offsetWidth || 800;
    const H = mount.clientHeight || mount.offsetHeight || 600;

    // 尺寸仍为 0：跳过本次渲染，浏览器下一帧会重触发 effect
    if (W === 0 || H === 0) return;

    // ── Scene ─────────────────────────────────────────────────
    const scene = new THREE.Scene();
    scene.background = new THREE.Color(T.bg);
    scene.fog = new THREE.FogExp2(T.bg, 0.0018);

    // ── Camera ────────────────────────────────────────────────
    const camera = new THREE.PerspectiveCamera(60, W / H, 1, 5000);
    camera.position.set(0, 0, 500);

    // ── Renderer ──────────────────────────────────────────────
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setSize(W, H);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    mount.appendChild(renderer.domElement);

    // ── Lights ────────────────────────────────────────────────
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.4);
    scene.add(ambientLight);
    const dirLight = new THREE.DirectionalLight(0xffffff, 0.8);
    dirLight.position.set(1, 2, 3);
    scene.add(dirLight);

    // ── 布局计算（同步，初始冷却） ────────────────────────────
    const nodes = data.nodes;
    const links = data.links;
    const positions = initLayout(nodes);
    const velocities: Vec3[] = nodes.map(() => ({ x: 0, y: 0, z: 0 }));

    // 初始迭代（在 useEffect 里同步跑，避免看到弹动过程）
    for (let step = 0; step < SIM_STEPS; step++) {
      const alpha = Math.max(0.01, 1 - step / SIM_STEPS);
      runLayoutStep(positions, velocities, nodes, links, alpha);
    }

    // ── 节点几何 ──────────────────────────────────────────────
    const nodeMeshes: THREE.Mesh[] = [];
    const nodeMap = new Map<string, number>(); // id → index

    nodes.forEach((node, i) => {
      nodeMap.set(node.id, i);
      const isTraversal = node.id === traversalPosition;
      const isFocus = node.label === focusConcept;

      const r = isTraversal ? 10 : isFocus ? 8 :
        2 + Math.sqrt(Math.min(node.degree, 100)) * 0.6;

      const geo = new THREE.SphereGeometry(r, 12, 8);
      const colorHex = isTraversal ? T.traversalPulse : fToColor(node.f_avg);
      const mat = new THREE.MeshPhongMaterial({
        color: new THREE.Color(colorHex),
        emissive: isTraversal
          ? new THREE.Color(T.traversalPulse)
          : new THREE.Color(0x000000),
        emissiveIntensity: isTraversal ? 0.5 : 0,
        transparent: true,
        opacity: isTraversal ? 1 : 0.8,
      });

      const mesh = new THREE.Mesh(geo, mat);
      mesh.position.set(positions[i].x, positions[i].y, positions[i].z);
      mesh.userData = { node, index: i };
      scene.add(mesh);
      nodeMeshes.push(mesh);

      // settled 外壳（蓝色半透明球）
      if (node.settled) {
        const shellGeo = new THREE.SphereGeometry(r + 6, 12, 8);
        const shellMat = new THREE.MeshPhongMaterial({
          color: new THREE.Color(T.settled),
          transparent: true,
          opacity: 0.15,
          side: THREE.BackSide,
        });
        const shell = new THREE.Mesh(shellGeo, shellMat);
        mesh.add(shell);
      }
    });

    // ── 边线 ──────────────────────────────────────────────────
    const edgeMeshes: THREE.Line[] = [];
    links.forEach((link) => {
      const srcId = typeof link.source === "string" ? link.source : (link.source as TopologyNode).id;
      const tgtId = typeof link.target === "string" ? link.target : (link.target as TopologyNode).id;
      const si = nodeMap.get(srcId);
      const ti = nodeMap.get(tgtId);
      if (si === undefined || ti === undefined) return;

      const points = [
        new THREE.Vector3(positions[si].x, positions[si].y, positions[si].z),
        new THREE.Vector3(positions[ti].x, positions[ti].y, positions[ti].z),
      ];
      const geo = new THREE.BufferGeometry().setFromPoints(points);
      const isNegation = link.type === "negation";
      const mat = new THREE.LineBasicMaterial({
        color: isNegation ? new THREE.Color(T.fHigh) : new THREE.Color(T.textMuted),
        transparent: true,
        opacity: isNegation ? 0.3 : 0.12,
      });
      const line = new THREE.Line(geo, mat);
      scene.add(line);
      edgeMeshes.push(line);
    });

    // ── 鼠标交互：旋转 + 缩放 ────────────────────────────────
    let isDragging = false;
    let prevMouse = { x: 0, y: 0 };
    const spherical = new THREE.Spherical(
      camera.position.length(),
      Math.PI / 2, 0
    );

    const onMouseDown = (e: MouseEvent) => {
      isDragging = true;
      prevMouse = { x: e.clientX, y: e.clientY };
    };
    const onMouseMove = (e: MouseEvent) => {
      if (!isDragging) return;
      const dx = e.clientX - prevMouse.x;
      const dy = e.clientY - prevMouse.y;
      spherical.theta -= dx * 0.008;
      spherical.phi = Math.max(0.1, Math.min(Math.PI - 0.1, spherical.phi + dy * 0.008));
      prevMouse = { x: e.clientX, y: e.clientY };
      camera.position.setFromSpherical(spherical);
      camera.lookAt(0, 0, 0);
    };
    const onMouseUp = () => { isDragging = false; };
    const onWheel = (e: WheelEvent) => {
      spherical.radius = Math.max(50, Math.min(2000, spherical.radius + e.deltaY * 0.5));
      camera.position.setFromSpherical(spherical);
      camera.lookAt(0, 0, 0);
    };

    // 点击：射线检测
    const raycaster = new THREE.Raycaster();
    const mouse = new THREE.Vector2();
    const onCanvasClick = (e: MouseEvent) => {
      if (!onSelectNode) return;
      const rect = renderer.domElement.getBoundingClientRect();
      mouse.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      mouse.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
      raycaster.setFromCamera(mouse, camera);
      const hits = raycaster.intersectObjects(nodeMeshes);
      if (hits.length > 0) {
        const nodeData = hits[0].object.userData.node as TopologyNode;
        if (nodeData) onSelectNode(nodeData);
      }
    };

    // 悬浮检测
    const onCanvasHover = (e: MouseEvent) => {
      const rect = renderer.domElement.getBoundingClientRect();
      mouse.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      mouse.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
      raycaster.setFromCamera(mouse, camera);
      const hits = raycaster.intersectObjects(nodeMeshes);
      if (hits.length > 0) {
        const nodeData = hits[0].object.userData.node as TopologyNode;
        setHoveredNode(nodeData);
      } else {
        setHoveredNode(null);
      }
    };

    renderer.domElement.addEventListener("mousedown", onMouseDown);
    renderer.domElement.addEventListener("mousemove", onMouseMove);
    renderer.domElement.addEventListener("mousemove", onCanvasHover);
    renderer.domElement.addEventListener("mouseup", onMouseUp);
    renderer.domElement.addEventListener("wheel", onWheel, { passive: true });
    renderer.domElement.addEventListener("click", onCanvasClick);

    // ── 动画循环 ──────────────────────────────────────────────
    let animId: number;
    let tick = 0;
    const animate = () => {
      animId = requestAnimationFrame(animate);
      tick++;

      // 持续布局（慢速冷却）
      if (tick < 300) {
        const alpha = Math.max(0.001, (300 - tick) / 300 * 0.1);
        runLayoutStep(positions, velocities, nodes, links, alpha);

        // 更新节点位置
        nodeMeshes.forEach((mesh, i) => {
          mesh.position.set(positions[i].x, positions[i].y, positions[i].z);
        });

        // 更新边
        let edgeIdx = 0;
        links.forEach((link) => {
          const srcId = typeof link.source === "string" ? link.source : (link.source as TopologyNode).id;
          const tgtId = typeof link.target === "string" ? link.target : (link.target as TopologyNode).id;
          const si = nodeMap.get(srcId);
          const ti = nodeMap.get(tgtId);
          if (si === undefined || ti === undefined) return;
          const line = edgeMeshes[edgeIdx++];
          if (!line) return;
          const pos = line.geometry.attributes.position as THREE.BufferAttribute;
          pos.setXYZ(0, positions[si].x, positions[si].y, positions[si].z);
          pos.setXYZ(1, positions[ti].x, positions[ti].y, positions[ti].z);
          pos.needsUpdate = true;
        });
      }

      // 穿越节点脉冲
      const traversalIdx = nodes.findIndex((n) => n.id === traversalPosition);
      if (traversalIdx >= 0) {
        const mesh = nodeMeshes[traversalIdx];
        const mat = mesh.material as THREE.MeshPhongMaterial;
        mat.emissiveIntensity = 0.3 + 0.3 * Math.sin(tick * 0.08);
      }

      renderer.render(scene, camera);
    };
    animate();

    // ── 响应窗口大小 ──────────────────────────────────────────
    const onResize = () => {
      const w = mount.clientWidth || mount.offsetWidth || W;
      const h = mount.clientHeight || mount.offsetHeight || H;
      if (w === 0 || h === 0) return;
      renderer.setSize(w, h);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    };
    window.addEventListener("resize", onResize);

    // ── 清理 ─────────────────────────────────────────────────
    return () => {
      cancelAnimationFrame(animId);
      renderer.domElement.removeEventListener("mousedown", onMouseDown);
      renderer.domElement.removeEventListener("mousemove", onMouseMove);
      renderer.domElement.removeEventListener("mousemove", onCanvasHover);
      renderer.domElement.removeEventListener("mouseup", onMouseUp);
      renderer.domElement.removeEventListener("wheel", onWheel);
      renderer.domElement.removeEventListener("click", onCanvasClick);
      window.removeEventListener("resize", onResize);
      renderer.dispose();
      if (mount.contains(renderer.domElement)) {
        mount.removeChild(renderer.domElement);
      }
    };
  }, [data, traversalPosition, focusConcept, onSelectNode]);

  return (
    <div style={{ position: "relative", width: "100%", height: "100%" }}>
      <div ref={mountRef} style={{ width: "100%", height: "100%" }} />

      {/* 操作提示 */}
      <div style={{
        position: "absolute", top: 12, left: 12,
        background: T.bgCard + "aa",
        border: `1px solid ${T.border}`,
        borderRadius: 4, padding: "4px 8px",
        fontFamily: FONT.mono, fontSize: 8, color: T.textMuted,
        pointerEvents: "none",
      }}>
        拖拽旋转 · 滚轮缩放 · 点击查询
      </div>

      {/* 图例 */}
      <div style={{
        position: "absolute", top: 12, right: 12,
        background: T.bgCard + "cc",
        border: `1px solid ${T.border}`,
        borderRadius: 6, padding: "8px 12px",
        fontFamily: FONT.mono, fontSize: 9, color: T.textDim,
        display: "flex", gap: 12,
        pointerEvents: "none",
      }}>
        <span><span style={{ color: T.fZero }}>●</span> f=0</span>
        <span><span style={{ color: T.fLow }}>●</span> fold</span>
        <span><span style={{ color: T.fMid }}>●</span> gray</span>
        <span><span style={{ color: T.fHigh }}>●</span> negate</span>
        <span><span style={{ color: T.settled }}>○</span> settled</span>
        <span><span style={{ color: T.traversalPulse }}>◉</span> here</span>
      </div>

      {/* 节点数据 meta */}
      {data?.meta && (
        <div style={{
          position: "absolute", bottom: 12, right: 12,
          background: T.bgCard + "cc",
          border: `1px solid ${T.border}`,
          borderRadius: 4, padding: "4px 8px",
          fontFamily: FONT.mono, fontSize: 9, color: T.textMuted,
          pointerEvents: "none",
        }}>
          3D · {data.meta.shown_vertices.toLocaleString()}V / {data.meta.total_vertices.toLocaleString()} total
        </div>
      )}

      {/* 悬浮 tooltip */}
      {hoveredNode && (
        <div style={{
          position: "absolute", bottom: 12, left: 12,
          background: T.bgCard + "ee",
          border: `1px solid ${T.border}`,
          borderRadius: 6, padding: "8px 12px",
          fontFamily: FONT.mono, fontSize: 11, color: T.text,
          backdropFilter: "blur(8px)",
          pointerEvents: "none",
        }}>
          <span style={{ color: fToColor(hoveredNode.f_avg), fontWeight: 600 }}>
            {hoveredNode.label.slice(0, 50)}
          </span>
          <span style={{ color: T.textDim, marginLeft: 12 }}>
            f={hoveredNode.f_avg.toFixed(1)}
          </span>
          <span style={{ color: T.textDim, marginLeft: 8 }}>
            deg={hoveredNode.degree}
          </span>
          {hoveredNode.settled && (
            <span style={{ color: T.settled, marginLeft: 8 }}>● settled</span>
          )}
        </div>
      )}
    </div>
  );
}
