/**
 * ForceGraph3DV2.tsx — 深空观测站
 *
 * 设计语言：生物发光暗场，引力井聚类，有机边曲线，穿越拖尾粒子。
 *
 * 渲染层：
 *   - 星空粒子：Points（深蓝背景随机星点）
 *   - 聚类气泡：SphereGeometry + AdditiveBlending（按 source type 分区）
 *   - 节点：InstancedMesh（单 draw call，100 节点）
 *   - Halo：InstancedMesh（settled 节点蓝环，billboard 朝向相机）
 *   - 边：LineSegments（批量几何体，negation 边有 dash 动画）
 *   - label：CSS2DObject（DOM overlay，hover 时显示）
 *   - 拖尾粒子：Points（逢亮最近 N 帧位置，additive blend）
 *
 * 交互层（OrbitControls）：
 *   - 左键拖拽 → 围绕场景中心旋转
 *   - 右键拖拽 → 平移
 *   - 滚轮 → 惯性缩放
 *   - 点击节点 → 相机飞行 + 选中回调
 *   - 气泡点击 → 区域模式
 *   - 双击空白 → 全景复位
 */

import { useEffect, useRef, useState, useCallback } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { CSS2DRenderer, CSS2DObject } from "three/examples/jsm/renderers/CSS2DRenderer.js";
import { T, FONT, fToColor } from "../tokens";
import type { TopologyNode, TopologyLink, TopologyResponse } from "../types";

// ── 类型 ──────────────────────────────────────────────────────────

interface Props {
  data: TopologyResponse | null;
  traversalPosition?: string;
  focusConcept?: string | null;
  onSelectNode?: (node: TopologyNode) => void;
}

interface Vec3 { x: number; y: number; z: number }

// ── 颜色常量 ──────────────────────────────────────────────────────

const COL = {
  bg: 0x0a0a1a,           // 深蓝（替换纯黑）
  foldZone: 0x22d68a,     // 青绿
  grayZone: 0xf0c040,     // 琥珀
  negZone: 0xff4466,      // 朱红
  fZero: 0x00ffcc,        // 青白
  settled: 0x4488ff,      // 静脉蓝
  traversal: 0xffffff,    // 纯白

  // 聚类气泡色
  bubbleText: 0x22d68a,   // 哲学/文本
  bubbleCode: 0x4488ff,   // 代码
  bubbleSystem: 0x7c6cff, // 谱系/system
  bubbleOther: 0x334455,  // 其他

  negEdge: 0xff3355,
  depEdge: 0x2a3a5a,      // 稍亮（原 0x1e2840 太暗）
} as const;

// ── 力导向布局 ────────────────────────────────────────────────────

const DAMPING = 0.87;
const REPULSION = 20000;
const SPRING_K = 0.004;
const SPRING_REST = 90;
const MAX_SPEED = 7;
const SIM_STEPS = 140;
const CLUSTER_STRENGTH = 0.012;

type SourceType = "text" | "code" | "system" | "other";

function getSourceType(node: TopologyNode): SourceType {
  if (node.type === "text") return "text";
  if (node.type === "code") return "code";
  if (node.type === "system") return "system";
  return "other";
}

// 聚类中心（固定在空间中）
const CLUSTER_CENTERS: Record<SourceType, Vec3> = {
  text:   { x: -120, y: 60,  z: 0   },
  code:   { x: 120,  y: -60, z: 30  },
  system: { x: 0,    y: -80, z: -60 },
  other:  { x: 60,   y: 100, z: -30 },
};

function initLayout(nodes: TopologyNode[], size = 280): Vec3[] {
  return nodes.map((n) => {
    const center = CLUSTER_CENTERS[getSourceType(n)];
    return {
      x: center.x + (Math.random() - 0.5) * size * 0.5,
      y: center.y + (Math.random() - 0.5) * size * 0.5,
      z: center.z + (Math.random() - 0.5) * size * 0.5,
    };
  });
}

function runLayoutStep(
  positions: Vec3[], velocities: Vec3[],
  nodes: TopologyNode[], links: TopologyLink[], alpha: number
): void {
  const N = nodes.length;
  if (N === 0) return;

  // 斥力 O(N²)
  for (let i = 0; i < N; i++) {
    for (let j = i + 1; j < N; j++) {
      const dx = positions[j].x - positions[i].x;
      const dy = positions[j].y - positions[i].y;
      const dz = positions[j].z - positions[i].z;
      const d2 = dx * dx + dy * dy + dz * dz + 1;
      const force = REPULSION * alpha / d2;
      velocities[i].x -= force * dx;
      velocities[i].y -= force * dy;
      velocities[i].z -= force * dz;
      velocities[j].x += force * dx;
      velocities[j].y += force * dy;
      velocities[j].z += force * dz;
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
    velocities[i].x += (force * dx) / d;
    velocities[i].y += (force * dy) / d;
    velocities[i].z += (force * dz) / d;
    velocities[j].x -= (force * dx) / d;
    velocities[j].y -= (force * dy) / d;
    velocities[j].z -= (force * dz) / d;
  }

  // 聚类引力
  for (let i = 0; i < N; i++) {
    const center = CLUSTER_CENTERS[getSourceType(nodes[i])];
    velocities[i].x += (center.x - positions[i].x) * CLUSTER_STRENGTH * alpha;
    velocities[i].y += (center.y - positions[i].y) * CLUSTER_STRENGTH * alpha;
    velocities[i].z += (center.z - positions[i].z) * CLUSTER_STRENGTH * alpha;
  }

  // 速度限制 + 阻尼 + 位置更新
  for (let i = 0; i < N; i++) {
    velocities[i].x *= DAMPING;
    velocities[i].y *= DAMPING;
    velocities[i].z *= DAMPING;
    const speed = Math.sqrt(velocities[i].x ** 2 + velocities[i].y ** 2 + velocities[i].z ** 2);
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

// ── f 值 → Three.js hex color ─────────────────────────────────────

function fToThreeColor(f: number): number {
  if (f < 0) return COL.depEdge;
  if (f <= 0.5) return COL.fZero;
  if (f < 5) return COL.foldZone;
  if (f < 12) return COL.grayZone;
  return COL.negZone;
}

// 节点半径（对数比例）
function nodeRadius(degree: number, isTraversal: boolean, isFocus: boolean): number {
  if (isTraversal) return 10;
  if (isFocus) return 9;
  return 2.5 + Math.log1p(Math.min(degree, 200)) * 1.6;
}

// ── 相机飞行工具 ──────────────────────────────────────────────────

function easeInOut(t: number): number {
  return t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t;
}

// ── settled halo GLSL ─────────────────────────────────────────────
// 注意：halo 用 PlaneGeometry + billboard，instanceMatrix 由 CPU compose

const haloVertGLSL = `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = projectionMatrix * modelViewMatrix * instanceMatrix * vec4(position, 1.0);
  }
`;
const haloFragGLSL = `
  uniform vec3 uColor;
  uniform float uTime;
  varying vec2 vUv;
  void main() {
    float d = length(vUv - 0.5) * 2.0;
    float outer = 1.0 - smoothstep(0.80, 0.84, d);
    float inner = smoothstep(0.62, 0.66, d);
    float ring = outer * inner;
    float pulse = 0.5 + 0.5 * sin(uTime * 1.4);
    float alpha = ring * (0.65 + 0.35 * pulse);
    gl_FragColor = vec4(uColor, alpha);
  }
`;

// ── 星空粒子生成 ──────────────────────────────────────────────────

function buildStarField(count: number, spread: number): THREE.Points {
  const positions = new Float32Array(count * 3);
  const colors = new Float32Array(count * 3);
  for (let i = 0; i < count; i++) {
    // 球面均匀分布
    const theta = Math.random() * Math.PI * 2;
    const phi = Math.acos(2 * Math.random() - 1);
    const r = spread * (0.7 + Math.random() * 0.3);
    positions[i * 3]     = r * Math.sin(phi) * Math.cos(theta);
    positions[i * 3 + 1] = r * Math.sin(phi) * Math.sin(theta);
    positions[i * 3 + 2] = r * Math.cos(phi);

    // 颜色：偏蓝白
    const brightness = 0.4 + Math.random() * 0.6;
    colors[i * 3]     = brightness * 0.85;
    colors[i * 3 + 1] = brightness * 0.90;
    colors[i * 3 + 2] = brightness;
  }
  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.BufferAttribute(positions, 3));
  geo.setAttribute("color", new THREE.BufferAttribute(colors, 3));
  const mat = new THREE.PointsMaterial({
    size: 1.2,
    vertexColors: true,
    transparent: true,
    opacity: 0.75,
    depthWrite: false,
    sizeAttenuation: true,
  });
  return new THREE.Points(geo, mat);
}

// ── 主组件 ────────────────────────────────────────────────────────

export function ForceGraph3DV2({
  data, traversalPosition, focusConcept, onSelectNode,
}: Props) {
  const mountRef = useRef<HTMLDivElement>(null);
  const labelRootRef = useRef<HTMLDivElement>(null);
  const [hoveredNode, setHoveredNode] = useState<TopologyNode | null>(null);
  const [activeCluster, setActiveCluster] = useState<SourceType | null>(null);

  // traversalPosition 通过 ref 传递给动画循环，避免重建场景
  const traversalRef = useRef(traversalPosition);
  traversalRef.current = traversalPosition;

  const handleClusterClick = useCallback((src: SourceType) => {
    setActiveCluster((prev) => (prev === src ? null : src));
  }, []);

  useEffect(() => {
    const mount = mountRef.current;
    const labelRoot = labelRootRef.current;
    if (!mount || !labelRoot || !data || data.nodes.length === 0) return;

    const W = mount.clientWidth || 800;
    const H = mount.clientHeight || 600;
    if (W === 0 || H === 0) return;

    // ── Scene ─────────────────────────────────────────────────────
    const scene = new THREE.Scene();
    scene.background = new THREE.Color(COL.bg);
    scene.fog = new THREE.FogExp2(COL.bg, 0.0014);

    // ── Camera ────────────────────────────────────────────────────
    const camera = new THREE.PerspectiveCamera(55, W / H, 0.5, 6000);
    camera.position.set(0, 0, 480);

    // ── WebGL Renderer ────────────────────────────────────────────
    const renderer = new THREE.WebGLRenderer({
      antialias: true,
      alpha: false,
      powerPreference: "high-performance",
    });
    renderer.setSize(W, H);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.4;   // 提高曝光（原 1.1）
    mount.appendChild(renderer.domElement);

    // ── CSS2D Renderer（label overlay） ───────────────────────────
    const css2d = new CSS2DRenderer();
    css2d.setSize(W, H);
    css2d.domElement.style.position = "absolute";
    css2d.domElement.style.top = "0";
    css2d.domElement.style.left = "0";
    css2d.domElement.style.pointerEvents = "none";
    labelRoot.appendChild(css2d.domElement);

    // ── Lights ────────────────────────────────────────────────────
    // AmbientLight：提高整体底色亮度（原 0x0a0a20 强度1.0 几乎全黑）
    scene.add(new THREE.AmbientLight(0x334466, 2.5));

    // HemisphereLight：冷色天光 + 暖色地光，整体照明
    const hemi = new THREE.HemisphereLight(0x4488ff, 0x1a1a2e, 1.2);
    scene.add(hemi);

    // 点光源（更高强度，覆盖更大范围）
    const pointA = new THREE.PointLight(0x6699ff, 3.5, 800);
    pointA.position.set(-150, 80, 100);
    scene.add(pointA);
    const pointB = new THREE.PointLight(0x22d68a, 2.2, 700);
    pointB.position.set(150, -60, -80);
    scene.add(pointB);
    const pointC = new THREE.PointLight(0xff4466, 1.5, 600);
    pointC.position.set(0, -150, 80);
    scene.add(pointC);

    // ── 星空背景粒子 ──────────────────────────────────────────────
    const starField = buildStarField(1200, 2800);
    scene.add(starField);

    // ── OrbitControls（替换手写球坐标旋转） ──────────────────────
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;       // 平滑惯性
    controls.dampingFactor = 0.08;       // 阻尼系数
    controls.rotateSpeed = 0.6;          // 旋转速度
    controls.panSpeed = 0.8;             // 平移速度
    controls.zoomSpeed = 1.0;            // 缩放速度
    controls.minDistance = 40;           // 最近距离
    controls.maxDistance = 2200;         // 最远距离
    controls.target.set(0, 0, 0);        // 围绕场景中心旋转
    controls.mouseButtons = {
      LEFT: THREE.MOUSE.ROTATE,
      MIDDLE: THREE.MOUSE.DOLLY,
      RIGHT: THREE.MOUSE.PAN,
    };
    controls.update();

    // ── 布局计算 ──────────────────────────────────────────────────
    const nodes = data.nodes;
    const links = data.links;
    const positions = initLayout(nodes);
    const velocities: Vec3[] = nodes.map(() => ({ x: 0, y: 0, z: 0 }));

    for (let step = 0; step < SIM_STEPS; step++) {
      const alpha = Math.max(0.005, 1 - step / SIM_STEPS);
      runLayoutStep(positions, velocities, nodes, links, alpha);
    }

    // ── InstancedMesh：节点球体 ───────────────────────────────────
    const N = nodes.length;
    const nodeGeo = new THREE.SphereGeometry(1, 16, 10);
    const nodeMat = new THREE.MeshPhongMaterial({
      vertexColors: false,
      transparent: true,
      opacity: 0.95,
      emissive: new THREE.Color(0x111122),    // 基础自发光（暗）
      emissiveIntensity: 0.6,                  // 由动画循环按节点类型覆盖
      shininess: 80,
      specular: new THREE.Color(0x336688),
    });
    const nodeInstanced = new THREE.InstancedMesh(nodeGeo, nodeMat, N);
    nodeInstanced.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
    nodeInstanced.instanceColor = new THREE.InstancedBufferAttribute(new Float32Array(N * 3), 3);

    const nodeIdxMap = new Map<string, number>();
    const nodeRadii: number[] = [];
    const nodePhases: number[] = [];
    const labelObjects: CSS2DObject[] = [];

    nodes.forEach((node, i) => {
      nodeIdxMap.set(node.id, i);
      const isTraversal = node.id === traversalRef.current;
      const isFocus = node.label === focusConcept;
      const r = nodeRadius(node.degree, isTraversal, isFocus);
      nodeRadii.push(r);
      nodePhases.push(Math.random() * Math.PI * 2);

      const m = new THREE.Matrix4();
      m.makeScale(r, r, r);
      m.setPosition(positions[i].x, positions[i].y, positions[i].z);
      nodeInstanced.setMatrixAt(i, m);
      nodeInstanced.setColorAt(
        i,
        new THREE.Color(isTraversal ? COL.traversal : fToThreeColor(node.f_avg))
      );

      // CSS2D label（初始透明）
      const div = document.createElement("div");
      div.style.cssText = `
        font-family: ${FONT.mono};
        font-size: 9px;
        color: ${fToColor(node.f_avg)};
        background: rgba(8,8,20,0.88);
        border: 1px solid rgba(68,136,255,0.35);
        border-radius: 3px;
        padding: 2px 6px;
        white-space: nowrap;
        pointer-events: none;
        opacity: 0;
        transition: opacity 0.15s;
        user-select: none;
        text-shadow: 0 0 8px currentColor;
      `;
      div.textContent = node.label.length > 28 ? node.label.slice(0, 27) + "…" : node.label;
      const label = new CSS2DObject(div);
      label.position.set(0, r + 2, 0);
      labelObjects.push(label);
    });

    nodeInstanced.instanceMatrix.needsUpdate = true;
    if (nodeInstanced.instanceColor) nodeInstanced.instanceColor.needsUpdate = true;
    scene.add(nodeInstanced);

    // ── settled halo（InstancedMesh，billboard shader） ───────────
    const settledIndices = nodes
      .map((n, i) => (n.settled ? i : -1))
      .filter((i) => i >= 0);
    const Ns = settledIndices.length;
    let settledInstanced: THREE.InstancedMesh | null = null;

    if (Ns > 0) {
      const haloGeo = new THREE.PlaneGeometry(1, 1);
      const haloMat = new THREE.ShaderMaterial({
        vertexShader: haloVertGLSL,
        fragmentShader: haloFragGLSL,
        uniforms: {
          uColor: { value: new THREE.Color(COL.settled) },
          uTime:  { value: 0 },
        },
        transparent: true,
        depthWrite: false,
        side: THREE.DoubleSide,
      });
      settledInstanced = new THREE.InstancedMesh(haloGeo, haloMat, Ns);
      settledInstanced.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
      // 初始矩阵（不含旋转，动画循环每帧更新为 billboard）
      settledIndices.forEach((ni, si) => {
        const r = nodeRadii[ni] * 3.2;
        const m = new THREE.Matrix4();
        m.makeScale(r, r, r);
        m.setPosition(positions[ni].x, positions[ni].y, positions[ni].z);
        settledInstanced!.setMatrixAt(si, m);
      });
      settledInstanced.instanceMatrix.needsUpdate = true;
      scene.add(settledInstanced);
    }

    // ── 聚类气泡 ─────────────────────────────────────────────────
    const clusterTypes: SourceType[] = ["text", "code", "system", "other"];
    const bubbleColorMap: Record<SourceType, number> = {
      text: COL.bubbleText, code: COL.bubbleCode,
      system: COL.bubbleSystem, other: COL.bubbleOther,
    };
    const bubbleLabelMap: Record<SourceType, string> = {
      text: "哲学/文本", code: "代码", system: "谱系", other: "其他",
    };

    const bubbleMeshes: Array<{ mesh: THREE.Mesh; src: SourceType }> = [];

    clusterTypes.forEach((src) => {
      const center = CLUSTER_CENTERS[src];
      const members = nodes.filter((n) => getSourceType(n) === src);
      if (members.length === 0) return;

      let maxDist = 60;
      members.forEach((n) => {
        const ni = nodes.indexOf(n);
        const dx = positions[ni].x - center.x;
        const dy = positions[ni].y - center.y;
        const dz = positions[ni].z - center.z;
        maxDist = Math.max(maxDist, Math.sqrt(dx * dx + dy * dy + dz * dz) + 20);
      });
      const bubbleRadius = Math.min(maxDist, 160);

      const bubbleGeo = new THREE.SphereGeometry(bubbleRadius, 32, 24);
      const bubbleMat = new THREE.MeshBasicMaterial({
        color: new THREE.Color(bubbleColorMap[src]),
        transparent: true,
        opacity: 0.07,           // 提高（原 0.04 几乎不可见）
        side: THREE.BackSide,
        depthWrite: false,
        blending: THREE.AdditiveBlending,
      });
      const bubbleMesh = new THREE.Mesh(bubbleGeo, bubbleMat);
      bubbleMesh.position.set(center.x, center.y, center.z);
      bubbleMesh.userData = { isBubble: true, src };
      scene.add(bubbleMesh);
      bubbleMeshes.push({ mesh: bubbleMesh, src });

      // 气泡线框（提高可见度）
      const edgesGeo = new THREE.EdgesGeometry(
        new THREE.SphereGeometry(bubbleRadius, 16, 12)
      );
      const edgesMat = new THREE.LineBasicMaterial({
        color: new THREE.Color(bubbleColorMap[src]),
        transparent: true, opacity: 0.12,   // 提高（原 0.06）
      });
      const edgeLines = new THREE.LineSegments(edgesGeo, edgesMat);
      edgeLines.position.set(center.x, center.y, center.z);
      scene.add(edgeLines);

      // 气泡标签
      const labelDiv = document.createElement("div");
      const rgba =
        src === "text"   ? "34,214,138" :
        src === "code"   ? "68,136,255" :
        src === "system" ? "124,108,255" : "80,100,120";
      labelDiv.style.cssText = `
        font-family: ${FONT.mono};
        font-size: 8px;
        letter-spacing: 0.12em;
        color: rgba(${rgba}, 0.75);
        text-transform: uppercase;
        pointer-events: none;
        user-select: none;
        text-shadow: 0 0 6px rgba(${rgba}, 0.5);
      `;
      labelDiv.textContent = bubbleLabelMap[src];
      const bubbleLabel = new CSS2DObject(labelDiv);
      bubbleLabel.position.set(center.x, center.y + bubbleRadius * 0.85, center.z);
      scene.add(bubbleLabel);
    });

    // ── 边（LineSegments 批量） ───────────────────────────────────
    const negLinks: TopologyLink[] = [];
    const depLinks: TopologyLink[] = [];
    links.forEach((l) => {
      if (l.type === "negation") negLinks.push(l);
      else depLinks.push(l);
    });

    function buildLineSegs(
      ls: TopologyLink[], color: number, opacity: number
    ): THREE.LineSegments | null {
      if (ls.length === 0) return null;
      const pts: number[] = [];
      ls.forEach((l) => {
        const srcId = typeof l.source === "string" ? l.source : (l.source as TopologyNode).id;
        const tgtId = typeof l.target === "string" ? l.target : (l.target as TopologyNode).id;
        const si = nodeIdxMap.get(srcId);
        const ti = nodeIdxMap.get(tgtId);
        if (si === undefined || ti === undefined) return;
        pts.push(
          positions[si].x, positions[si].y, positions[si].z,
          positions[ti].x, positions[ti].y, positions[ti].z,
        );
      });
      const geo = new THREE.BufferGeometry();
      geo.setAttribute("position", new THREE.Float32BufferAttribute(pts, 3));
      const mat = new THREE.LineBasicMaterial({
        color: new THREE.Color(color),
        transparent: true, opacity, depthWrite: false,
      });
      return new THREE.LineSegments(geo, mat);
    }

    const negLineSegs = buildLineSegs(negLinks, COL.negEdge, 0.55);    // 提高（原 0.28）
    const depLineSegs = buildLineSegs(depLinks, COL.depEdge, 0.30);    // 提高（原 0.15）
    if (negLineSegs) scene.add(negLineSegs);
    if (depLineSegs) scene.add(depLineSegs);

    // negation 脉冲点（每条 negation 边一个移动粒子）
    const pulsePoints: Array<{
      point: THREE.Points;
      srcIdx: number;
      tgtIdx: number;
      phase: number;
    }> = [];
    negLinks.forEach((l, li) => {
      const srcId = typeof l.source === "string" ? l.source : (l.source as TopologyNode).id;
      const tgtId = typeof l.target === "string" ? l.target : (l.target as TopologyNode).id;
      const si = nodeIdxMap.get(srcId);
      const ti = nodeIdxMap.get(tgtId);
      if (si === undefined || ti === undefined) return;
      const geo = new THREE.BufferGeometry();
      geo.setAttribute("position", new THREE.Float32BufferAttribute([0, 0, 0], 3));
      const mat = new THREE.PointsMaterial({
        color: new THREE.Color(COL.negEdge),
        size: 4.0, transparent: true, opacity: 0.92,
        depthWrite: false, sizeAttenuation: true,
      });
      const pt = new THREE.Points(geo, mat);
      scene.add(pt);
      pulsePoints.push({ point: pt, srcIdx: si, tgtIdx: ti, phase: li * 0.37 });
    });

    // ── 穿越拖尾粒子 ─────────────────────────────────────────────
    const TRAIL_LEN = 10;
    const trailHistory: THREE.Vector3[] = [];
    const trailGeo = new THREE.BufferGeometry();
    const trailPosBuf = new Float32Array(TRAIL_LEN * 3);
    const trailColBuf = new Float32Array(TRAIL_LEN * 3);
    trailGeo.setAttribute("position", new THREE.BufferAttribute(trailPosBuf, 3));
    trailGeo.setAttribute("color", new THREE.BufferAttribute(trailColBuf, 3));
    const trailMat = new THREE.PointsMaterial({
      size: 4, vertexColors: true, transparent: true,
      depthWrite: false, sizeAttenuation: true,
      blending: THREE.AdditiveBlending,
    });
    scene.add(new THREE.Points(trailGeo, trailMat));

    const traversalLight = new THREE.PointLight(0xaaddff, 0, 120);
    scene.add(traversalLight);

    // ── 鼠标交互状态（节点 hover + click 检测，兼容 OrbitControls） ──
    let mouseDownPos = { x: 0, y: 0 };
    let mouseMoved = false;
    let hoveredIdx = -1;
    let neighborSet = new Set<number>();

    // hover 状态
    const nodeScales = new Float32Array(N).fill(1.0);

    function computeNeighbors(idx: number): Set<number> {
      const s = new Set<number>();
      const nodeId = nodes[idx].id;
      links.forEach((l) => {
        const srcId = typeof l.source === "string" ? l.source : (l.source as TopologyNode).id;
        const tgtId = typeof l.target === "string" ? l.target : (l.target as TopologyNode).id;
        if (srcId === nodeId) { const ti = nodeIdxMap.get(tgtId); if (ti !== undefined) s.add(ti); }
        if (tgtId === nodeId) { const si = nodeIdxMap.get(srcId); if (si !== undefined) s.add(si); }
      });
      return s;
    }

    const raycaster = new THREE.Raycaster();
    const mouseNDC = new THREE.Vector2();

    // 相机飞行（独立于 OrbitControls，通过 enabled 切换）
    let flyFrom: THREE.Vector3 | null = null;
    let flyTo: THREE.Vector3 | null = null;
    let flyLookFrom: THREE.Vector3 | null = null;
    let flyLookTo: THREE.Vector3 | null = null;
    let flyT = 1;

    function startFly(
      targetPos: THREE.Vector3,
      lookAt: THREE.Vector3 = new THREE.Vector3(0, 0, 0)
    ) {
      flyFrom = camera.position.clone();
      flyTo = targetPos.clone();
      flyLookFrom = controls.target.clone();
      flyLookTo = lookAt.clone();
      flyT = 0;
      controls.enabled = false;   // 飞行期间禁用 OrbitControls
    }

    const onMouseDown = (e: MouseEvent) => {
      mouseDownPos = { x: e.clientX, y: e.clientY };
      mouseMoved = false;
    };

    const onMouseMove = (e: MouseEvent) => {
      const dx = e.clientX - mouseDownPos.x;
      const dy = e.clientY - mouseDownPos.y;
      if (Math.abs(dx) + Math.abs(dy) > 3) mouseMoved = true;

      // hover 检测
      const rect = renderer.domElement.getBoundingClientRect();
      mouseNDC.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      mouseNDC.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
      raycaster.setFromCamera(mouseNDC, camera);
      const hits = raycaster.intersectObject(nodeInstanced);

      if (hits.length > 0) {
        const idx = hits[0].instanceId!;
        if (idx !== hoveredIdx) {
          if (hoveredIdx >= 0 && labelObjects[hoveredIdx]) {
            labelObjects[hoveredIdx].element.style.opacity = "0";
            if (labelObjects[hoveredIdx].parent) scene.remove(labelObjects[hoveredIdx]);
          }
          hoveredIdx = idx;
          neighborSet = computeNeighbors(idx);
          setHoveredNode(nodes[idx]);
          if (labelObjects[idx]) {
            scene.add(labelObjects[idx]);
            setTimeout(() => {
              if (labelObjects[idx]) labelObjects[idx].element.style.opacity = "1";
            }, 10);
          }
        }
      } else {
        if (hoveredIdx >= 0) {
          if (labelObjects[hoveredIdx]) {
            labelObjects[hoveredIdx].element.style.opacity = "0";
            const capturedIdx = hoveredIdx;
            setTimeout(() => {
              if (labelObjects[capturedIdx]?.parent) scene.remove(labelObjects[capturedIdx]);
            }, 160);
          }
          hoveredIdx = -1;
          neighborSet = new Set();
          setHoveredNode(null);
        }
      }
    };

    const onMouseUp = () => { /* OrbitControls 处理旋转/平移 */ };

    const onDblClick = () => {
      startFly(new THREE.Vector3(0, 0, 480), new THREE.Vector3(0, 0, 0));
    };

    const onCanvasClick = (e: MouseEvent) => {
      if (mouseMoved) return;
      const rect = renderer.domElement.getBoundingClientRect();
      mouseNDC.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      mouseNDC.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
      raycaster.setFromCamera(mouseNDC, camera);

      // 节点点击
      const nodeHits = raycaster.intersectObject(nodeInstanced);
      if (nodeHits.length > 0) {
        const idx = nodeHits[0].instanceId!;
        const node = nodes[idx];
        if (node && onSelectNode) {
          onSelectNode(node);
          const target = new THREE.Vector3(positions[idx].x, positions[idx].y, positions[idx].z);
          const dir = target.clone().normalize();
          startFly(target.clone().add(dir.multiplyScalar(80)), target);
        }
        return;
      }

      // 气泡点击
      const bubbleMeshList = bubbleMeshes.map((b) => b.mesh);
      const bubbleHits = raycaster.intersectObjects(bubbleMeshList);
      if (bubbleHits.length > 0) {
        const src = bubbleHits[0].object.userData.src as SourceType;
        handleClusterClick(src);
        const center = CLUSTER_CENTERS[src];
        startFly(
          new THREE.Vector3(center.x, center.y, center.z + 200),
          new THREE.Vector3(center.x, center.y, center.z)
        );
      }
    };

    renderer.domElement.addEventListener("mousedown", onMouseDown);
    renderer.domElement.addEventListener("mousemove", onMouseMove);
    renderer.domElement.addEventListener("mouseup", onMouseUp);
    renderer.domElement.addEventListener("click", onCanvasClick);
    renderer.domElement.addEventListener("dblclick", onDblClick);

    // ── 动画循环 ──────────────────────────────────────────────────
    let animId: number;
    let tick = 0;
    const tmpMatrix = new THREE.Matrix4();
    const tmpColor = new THREE.Color();
    const tmpQuat = new THREE.Quaternion();
    const tmpScale3 = new THREE.Vector3();
    const tmpPos3 = new THREE.Vector3();

    const animate = () => {
      animId = requestAnimationFrame(animate);
      tick++;
      const t = tick * 0.016;

      // OrbitControls 阻尼更新
      controls.update();

      // 相机飞行（接管期间禁用 OrbitControls）
      if (flyT < 1 && flyFrom && flyTo) {
        flyT = Math.min(1, flyT + 0.025);
        const et = easeInOut(flyT);
        camera.position.lerpVectors(flyFrom, flyTo, et);
        if (flyLookFrom && flyLookTo) {
          const lx = flyLookFrom.x + (flyLookTo.x - flyLookFrom.x) * et;
          const ly = flyLookFrom.y + (flyLookTo.y - flyLookFrom.y) * et;
          const lz = flyLookFrom.z + (flyLookTo.z - flyLookFrom.z) * et;
          camera.lookAt(lx, ly, lz);
          controls.target.set(lx, ly, lz);
        } else {
          camera.lookAt(0, 0, 0);
          controls.target.set(0, 0, 0);
        }
        if (flyT >= 1) {
          controls.enabled = true;   // 飞行结束，恢复 OrbitControls
        }
      }

      // 持续布局冷却（前 200 帧）
      if (tick < 200) {
        const alpha = Math.max(0.001, (200 - tick) / 200 * 0.08);
        runLayoutStep(positions, velocities, nodes, links, alpha);
      }

      // 节点 InstancedMesh：呼吸 + hover 放大 + 邻域高亮 + 发光增强
      for (let i = 0; i < N; i++) {
        const isTraversal = nodes[i].id === traversalRef.current;
        const isHovered = i === hoveredIdx;
        const isNeighbor = neighborSet.has(i);

        let scale = 1.0 + 0.025 * Math.sin(t * 1.5 + nodePhases[i]);
        if (isTraversal) scale *= 1.0 + 0.2 * Math.sin(t * 3.0);

        // hover：放大目标 1.7x（原 1.45x），更明显
        nodeScales[i] = isHovered
          ? Math.min(nodeScales[i] + 0.07, 1.7)
          : Math.max(nodeScales[i] - 0.05, 1.0);
        scale *= nodeScales[i];

        const r = nodeRadii[i] * scale;
        tmpMatrix.makeScale(r, r, r);
        tmpMatrix.setPosition(positions[i].x, positions[i].y, positions[i].z);
        nodeInstanced.setMatrixAt(i, tmpMatrix);

        const baseHex = isTraversal ? COL.traversal : fToThreeColor(nodes[i].f_avg);
        tmpColor.setHex(baseHex);

        if (hoveredIdx >= 0 && !isHovered && !isNeighbor) {
          tmpColor.multiplyScalar(0.3);    // 非邻域节点压暗
        } else if (isHovered) {
          tmpColor.multiplyScalar(1.6);    // hover 节点大幅提亮（原 1.3）
        } else if (isNeighbor) {
          tmpColor.multiplyScalar(1.2);    // 邻域节点轻微提亮
        }
        nodeInstanced.setColorAt(i, tmpColor);
      }
      nodeInstanced.instanceMatrix.needsUpdate = true;
      if (nodeInstanced.instanceColor) nodeInstanced.instanceColor.needsUpdate = true;

      // hover 节点：增强 emissive 发光（通过材质 emissiveIntensity 动态调整）
      // 注意：InstancedMesh 共享材质，用 emissiveIntensity 做全局效果
      if (hoveredIdx >= 0) {
        nodeMat.emissiveIntensity = 0.9 + 0.2 * Math.sin(t * 4.0);
        nodeMat.emissive.setHex(fToThreeColor(nodes[hoveredIdx]?.f_avg ?? 0));
      } else {
        nodeMat.emissiveIntensity = 0.3;
        nodeMat.emissive.setHex(0x111122);
      }

      // settled halo：billboard（compose = position × camera_quat × scale）
      if (settledInstanced && Ns > 0) {
        const haloMat = settledInstanced.material as THREE.ShaderMaterial;
        haloMat.uniforms.uTime.value = t;

        // 相机四元数：halo 平面始终朝向相机
        camera.getWorldQuaternion(tmpQuat);

        settledIndices.forEach((ni, si) => {
          const r = nodeRadii[ni] * 3.2;
          tmpPos3.set(positions[ni].x, positions[ni].y, positions[ni].z);
          tmpScale3.set(r, r, r);
          tmpMatrix.compose(tmpPos3, tmpQuat, tmpScale3);
          settledInstanced!.setMatrixAt(si, tmpMatrix);
        });
        settledInstanced.instanceMatrix.needsUpdate = true;
      }

      // negation 脉冲点：沿边滑动
      pulsePoints.forEach(({ point, srcIdx, tgtIdx, phase }) => {
        const frac = ((t * 0.6 + phase) % 1);
        const sx = positions[srcIdx].x, sy = positions[srcIdx].y, sz = positions[srcIdx].z;
        const tx = positions[tgtIdx].x, ty = positions[tgtIdx].y, tz = positions[tgtIdx].z;
        const pos = point.geometry.attributes.position as THREE.BufferAttribute;
        pos.setXYZ(0, sx + (tx - sx) * frac, sy + (ty - sy) * frac, sz + (tz - sz) * frac);
        pos.needsUpdate = true;
        const distFrac = Math.abs(frac - 0.5) * 2;
        (point.material as THREE.PointsMaterial).opacity = 0.95 * (1 - distFrac * distFrac);
      });

      // 穿越拖尾
      const trvIdx = nodes.findIndex((n) => n.id === traversalRef.current);
      if (trvIdx >= 0) {
        const trvPos = new THREE.Vector3(
          positions[trvIdx].x, positions[trvIdx].y, positions[trvIdx].z
        );
        traversalLight.position.copy(trvPos);
        traversalLight.intensity = 1.5 + 0.5 * Math.sin(t * 4);

        if (
          trailHistory.length === 0 ||
          trailHistory[trailHistory.length - 1].distanceTo(trvPos) > 0.5
        ) {
          trailHistory.push(trvPos.clone());
          if (trailHistory.length > TRAIL_LEN) trailHistory.shift();
        }

        const posAttr = trailGeo.attributes.position as THREE.BufferAttribute;
        const colAttr = trailGeo.attributes.color as THREE.BufferAttribute;
        for (let k = 0; k < TRAIL_LEN; k++) {
          if (k < trailHistory.length) {
            const hp = trailHistory[trailHistory.length - 1 - k];
            posAttr.setXYZ(k, hp.x, hp.y, hp.z);
            const bright = 1 - k / TRAIL_LEN;
            colAttr.setXYZ(k, bright, bright, Math.min(1, bright * (1 + k * 0.15)));
          } else {
            posAttr.setXYZ(k, 0, 0, -99999);
            colAttr.setXYZ(k, 0, 0, 0);
          }
        }
        posAttr.needsUpdate = true;
        colAttr.needsUpdate = true;
        trailMat.opacity = 0.75;
      } else {
        traversalLight.intensity = 0;
        trailMat.opacity = 0;
      }

      // 气泡透明度（区域模式）
      bubbleMeshes.forEach(({ mesh, src }) => {
        const mat = mesh.material as THREE.MeshBasicMaterial;
        mat.opacity =
          activeCluster === null ? 0.07 :
          src === activeCluster ? 0.14 : 0.02;
      });

      renderer.render(scene, camera);
      css2d.render(scene, camera);
    };
    animate();

    // ── 窗口缩放 ──────────────────────────────────────────────────
    const onResize = () => {
      const w = mount.clientWidth || W;
      const h = mount.clientHeight || H;
      if (w === 0 || h === 0) return;
      renderer.setSize(w, h);
      css2d.setSize(w, h);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      controls.update();
    };
    window.addEventListener("resize", onResize);

    // ── 清理 ──────────────────────────────────────────────────────
    return () => {
      cancelAnimationFrame(animId);
      controls.dispose();
      renderer.domElement.removeEventListener("mousedown", onMouseDown);
      renderer.domElement.removeEventListener("mousemove", onMouseMove);
      renderer.domElement.removeEventListener("mouseup", onMouseUp);
      renderer.domElement.removeEventListener("click", onCanvasClick);
      renderer.domElement.removeEventListener("dblclick", onDblClick);
      window.removeEventListener("resize", onResize);
      nodeGeo.dispose();
      nodeMat.dispose();
      renderer.dispose();
      if (mount.contains(renderer.domElement)) mount.removeChild(renderer.domElement);
      if (labelRoot.contains(css2d.domElement)) labelRoot.removeChild(css2d.domElement);
      labelObjects.forEach((lo) => { if (lo.parent) scene.remove(lo); });
    };
  }, [data, focusConcept, onSelectNode, activeCluster, handleClusterClick]);

  return (
    <div style={{ position: "relative", width: "100%", height: "100%" }}>
      {/* Three.js canvas */}
      <div ref={mountRef} style={{ width: "100%", height: "100%", position: "absolute", inset: 0 }} />

      {/* CSS2D label overlay */}
      <div
        ref={labelRootRef}
        style={{ position: "absolute", inset: 0, pointerEvents: "none", overflow: "hidden" }}
      />

      {/* 操作提示 */}
      <div style={{
        position: "absolute", top: 10, left: 10,
        fontFamily: FONT.mono, fontSize: 8,
        color: "rgba(106,106,138,0.7)",
        letterSpacing: "0.08em",
        pointerEvents: "none",
        lineHeight: 1.8,
      }}>
        左键旋转 · 右键平移 · 滚轮缩放（惯性）<br />
        点击节点选中 · 点击气泡区域 · 双击复位
      </div>

      {/* 面包屑 */}
      {activeCluster && (
        <div style={{
          position: "absolute", top: 10, left: "50%",
          transform: "translateX(-50%)",
          fontFamily: FONT.mono, fontSize: 9,
          color: "rgba(124,108,255,0.8)",
          letterSpacing: "0.1em",
          pointerEvents: "none",
        }}>
          全景 &rsaquo; {
            activeCluster === "text" ? "哲学/文本" :
            activeCluster === "code" ? "代码" :
            activeCluster === "system" ? "谱系" : "其他"
          }
        </div>
      )}

      {/* 图例 */}
      <div style={{
        position: "absolute", top: 10, right: 10,
        background: "rgba(8,8,20,0.80)",
        border: "1px solid rgba(40,50,80,0.9)",
        borderRadius: 5, padding: "6px 10px",
        fontFamily: FONT.mono, fontSize: 8,
        color: "rgba(130,140,170,0.85)",
        display: "flex", flexDirection: "column", gap: 3,
        pointerEvents: "none",
        backdropFilter: "blur(6px)",
      }}>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <span style={{ color: "#00ffcc" }}>●</span> f=0
          <span style={{ color: "#22d68a" }}>●</span> fold
          <span style={{ color: "#f0c040" }}>●</span> gray
          <span style={{ color: "#ff4466" }}>●</span> negate
        </div>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <span style={{ color: "#4488ff" }}>◎</span> settled
          <span style={{ color: "#fff" }}>◉</span> 逢亮
          <span style={{ color: "#ff3355", fontSize: 7 }}>&#10239;</span> negation
        </div>
      </div>

      {/* 元信息 */}
      {data?.meta && (
        <div style={{
          position: "absolute", bottom: 10, right: 10,
          fontFamily: FONT.mono, fontSize: 8,
          color: "rgba(80,90,120,0.85)",
          pointerEvents: "none",
          letterSpacing: "0.06em",
        }}>
          深空观测站 V2 · {data.meta.shown_vertices}V / {data.meta.total_vertices.toLocaleString()} total
        </div>
      )}

      {/* hover tooltip */}
      {hoveredNode && (
        <div style={{
          position: "absolute", bottom: 10, left: 10,
          background: "rgba(8,8,20,0.92)",
          border: "1px solid rgba(68,136,255,0.35)",
          borderRadius: 6, padding: "8px 14px",
          fontFamily: FONT.mono, fontSize: 11,
          backdropFilter: "blur(12px)",
          pointerEvents: "none",
          display: "flex", flexDirection: "column", gap: 3,
          maxWidth: 280,
          boxShadow: "0 0 16px rgba(68,136,255,0.2)",
        }}>
          <span style={{
            color: fToColor(hoveredNode.f_avg),
            fontWeight: 600, fontSize: 12,
            textShadow: "0 0 10px currentColor",
          }}>
            {hoveredNode.label.length > 36
              ? hoveredNode.label.slice(0, 35) + "…"
              : hoveredNode.label}
          </span>
          <div style={{ display: "flex", gap: 10, color: "rgba(140,150,190,0.9)", fontSize: 9 }}>
            <span>f={hoveredNode.f_avg.toFixed(1)}</span>
            <span>deg={hoveredNode.degree}</span>
            <span style={{ color: "rgba(100,110,150,0.8)" }}>
              {hoveredNode.type === "text" ? "哲学" :
               hoveredNode.type === "code" ? "代码" : "系统"}
            </span>
            {hoveredNode.settled && (
              <span style={{ color: "#4488ff" }}>settled</span>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
