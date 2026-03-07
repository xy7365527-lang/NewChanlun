/**
 * GalaxyView.tsx — 宇宙星系图
 *
 * 设计语言：NASA 可观测宇宙 + Elite Dangerous 银河系地图 + 宇宙大尺度结构（cosmic web）
 *
 * 渲染层（15K 节点全量，单 draw call）：
 *   - 星星粒子：THREE.Points + InstancedBufferGeometry（大小/颜色作为 attribute）
 *   - 星云气泡：GLSL shader，体积感光晕（AdditiveBlending）
 *   - 暗物质丝：LineSegments，仅 degree >= 5 的边，极低不透明度
 *   - 星座线：settled 节点之间的蓝色连线，脉冲动画
 *   - 穿越粒子：逢亮位置 + 高亮拖尾
 *   - 背景星场：静态 2K 随机粒子（点缀宇宙感）
 *
 * 布局算法（不用力导向，O(N) 预计算）：
 *   1. 按 source type 分组（text/code/system/other）
 *   2. 每组分配一个球面扇区（黄金螺旋分布）
 *   3. 组内节点：高 degree 在中心，向外用 logistic 密度递减排列
 *   4. 星系之间距离固定（不依赖边密度，避免 O(E) 计算）
 *
 * 性能：
 *   - 节点：1 个 THREE.Points draw call（Float32Array attribute）
 *   - 边：1 个 THREE.LineSegments draw call（过滤 degree >= 5）
 *   - settled 线：1 个 THREE.LineSegments draw call
 *   - 全部静态几何体（不含逐帧重建），动画仅更新 uniform
 *
 * 增量更新（2024 修订）：
 *   - 节点 ID 集合不变时，只更新 color/size attribute buffer，不重建场景
 *   - traversalPosition 通过 traversalRef 实时注入动画循环，不触发场景重建
 *   - 相机状态（球坐标、lookAt）跨 poll 周期保持
 */

import { useEffect, useRef, useState, useCallback } from "react";
import * as THREE from "three";
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

// ── 星系常数 ─────────────────────────────────────────────────────

// 星系中心位置（三维空间，手动分离保证视觉区分）
const GALAXY_CENTERS: Record<string, Vec3> = {
  text:   { x: -380, y:  180, z: -120 },   // 哲学区：左上
  code:   { x:  380, y: -180, z:  100 },   // 代码区：右下
  system: { x:   40, y:   80, z: -280 },   // 谱系区：中后
  other:  { x:  -80, y: -220, z:  200 },   // 其他区：前下
};

// 每个星系的半径（决定节点散布范围）
const GALAXY_RADIUS: Record<string, number> = {
  text:   260,
  code:   240,
  system: 200,
  other:  160,
};

// 星云颜色（RGBA hex）
const NEBULA_COLOR: Record<string, number> = {
  text:   0xff7040,   // 暖橙——哲学星云
  code:   0x40a0ff,   // 冷蓝——代码星云
  system: 0x9060ff,   // 紫——谱系星云
  other:  0x40d090,   // 青绿——其他
};

// 标签文字
const NEBULA_LABEL: Record<string, string> = {
  text:   "哲学/文本",
  code:   "代码",
  system: "谱系",
  other:  "其他",
};

// ── 颜色 ──────────────────────────────────────────────────────────

// f 值热成像（青→绿→黄→红）
function fToRGB(f: number): [number, number, number] {
  if (f < 0)   return [0.12, 0.12, 0.22];   // 未知：暗紫
  if (f <= 0.5) return [0.0,  1.0,  0.8];    // f=0：青白
  if (f < 5)   return [0.13, 0.84, 0.54];   // fold zone：翠绿
  if (f < 12)  return [0.94, 0.75, 0.25];   // gray zone：琥珀
  return [1.0, 0.27, 0.40];                  // negate zone：朱红
}

// ── 布局：黄金螺旋球面分布 ────────────────────────────────────────

/**
 * 在球面上均匀分布 n 个点（黄金螺旋法）。
 * 返回单位球面上的坐标数组。
 */
function goldenSpherePoints(n: number): Vec3[] {
  const phi = Math.PI * (Math.sqrt(5) - 1);  // 黄金角
  const pts: Vec3[] = [];
  for (let i = 0; i < n; i++) {
    const y = 1 - (i / (n - 1)) * 2;
    const r = Math.sqrt(Math.max(0, 1 - y * y));
    const theta = phi * i;
    pts.push({ x: r * Math.cos(theta), y, z: r * Math.sin(theta) });
  }
  return pts;
}

/**
 * 计算星系布局（纯预计算，不用力导向）。
 *
 * 算法：
 *   1. 按 source type 分组
 *   2. 组内按 degree 降序排列（高 degree 在中心）
 *   3. 用黄金螺旋分布在球面上，半径用 logistic 函数从中心向外递增
 *      r = R * (1 - e^(-k*rank)) / (1 - e^(-k*N))
 *   4. 加上随机扰动，让星系看起来有机
 */
function computeGalaxyLayout(nodes: TopologyNode[]): Vec3[] {
  const groups: Record<string, number[]> = { text: [], code: [], system: [], other: [] };

  nodes.forEach((n, i) => {
    const g = groups[n.type] ?? groups.other;
    g.push(i);
  });

  const positions: Vec3[] = new Array(nodes.length);
  const rng = mulberry32(0x12345678);  // 确定性伪随机（布局稳定）

  for (const [gtype, indices] of Object.entries(groups)) {
    if (indices.length === 0) continue;

    const center = GALAXY_CENTERS[gtype] ?? GALAXY_CENTERS.other;
    const R = GALAXY_RADIUS[gtype] ?? 180;

    // 按 degree 降序排列
    indices.sort((a, b) => nodes[b].degree - nodes[a].degree);

    const N = indices.length;
    const spherePts = goldenSpherePoints(Math.max(N, 2));

    indices.forEach((nodeIdx, rank) => {
      // logistic 半径：中心密集，外围稀疏
      const k = 4.5;
      const normRank = rank / Math.max(N - 1, 1);
      const radiusFrac = (1 - Math.exp(-k * normRank)) / (1 - Math.exp(-k));
      const r = R * (0.05 + 0.95 * radiusFrac);

      // 黄金螺旋球面方向
      const sp = spherePts[rank % spherePts.length];

      // 轻微随机扰动（保证同一颗"星"不完全规则）
      const jitter = r * 0.12;
      positions[nodeIdx] = {
        x: center.x + sp.x * r + (rng() - 0.5) * jitter,
        y: center.y + sp.y * r + (rng() - 0.5) * jitter,
        z: center.z + sp.z * r + (rng() - 0.5) * jitter,
      };
    });
  }

  return positions;
}

// 确定性伪随机（Mulberry32）
function mulberry32(seed: number): () => number {
  let s = seed;
  return () => {
    s |= 0; s = s + 0x6D2B79F5 | 0;
    let t = Math.imul(s ^ s >>> 15, 1 | s);
    t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t;
    return ((t ^ t >>> 14) >>> 0) / 4294967296;
  };
}

// ── 节点大小（对数） ──────────────────────────────────────────────

function nodeSize(degree: number, isTraversal: boolean, isFocus: boolean): number {
  if (isTraversal) return 14;
  if (isFocus)     return 10;
  // 基础粒子：最小 2px（微尘），最大 9px（超巨星）
  return 2 + Math.log1p(Math.min(degree, 300)) * 1.4;
}

// ── 背景星场（静态，不更新） ──────────────────────────────────────

function buildBackgroundStars(count: number): THREE.Points {
  const rng = mulberry32(0xdeadbeef);
  const pos = new Float32Array(count * 3);
  const col = new Float32Array(count * 3);
  const BG_R = 3500;

  for (let i = 0; i < count; i++) {
    // 球面均匀分布
    const u = rng(), v = rng();
    const theta = 2 * Math.PI * u;
    const phi = Math.acos(2 * v - 1);
    const r = BG_R * (0.6 + 0.4 * rng());
    pos[i * 3]     = r * Math.sin(phi) * Math.cos(theta);
    pos[i * 3 + 1] = r * Math.sin(phi) * Math.sin(theta);
    pos[i * 3 + 2] = r * Math.cos(phi);

    // 颜色：蓝白冷星 / 白星 / 淡黄暖星
    const starType = rng();
    if (starType < 0.3) {
      // 蓝白星
      col[i * 3]     = 0.7 + rng() * 0.3;
      col[i * 3 + 1] = 0.8 + rng() * 0.2;
      col[i * 3 + 2] = 1.0;
    } else if (starType < 0.7) {
      // 白星
      const w = 0.85 + rng() * 0.15;
      col[i * 3] = col[i * 3 + 1] = col[i * 3 + 2] = w;
    } else {
      // 淡黄/橙星
      col[i * 3]     = 1.0;
      col[i * 3 + 1] = 0.85 + rng() * 0.1;
      col[i * 3 + 2] = 0.5 + rng() * 0.3;
    }
  }

  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.BufferAttribute(pos, 3));
  geo.setAttribute("color", new THREE.BufferAttribute(col, 3));

  const mat = new THREE.PointsMaterial({
    size: 1.2,
    vertexColors: true,
    transparent: true,
    opacity: 0.55,
    depthWrite: false,
    sizeAttenuation: true,
    blending: THREE.AdditiveBlending,
  });

  return new THREE.Points(geo, mat);
}

// ── 星云 GLSL（体积感球状星云） ───────────────────────────────────

const nebulaVertGLSL = `
varying vec3 vWorldPos;
varying vec3 vCenter;
uniform vec3 uCenter;
void main() {
  vec4 worldPos = modelMatrix * vec4(position, 1.0);
  vWorldPos = worldPos.xyz;
  vCenter = uCenter;
  gl_Position = projectionMatrix * viewMatrix * worldPos;
}
`;

const nebulaFragGLSL = `
uniform vec3 uColor;
uniform float uRadius;
uniform float uTime;
uniform float uOpacity;
varying vec3 vWorldPos;
varying vec3 vCenter;

float hash(vec3 p) {
  p = fract(p * vec3(0.1031, 0.1030, 0.0973));
  p += dot(p, p.yxz + 33.33);
  return fract((p.x + p.y) * p.z);
}

void main() {
  float dist = length(vWorldPos - vCenter);
  float norm = dist / uRadius;

  // 柔和边缘衰减（cubic falloff）
  float edge = 1.0 - smoothstep(0.55, 1.0, norm);
  float core = smoothstep(0.0, 0.35, norm) * (1.0 - smoothstep(0.35, 0.65, norm));

  // 轻微噪声纹理（让星云不均匀）
  float noise = hash(vWorldPos * 0.008 + uTime * 0.01);
  float nebulaBrightness = (edge * 0.6 + core * 0.25) * (0.75 + noise * 0.5);

  float alpha = nebulaBrightness * uOpacity;
  gl_FragColor = vec4(uColor, alpha);
}
`;

// ── 合成边生成（3D 用） ───────────────────────────────────────────
// 当 backend 返回 links=[] 时，按 cluster 生成轻量视觉边，保证
// negation 边脉冲粒子和暗物质丝有内容可渲染。

interface SyntheticLink3D {
  srcIdx: number;
  tgtIdx: number;
  type: "negation" | "inclusion";
}

function generateSyntheticLinks3D(
  nodes: TopologyNode[],
  maxEdges: number,
): SyntheticLink3D[] {
  if (nodes.length === 0) return [];

  // 按 type 分组，索引到原始 nodes 数组
  const byType: Record<string, number[]> = {};
  nodes.forEach((n, i) => {
    const k = n.type || "other";
    if (!byType[k]) byType[k] = [];
    byType[k].push(i);
  });

  const edges: SyntheticLink3D[] = [];
  const edgesPerCluster = Math.ceil(maxEdges / Math.max(Object.keys(byType).length, 1));

  for (const indices of Object.values(byType)) {
    // 按 degree 降序排列
    const sorted = [...indices].sort((a, b) => nodes[b].degree - nodes[a].degree);

    // Spine
    const spineLen = Math.min(sorted.length - 1, Math.ceil(edgesPerCluster * 0.6));
    for (let i = 0; i < spineLen; i++) {
      edges.push({
        srcIdx: sorted[i],
        tgtIdx: sorted[i + 1],
        type: nodes[sorted[i]].f_avg > 10 ? "negation" : "inclusion",
      });
    }

    // Leaf edges
    const leafLen = Math.min(
      indices.length - spineLen - 1,
      edgesPerCluster - spineLen,
    );
    for (let i = spineLen + 1; i < spineLen + 1 + leafLen; i++) {
      if (i >= sorted.length) break;
      const spineIdx = Math.floor(Math.random() * Math.max(spineLen, 1));
      edges.push({
        srcIdx: sorted[spineIdx],
        tgtIdx: sorted[i],
        type: nodes[sorted[i]].f_avg > 10 ? "negation" : "inclusion",
      });
    }

    if (edges.length >= maxEdges) break;
  }

  return edges;
}

// ── 相机缓动 ──────────────────────────────────────────────────────

function easeInOut(t: number): number {
  return t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t;
}

// ── Hex color helper ──────────────────────────────────────────────

function hexToRgbString(hex: number): string {
  const r = (hex >> 16) & 0xff;
  const g = (hex >> 8)  & 0xff;
  const b =  hex        & 0xff;
  return `${r},${g},${b}`;
}

// ── 场景句柄（用于增量更新） ──────────────────────────────────────

interface SceneHandles {
  nodes: TopologyNode[];
  nodeIdxMap: Map<string, number>;
  colAttr: THREE.BufferAttribute;
  sizeAttr: THREE.BufferAttribute;
}

// ── 主组件 ────────────────────────────────────────────────────────

export function GalaxyView({
  data, traversalPosition, focusConcept, onSelectNode,
}: Props) {
  const mountRef    = useRef<HTMLDivElement>(null);
  const labelRootRef = useRef<HTMLDivElement>(null);
  const [hoveredNode, setHoveredNode] = useState<TopologyNode | null>(null);
  const [loadingFull, setLoadingFull] = useState(false);
  const [nodeCount, setNodeCount] = useState(0);

  // traversalPosition 通过 ref 传入动画循环，避免重建场景
  const traversalRef = useRef(traversalPosition);
  traversalRef.current = traversalPosition;

  // focusConcept 通过 ref 传入，避免重建场景
  const focusConceptRef = useRef(focusConcept);
  focusConceptRef.current = focusConcept;

  // 场景句柄 ref，用于增量属性更新
  const sceneHandlesRef = useRef<SceneHandles | null>(null);

  // 上一次建场景时的节点 ID 集合
  const prevNodeIdsRef = useRef<Set<string>>(new Set());

  // 全量数据增量加载
  const [fullData, setFullData] = useState<TopologyResponse | null>(null);
  const activeData = fullData ?? data;

  const fetchFull = useCallback(async () => {
    if (loadingFull || fullData) return;
    setLoadingFull(true);
    try {
      const res = await fetch("http://localhost:9090/topology?full=true");
      if (res.ok) {
        const json: TopologyResponse = await res.json();
        setFullData(json);
        setNodeCount(json.nodes.length);
      }
    } catch {
      // 降级：使用骨架数据
    } finally {
      setLoadingFull(false);
    }
  }, [loadingFull, fullData]);

  // 组件挂载后后台加载全量数据
  useEffect(() => {
    if (data && !fullData) {
      setNodeCount(data.nodes.length);
      const timer = setTimeout(() => fetchFull(), 800);
      return () => clearTimeout(timer);
    }
  }, [data, fullData, fetchFull]);

  // ---------------------------------------------------------------------------
  // 增量属性更新：节点集合不变时，只更新 color/size attribute buffer。
  // 不重建场景，不重置相机。
  // ---------------------------------------------------------------------------
  const applyAttributeUpdate = useCallback((incoming: TopologyResponse) => {
    const handles = sceneHandlesRef.current;
    if (!handles) return;

    const { nodes, nodeIdxMap, colAttr, sizeAttr } = handles;
    const incomingMap = new Map(incoming.nodes.map((n) => [n.id, n]));
    const currentTraversal = traversalRef.current;
    const currentFocus = focusConceptRef.current;

    for (const node of nodes) {
      const src = incomingMap.get(node.id);
      if (!src) continue;

      // Mutate the node object in-place (shared with animation loop)
      node.degree  = src.degree;
      node.settled = src.settled;
      node.f_avg   = src.f_avg;

      const idx = nodeIdxMap.get(node.id);
      if (idx === undefined) continue;

      const isTraversal = node.id === currentTraversal;
      const isFocus     = node.label === currentFocus;

      // Update color buffer
      if (isTraversal) {
        colAttr.setXYZ(idx, 1.0, 1.0, 1.0);
      } else {
        const [r, g, b] = fToRGB(src.f_avg);
        colAttr.setXYZ(idx, r, g, b);
      }

      // Update size buffer
      sizeAttr.setX(idx, nodeSize(src.degree, isTraversal, isFocus));
    }

    colAttr.needsUpdate  = true;
    sizeAttr.needsUpdate = true;
  }, []);

  // ---------------------------------------------------------------------------
  // 主场景 effect — 只在节点 ID 集合变化时重建场景。
  // traversalPosition / focusConcept 的变化通过 ref 注入，不触发重建。
  // ---------------------------------------------------------------------------
  useEffect(() => {
    const mount    = mountRef.current;
    const labelRoot = labelRootRef.current;
    if (!mount || !labelRoot || !activeData || activeData.nodes.length === 0) return;

    // ── 节点集合稳定性检测 ──────────────────────────────────────
    const newIds = new Set(activeData.nodes.map((n) => n.id));
    const prevIds = prevNodeIdsRef.current;
    const sameNodes =
      newIds.size === prevIds.size &&
      [...newIds].every((id) => prevIds.has(id));

    if (sameNodes && sceneHandlesRef.current) {
      // 节点集合相同 — 只更新属性，不重建场景
      applyAttributeUpdate(activeData);
      return;
    }

    // 节点集合变化 — 完整重建
    prevNodeIdsRef.current = newIds;

    const W = mount.clientWidth  || 900;
    const H = mount.clientHeight || 600;
    if (W === 0 || H === 0) return;

    const nodes = activeData.nodes;
    const rawLinks = activeData.links;
    const N     = nodes.length;

    // ── Scene ──────────────────────────────────────────────────────
    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x010108);
    // 远雾——让边缘星系自然消散
    scene.fog = new THREE.FogExp2(0x010108, 0.00045);

    // ── Camera ─────────────────────────────────────────────────────
    const camera = new THREE.PerspectiveCamera(60, W / H, 1, 12000);
    camera.position.set(0, 80, 920);

    // ── Renderer ───────────────────────────────────────────────────
    const renderer = new THREE.WebGLRenderer({
      antialias: true,
      alpha: false,
      powerPreference: "high-performance",
    });
    renderer.setSize(W, H);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 0.85;
    mount.appendChild(renderer.domElement);

    // ── CSS2D（label overlay） ─────────────────────────────────────
    const css2d = new CSS2DRenderer();
    css2d.setSize(W, H);
    css2d.domElement.style.position = "absolute";
    css2d.domElement.style.top      = "0";
    css2d.domElement.style.left     = "0";
    css2d.domElement.style.pointerEvents = "none";
    labelRoot.appendChild(css2d.domElement);

    // ── 背景星场 ───────────────────────────────────────────────────
    const bgStars = buildBackgroundStars(3000);
    scene.add(bgStars);

    // ── 预计算布局 ─────────────────────────────────────────────────
    const positions = computeGalaxyLayout(nodes);

    // 节点 → 索引映射
    const nodeIdxMap = new Map<string, number>(nodes.map((n, i) => [n.id, i]));

    // ── 星星粒子（主 draw call）────────────────────────────────────

    // attribute buffers
    const posArr    = new Float32Array(N * 3);
    const colArr    = new Float32Array(N * 3);
    const sizeArr   = new Float32Array(N);
    const phaseArr  = new Float32Array(N);     // 闪烁相位
    const rng2      = mulberry32(0xabcdef01);

    nodes.forEach((node, i) => {
      const p = positions[i];
      posArr[i * 3]     = p.x;
      posArr[i * 3 + 1] = p.y;
      posArr[i * 3 + 2] = p.z;

      const [r, g, b] = fToRGB(node.f_avg);
      colArr[i * 3]     = r;
      colArr[i * 3 + 1] = g;
      colArr[i * 3 + 2] = b;

      // Use current traversalRef.current so the size is correct from the start
      const isTraversal = node.id === traversalRef.current;
      const isFocus     = node.label === focusConceptRef.current;
      sizeArr[i]  = nodeSize(node.degree, isTraversal, isFocus);
      phaseArr[i] = rng2() * Math.PI * 2;
    });

    // 自定义 shader——支持每顶点大小和颜色
    const starVertGLSL = `
      attribute float aSize;
      attribute float aPhase;
      attribute float aHighlight;
      uniform float uTime;
      uniform float uPixelRatio;
      varying vec3 vColor;
      varying float vHighlight;

      void main() {
        vColor = color;
        vHighlight = aHighlight;

        // 闪烁：轻微正弦调制大小
        float twinkle = 1.0 + 0.08 * sin(uTime * 2.1 + aPhase)
                            + 0.04 * sin(uTime * 3.7 + aPhase * 1.3);
        float sz = aSize * twinkle * (1.0 + aHighlight * 1.5);

        vec4 mvPosition = modelViewMatrix * vec4(position, 1.0);
        gl_PointSize = sz * uPixelRatio * (600.0 / -mvPosition.z);
        gl_PointSize = clamp(gl_PointSize, 1.0, 40.0);
        gl_Position = projectionMatrix * mvPosition;
      }
    `;

    const starFragGLSL = `
      varying vec3 vColor;
      varying float vHighlight;

      void main() {
        // 圆形点 + 柔边（星星形状）
        vec2 uv = gl_PointCoord * 2.0 - 1.0;
        float d = length(uv);

        // 核心光晕
        float core = 1.0 - smoothstep(0.0, 0.45, d);
        float halo = (1.0 - smoothstep(0.35, 1.0, d)) * 0.35;
        float alpha = core + halo;

        // 高亮星星加十字衍射（模拟真实星光）
        if (vHighlight > 0.5) {
          float cross1 = (1.0 - smoothstep(0.0, 0.08, abs(uv.x))) * (1.0 - smoothstep(0.2, 1.0, abs(uv.y)));
          float cross2 = (1.0 - smoothstep(0.0, 0.08, abs(uv.y))) * (1.0 - smoothstep(0.2, 1.0, abs(uv.x)));
          alpha += (cross1 + cross2) * 0.5;
        }

        if (alpha < 0.01) discard;
        gl_FragColor = vec4(vColor * (1.0 + vHighlight * 0.8), alpha);
      }
    `;

    const starGeo = new THREE.BufferGeometry();
    starGeo.setAttribute("position",  new THREE.BufferAttribute(posArr.slice(), 3));

    const colAttr = new THREE.BufferAttribute(colArr.slice(), 3);
    colAttr.setUsage(THREE.DynamicDrawUsage);
    starGeo.setAttribute("color", colAttr);

    const sizeAttr = new THREE.BufferAttribute(sizeArr.slice(), 1);
    sizeAttr.setUsage(THREE.DynamicDrawUsage);
    starGeo.setAttribute("aSize", sizeAttr);

    starGeo.setAttribute("aPhase",    new THREE.BufferAttribute(phaseArr, 1));

    // highlight attribute（hover/focus 用）— 初始全 0
    const highlightArr = new Float32Array(N).fill(0);
    const highlightAttr = new THREE.BufferAttribute(highlightArr, 1);
    highlightAttr.setUsage(THREE.DynamicDrawUsage);
    starGeo.setAttribute("aHighlight", highlightAttr);

    // traversal node: override color to bright white and size already set above
    // additionally write white color into colArr for the traversal node
    {
      const trvIdx = nodes.findIndex((n) => n.id === traversalRef.current);
      if (trvIdx >= 0) {
        colAttr.setXYZ(trvIdx, 1.0, 1.0, 1.0);
        colAttr.needsUpdate = true;
      }
    }

    const starMat = new THREE.ShaderMaterial({
      vertexShader:   starVertGLSL,
      fragmentShader: starFragGLSL,
      uniforms: {
        uTime:       { value: 0 },
        uPixelRatio: { value: renderer.getPixelRatio() },
      },
      vertexColors:  true,
      transparent:   true,
      depthWrite:    false,
      blending:      THREE.AdditiveBlending,
    });

    const starPoints = new THREE.Points(starGeo, starMat);
    scene.add(starPoints);

    // 保存句柄供增量更新使用
    sceneHandlesRef.current = { nodes, nodeIdxMap, colAttr, sizeAttr };

    // ── 星云气泡（per-galaxy volumetric glow） ────────────────────

    const nebulaMeshes: Array<{ mesh: THREE.Mesh; gtype: string }> = [];
    const nebulaUniforms: THREE.ShaderMaterial["uniforms"][] = [];

    const galaxyTypes = ["text", "code", "system", "other"] as const;
    galaxyTypes.forEach((gtype) => {
      const center = GALAXY_CENTERS[gtype];
      const R      = GALAXY_RADIUS[gtype];
      const col3   = new THREE.Color(NEBULA_COLOR[gtype]);

      // 体积球——用双面渲染叠加，造成厚度感
      [1.0, 0.65, 0.38].forEach((scale, layer) => {
        const geo = new THREE.SphereGeometry(R * scale, 24, 18);
        const uniforms = {
          uColor:   { value: col3 },
          uRadius:  { value: R * scale },
          uCenter:  { value: new THREE.Vector3(center.x, center.y, center.z) },
          uTime:    { value: 0 },
          uOpacity: { value: layer === 0 ? 0.018 : layer === 1 ? 0.025 : 0.035 },
        };
        nebulaUniforms.push(uniforms);
        const mat = new THREE.ShaderMaterial({
          vertexShader:   nebulaVertGLSL,
          fragmentShader: nebulaFragGLSL,
          uniforms,
          transparent: true,
          depthWrite:  false,
          side:        THREE.DoubleSide,
          blending:    THREE.AdditiveBlending,
        });
        const mesh = new THREE.Mesh(geo, mat);
        mesh.position.set(center.x, center.y, center.z);
        scene.add(mesh);
        nebulaMeshes.push({ mesh, gtype });
      });

      // 星系标签
      const div = document.createElement("div");
      div.style.cssText = `
        font-family: ${FONT.mono};
        font-size: 9px;
        letter-spacing: 0.15em;
        text-transform: uppercase;
        color: rgba(${hexToRgbString(NEBULA_COLOR[gtype])}, 0.45);
        pointer-events: none;
        user-select: none;
        white-space: nowrap;
      `;
      div.textContent = NEBULA_LABEL[gtype];
      const lbl = new CSS2DObject(div);
      lbl.position.set(center.x, center.y + R * 0.9, center.z);
      scene.add(lbl);
    });

    // ── 解析/合成边（统一入口） ───────────────────────────────────
    // 将 TopologyLink[] 和合成边都归一化为 {srcIdx, tgtIdx, type} 格式

    interface ResolvedLink {
      srcIdx: number;
      tgtIdx: number;
      type: string;
    }

    let resolvedLinks: ResolvedLink[] = [];

    if (rawLinks.length > 0) {
      for (const l of rawLinks) {
        const srcId = typeof l.source === "string" ? l.source : (l.source as TopologyNode).id;
        const tgtId = typeof l.target === "string" ? l.target : (l.target as TopologyNode).id;
        const si = nodeIdxMap.get(srcId);
        const ti = nodeIdxMap.get(tgtId);
        if (si === undefined || ti === undefined) continue;
        resolvedLinks.push({ srcIdx: si, tgtIdx: ti, type: l.type });
      }
    } else {
      // Backend returned no edges — generate synthetic edges so negation
      // pulse particles and dark-matter filaments have data to render.
      const synth = generateSyntheticLinks3D(nodes, Math.min(N * 2, 6000));
      resolvedLinks = synth.map((s) => ({ srcIdx: s.srcIdx, tgtIdx: s.tgtIdx, type: s.type }));
    }

    // ── 暗物质丝（高 degree 节点之间的边） ───────────────────────

    const MIN_EDGE_DEGREE = 4;  // 只渲染至少一端 degree >= 4 的边
    const negPts: number[] = [];
    const depPts: number[] = [];

    for (const { srcIdx, tgtIdx, type } of resolvedLinks) {
      const maxDeg = Math.max(nodes[srcIdx].degree, nodes[tgtIdx].degree);
      if (maxDeg < MIN_EDGE_DEGREE) continue;

      const ps = positions[srcIdx], pt = positions[tgtIdx];
      const arr = type === "negation" ? negPts : depPts;
      arr.push(ps.x, ps.y, ps.z, pt.x, pt.y, pt.z);
    }

    function makeLineSegs(pts: number[], color: THREE.Color, opacity: number): THREE.LineSegments | null {
      if (pts.length === 0) return null;
      const geo = new THREE.BufferGeometry();
      geo.setAttribute("position", new THREE.Float32BufferAttribute(pts, 3));
      const mat = new THREE.LineBasicMaterial({ color, transparent: true, opacity, depthWrite: false });
      return new THREE.LineSegments(geo, mat);
    }

    const negLines = makeLineSegs(negPts, new THREE.Color(0xff2244), 0.12);
    const depLines = makeLineSegs(depPts, new THREE.Color(0x1a2535), 0.09);
    if (negLines) scene.add(negLines);
    if (depLines) scene.add(depLines);

    // ── settled 星座线（蓝色连线，脉冲动画） ─────────────────────

    const settledIds   = new Set(nodes.filter((n) => n.settled).map((n) => n.id));
    const settledPts: number[] = [];

    for (const { srcIdx, tgtIdx } of resolvedLinks) {
      if (!settledIds.has(nodes[srcIdx].id) || !settledIds.has(nodes[tgtIdx].id)) continue;
      const ps = positions[srcIdx], pt = positions[tgtIdx];
      settledPts.push(ps.x, ps.y, ps.z, pt.x, pt.y, pt.z);
    }

    let settledLines: THREE.LineSegments | null = null;
    if (settledPts.length > 0) {
      const geo = new THREE.BufferGeometry();
      geo.setAttribute("position", new THREE.Float32BufferAttribute(settledPts, 3));
      const mat = new THREE.LineBasicMaterial({
        color: new THREE.Color(0x4488ff),
        transparent: true, opacity: 0.0,  // 动画中更新
        depthWrite: false,
      });
      settledLines = new THREE.LineSegments(geo, mat);
      scene.add(settledLines);
    }

    // ── negation 边脉冲粒子 ───────────────────────────────────────

    interface PulseParticle {
      pt:     THREE.Points;
      si:     number;
      ti:     number;
      phase:  number;
    }
    const pulses: PulseParticle[] = [];

    // negation 边（真实或合成）脉冲粒子，上限 80 个
    const negResolvedLinks = resolvedLinks.filter((l) => l.type === "negation");
    negResolvedLinks.slice(0, 80).forEach(({ srcIdx, tgtIdx }, li) => {
      const geo = new THREE.BufferGeometry();
      geo.setAttribute("position", new THREE.Float32BufferAttribute([0, 0, 0], 3));
      const mat = new THREE.PointsMaterial({
        color: new THREE.Color(0xff2244), size: 4,
        transparent: true, opacity: 0.9,
        depthWrite: false, sizeAttenuation: true,
        blending: THREE.AdditiveBlending,
      });
      const pt = new THREE.Points(geo, mat);
      scene.add(pt);
      pulses.push({ pt, si: srcIdx, ti: tgtIdx, phase: li * 0.41 });
    });

    // ── 穿越拖尾 ──────────────────────────────────────────────────

    const TRAIL_LEN = 14;
    const trailHistory: THREE.Vector3[] = [];
    const trailGeo    = new THREE.BufferGeometry();
    const trailPosBuf = new Float32Array(TRAIL_LEN * 3);
    const trailColBuf = new Float32Array(TRAIL_LEN * 3);
    trailGeo.setAttribute("position", new THREE.BufferAttribute(trailPosBuf, 3));
    trailGeo.setAttribute("color",    new THREE.BufferAttribute(trailColBuf, 3));
    const trailMat = new THREE.PointsMaterial({
      size: 6, vertexColors: true, transparent: true, opacity: 0,
      depthWrite: false, sizeAttenuation: true, blending: THREE.AdditiveBlending,
    });
    scene.add(new THREE.Points(trailGeo, trailMat));

    // 穿越节点光源
    const traversalLight = new THREE.PointLight(0xaaddff, 0, 200);
    scene.add(traversalLight);

    // 穿越节点：额外白色光晕 sprite（始终可见，不依赖边）
    const traversalSprite = (() => {
      const canvas2d = document.createElement("canvas");
      canvas2d.width = canvas2d.height = 64;
      const ctx = canvas2d.getContext("2d")!;
      const grad = ctx.createRadialGradient(32, 32, 2, 32, 32, 30);
      grad.addColorStop(0, "rgba(255,255,255,1.0)");
      grad.addColorStop(0.25, "rgba(200,220,255,0.7)");
      grad.addColorStop(1, "rgba(100,150,255,0.0)");
      ctx.fillStyle = grad;
      ctx.fillRect(0, 0, 64, 64);
      const tex = new THREE.CanvasTexture(canvas2d);
      const mat = new THREE.SpriteMaterial({
        map: tex,
        transparent: true,
        depthWrite: false,
        blending: THREE.AdditiveBlending,
      });
      const sprite = new THREE.Sprite(mat);
      sprite.scale.set(28, 28, 1);
      sprite.visible = false;
      return sprite;
    })();
    scene.add(traversalSprite);

    // ── 环境光 ────────────────────────────────────────────────────

    scene.add(new THREE.AmbientLight(0x080818, 0.5));
    // 各星系区域的微弱环境光
    const envLights = [
      { col: 0xff7040, pos: GALAXY_CENTERS.text   },
      { col: 0x4090ff, pos: GALAXY_CENTERS.code   },
      { col: 0x9060ff, pos: GALAXY_CENTERS.system },
    ];
    envLights.forEach(({ col, pos }) => {
      const pl = new THREE.PointLight(col, 0.4, 600);
      pl.position.set(pos.x, pos.y, pos.z);
      scene.add(pl);
    });

    // ── 鼠标交互 ──────────────────────────────────────────────────

    let isDragging  = false;
    let mouseMoved  = false;
    let prevMouse   = { x: 0, y: 0 };
    const spherical = new THREE.Spherical(camera.position.length(), Math.PI / 2.2, 0.0);
    let zoomVelocity = 0;

    // 相机飞行
    let flyFrom: THREE.Vector3 | null = null;
    let flyTo:   THREE.Vector3 | null = null;
    let flyLookFrom: THREE.Vector3 | null = null;
    let flyLookTo:   THREE.Vector3 | null = null;
    let flyT = 1;
    const flyLookTarget = new THREE.Vector3(0, 0, 0);  // 当前 lookAt（跟随飞行平滑变化）

    function startFly(targetPos: THREE.Vector3, lookAt: THREE.Vector3 = new THREE.Vector3(0, 0, 0)) {
      flyFrom = camera.position.clone();
      flyTo   = targetPos.clone();
      flyLookFrom = flyLookTarget.clone();
      flyLookTo   = lookAt.clone();
      flyT = 0;
    }

    // hover 状态
    let hoveredIdx  = -1;
    const neighborSet = new Set<number>();

    function computeNeighbors(idx: number): Set<number> {
      const s = new Set<number>();
      for (const { srcIdx, tgtIdx } of resolvedLinks) {
        if (srcIdx === idx) s.add(tgtIdx);
        if (tgtIdx === idx) s.add(srcIdx);
      }
      return s;
    }

    // label 对象（懒创建）
    const labelCache = new Map<number, CSS2DObject>();

    function getOrCreateLabel(idx: number): CSS2DObject {
      if (labelCache.has(idx)) return labelCache.get(idx)!;
      const node = nodes[idx];
      const div  = document.createElement("div");
      div.style.cssText = `
        font-family: ${FONT.mono};
        font-size: 9.5px;
        color: ${fToColor(node.f_avg)};
        background: rgba(1,1,8,0.88);
        border: 1px solid rgba(68,136,255,0.22);
        border-radius: 3px;
        padding: 2px 7px;
        white-space: nowrap;
        pointer-events: none;
        opacity: 0;
        transition: opacity 0.15s;
        user-select: none;
        backdrop-filter: blur(4px);
      `;
      div.textContent = node.label.length > 32 ? node.label.slice(0, 31) + "…" : node.label;
      const obj = new CSS2DObject(div);
      obj.position.set(
        positions[idx].x,
        positions[idx].y + sizeArr[idx] * 1.8,
        positions[idx].z
      );
      labelCache.set(idx, obj);
      return obj;
    }

    // Raycaster（对 Points 需要设置 threshold）
    const raycaster  = new THREE.Raycaster();
    raycaster.params.Points = { threshold: 12 };
    const mouseNDC   = new THREE.Vector2();

    const onMouseDown = (e: MouseEvent) => {
      isDragging = true;
      mouseMoved = false;
      prevMouse  = { x: e.clientX, y: e.clientY };
    };

    const onMouseMove = (e: MouseEvent) => {
      if (isDragging) {
        const dx = e.clientX - prevMouse.x;
        const dy = e.clientY - prevMouse.y;
        if (Math.abs(dx) + Math.abs(dy) > 2) mouseMoved = true;
        spherical.theta -= dx * 0.006;
        spherical.phi = Math.max(0.05, Math.min(Math.PI - 0.05, spherical.phi + dy * 0.006));
        prevMouse = { x: e.clientX, y: e.clientY };
        camera.position.setFromSpherical(spherical);
        camera.lookAt(flyLookTarget);
      }

      // hover 检测
      const rect = renderer.domElement.getBoundingClientRect();
      mouseNDC.x =  ((e.clientX - rect.left) / rect.width)  * 2 - 1;
      mouseNDC.y = -((e.clientY - rect.top)  / rect.height) * 2 + 1;
      raycaster.setFromCamera(mouseNDC, camera);
      const hits = raycaster.intersectObject(starPoints);

      if (hits.length > 0) {
        const idx = hits[0].index!;
        if (idx !== hoveredIdx) {
          // 取消上一个 hover
          if (hoveredIdx >= 0) {
            highlightArr[hoveredIdx] = 0;
            const lbl = labelCache.get(hoveredIdx);
            if (lbl) {
              lbl.element.style.opacity = "0";
              const ci = hoveredIdx;
              setTimeout(() => { if (labelCache.get(ci)?.parent) scene.remove(labelCache.get(ci)!); }, 160);
            }
            neighborSet.forEach((ni) => { highlightArr[ni] = 0; });
            neighborSet.clear();
          }

          hoveredIdx = idx;
          highlightArr[idx] = 1;
          computeNeighbors(idx).forEach((ni) => {
            neighborSet.add(ni);
            highlightArr[ni] = 0.4;
          });
          highlightAttr.needsUpdate = true;

          setHoveredNode(nodes[idx]);

          const lbl = getOrCreateLabel(idx);
          scene.add(lbl);
          setTimeout(() => { lbl.element.style.opacity = "1"; }, 10);
        }
      } else if (hoveredIdx >= 0) {
        // 清除 hover
        highlightArr[hoveredIdx] = 0;
        const lbl = labelCache.get(hoveredIdx);
        if (lbl) {
          lbl.element.style.opacity = "0";
          const ci = hoveredIdx;
          setTimeout(() => { if (labelCache.get(ci)?.parent) scene.remove(labelCache.get(ci)!); }, 160);
        }
        neighborSet.forEach((ni) => { highlightArr[ni] = 0; });
        neighborSet.clear();
        highlightAttr.needsUpdate = true;
        hoveredIdx = -1;
        setHoveredNode(null);
      }
    };

    const onMouseUp  = () => { isDragging = false; };
    const onWheel    = (e: WheelEvent) => { zoomVelocity += e.deltaY * 0.4; };
    const onDblClick = () => { startFly(new THREE.Vector3(0, 80, 920)); };

    const onCanvasClick = (e: MouseEvent) => {
      if (mouseMoved) return;
      const rect = renderer.domElement.getBoundingClientRect();
      mouseNDC.x =  ((e.clientX - rect.left) / rect.width)  * 2 - 1;
      mouseNDC.y = -((e.clientY - rect.top)  / rect.height) * 2 + 1;
      raycaster.setFromCamera(mouseNDC, camera);
      const hits = raycaster.intersectObject(starPoints);
      if (hits.length > 0) {
        const idx  = hits[0].index!;
        const node = nodes[idx];
        if (node && onSelectNode) {
          onSelectNode(node);
          const p = positions[idx];
          const target = new THREE.Vector3(p.x, p.y, p.z);
          const offset = camera.position.clone().sub(target).normalize().multiplyScalar(90);
          startFly(target.clone().add(offset), target);
        }
      }
    };

    renderer.domElement.addEventListener("mousedown",  onMouseDown);
    renderer.domElement.addEventListener("mousemove",  onMouseMove);
    renderer.domElement.addEventListener("mouseup",    onMouseUp);
    renderer.domElement.addEventListener("wheel",      onWheel,      { passive: true });
    renderer.domElement.addEventListener("click",      onCanvasClick);
    renderer.domElement.addEventListener("dblclick",   onDblClick);

    // ── 动画循环 ──────────────────────────────────────────────────

    let animId: number;
    let tick = 0;

    const animate = () => {
      animId = requestAnimationFrame(animate);
      tick++;
      const t = tick * 0.016;

      // 惯性缩放
      if (Math.abs(zoomVelocity) > 0.5) {
        spherical.radius = Math.max(60, Math.min(5000, spherical.radius + zoomVelocity));
        camera.position.setFromSpherical(spherical);
        camera.lookAt(flyLookTarget);
        zoomVelocity *= 0.86;
      }

      // 相机飞行插值
      if (flyT < 1 && flyFrom && flyTo) {
        flyT = Math.min(1, flyT + 0.022);
        const et = easeInOut(flyT);
        camera.position.lerpVectors(flyFrom, flyTo, et);
        if (flyLookFrom && flyLookTo) {
          flyLookTarget.lerpVectors(flyLookFrom, flyLookTo, et);
        }
        camera.lookAt(flyLookTarget);
        spherical.setFromVector3(camera.position);
      }

      // 星星闪烁 uniform
      starMat.uniforms.uTime.value = t;

      // 星云呼吸（缓慢脉冲）
      nebulaUniforms.forEach((u, idx) => {
        u.uTime.value = t;
        // 各层有不同呼吸节奏
        const baseOp = idx % 3 === 0 ? 0.018 : idx % 3 === 1 ? 0.025 : 0.035;
        u.uOpacity.value = baseOp * (0.85 + 0.15 * Math.sin(t * 0.35 + idx * 0.7));
      });

      // settled 星座线脉冲
      if (settledLines) {
        const mat = settledLines.material as THREE.LineBasicMaterial;
        mat.opacity = 0.08 + 0.06 * Math.sin(t * 0.8);
      }

      // negation 脉冲粒子（沿边线滑动的红色粒子）
      pulses.forEach(({ pt, si, ti, phase }) => {
        const frac = ((t * 0.55 + phase) % 1);
        const ps   = positions[si], pp = positions[ti];
        const posAttr = pt.geometry.attributes.position as THREE.BufferAttribute;
        posAttr.setXYZ(
          0,
          ps.x + (pp.x - ps.x) * frac,
          ps.y + (pp.y - ps.y) * frac,
          ps.z + (pp.z - ps.z) * frac,
        );
        posAttr.needsUpdate = true;
        const dFrac = Math.abs(frac - 0.5) * 2;
        (pt.material as THREE.PointsMaterial).opacity = 0.85 * (1 - dFrac * dFrac);
      });

      // 穿越拖尾 + 光源 + 白色光晕 sprite
      // 每帧从 traversalRef 读取当前值，避免需要重建场景
      const trvIdx = nodes.findIndex((n) => n.id === traversalRef.current);
      if (trvIdx >= 0) {
        const tp  = positions[trvIdx];
        const trvVec = new THREE.Vector3(tp.x, tp.y, tp.z);
        traversalLight.position.copy(trvVec);
        traversalLight.intensity = 2.0 + 0.8 * Math.sin(t * 4.5);

        // 穿越 sprite：位置跟随 + 脉冲缩放
        traversalSprite.position.copy(trvVec);
        const pulse = 1.0 + 0.35 * Math.sin(t * 4.5);
        traversalSprite.scale.set(28 * pulse, 28 * pulse, 1);
        traversalSprite.visible = true;

        if (trailHistory.length === 0 ||
            trailHistory[trailHistory.length - 1].distanceTo(trvVec) > 0.5) {
          trailHistory.push(trvVec.clone());
          if (trailHistory.length > TRAIL_LEN) trailHistory.shift();
        }

        const posAttr = trailGeo.attributes.position as THREE.BufferAttribute;
        const colAttr2 = trailGeo.attributes.color    as THREE.BufferAttribute;
        for (let k = 0; k < TRAIL_LEN; k++) {
          if (k < trailHistory.length) {
            const hp = trailHistory[trailHistory.length - 1 - k];
            posAttr.setXYZ(k, hp.x, hp.y, hp.z);
            const bright = 1 - k / TRAIL_LEN;
            colAttr2.setXYZ(k, bright * 0.6, bright * 0.9, 1.0);
          } else {
            posAttr.setXYZ(k, 0, 0, -99999);
            colAttr2.setXYZ(k, 0, 0, 0);
          }
        }
        posAttr.needsUpdate = true;
        colAttr2.needsUpdate = true;
        trailMat.opacity = 0.8;
      } else {
        traversalLight.intensity = 0;
        traversalSprite.visible = false;
        trailMat.opacity = 0;
      }

      renderer.render(scene, camera);
      css2d.render(scene, camera);
    };
    animate();

    // ── 窗口缩放 ──────────────────────────────────────────────────
    const onResize = () => {
      const w = mount.clientWidth  || W;
      const h = mount.clientHeight || H;
      if (w === 0 || h === 0) return;
      renderer.setSize(w, h);
      css2d.setSize(w, h);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    };
    window.addEventListener("resize", onResize);

    // ── 清理 ──────────────────────────────────────────────────────
    return () => {
      cancelAnimationFrame(animId);
      sceneHandlesRef.current = null;
      renderer.domElement.removeEventListener("mousedown",  onMouseDown);
      renderer.domElement.removeEventListener("mousemove",  onMouseMove);
      renderer.domElement.removeEventListener("mouseup",    onMouseUp);
      renderer.domElement.removeEventListener("wheel",      onWheel);
      renderer.domElement.removeEventListener("click",      onCanvasClick);
      renderer.domElement.removeEventListener("dblclick",   onDblClick);
      window.removeEventListener("resize", onResize);
      renderer.dispose();
      if (mount.contains(renderer.domElement)) mount.removeChild(renderer.domElement);
      if (labelRoot.contains(css2d.domElement)) labelRoot.removeChild(css2d.domElement);
    };
  // traversalPosition 和 focusConcept 通过 ref 注入，不加入依赖，不触发场景重建
  }, [activeData, onSelectNode, applyAttributeUpdate]);

  return (
    <div style={{ position: "relative", width: "100%", height: "100%" }}>
      {/* Three.js canvas */}
      <div
        ref={mountRef}
        style={{ width: "100%", height: "100%", position: "absolute", inset: 0 }}
      />

      {/* CSS2D label overlay */}
      <div
        ref={labelRootRef}
        style={{ position: "absolute", inset: 0, pointerEvents: "none", overflow: "hidden" }}
      />

      {/* 加载指示 */}
      {loadingFull && (
        <div style={{
          position: "absolute", top: 12, left: "50%",
          transform: "translateX(-50%)",
          fontFamily: FONT.mono, fontSize: 8,
          color: "rgba(124,108,255,0.6)",
          letterSpacing: "0.12em",
          pointerEvents: "none",
          animation: "pulse 1.4s ease-in-out infinite",
        }}>
          加载全量星系数据...
        </div>
      )}

      {/* 操作提示 */}
      <div style={{
        position: "absolute", bottom: 32, left: 12,
        fontFamily: FONT.mono, fontSize: 7.5,
        color: "rgba(60,60,90,0.7)",
        letterSpacing: "0.07em",
        pointerEvents: "none",
        lineHeight: 1.9,
      }}>
        拖拽旋转 · 滚轮缩放（惯性）· 点击星星选中 · 双击复位全景
      </div>

      {/* 图例 */}
      <div style={{
        position: "absolute", top: 10, right: 10,
        background: "rgba(1,1,8,0.72)",
        border: "1px solid rgba(20,20,40,0.9)",
        borderRadius: 6, padding: "7px 12px",
        fontFamily: FONT.mono, fontSize: 7.5,
        color: "rgba(80,80,110,0.8)",
        display: "flex", flexDirection: "column", gap: 4,
        pointerEvents: "none",
        backdropFilter: "blur(8px)",
      }}>
        <div style={{ color: "rgba(100,100,130,0.9)", letterSpacing: "0.08em", marginBottom: 2 }}>
          宇宙星系图
        </div>
        <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
          <span style={{ color: "#00ffcc" }}>★</span> f=0 均衡
          <span style={{ color: "#22d68a" }}>★</span> fold
          <span style={{ color: "#f0c040" }}>★</span> gray
          <span style={{ color: "#ff4466" }}>★</span> negate
        </div>
        <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
          <span style={{ color: "#4488ff" }}>─</span> settled星座
          <span style={{ color: "#ffffff" }}>◉</span> 逢亮
          <span style={{ color: "#ff2244" }}>·</span> 否定脉冲
        </div>
        <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
          <span style={{ color: "#ff7040", opacity: 0.7 }}>◎</span> 哲学
          <span style={{ color: "#4090ff", opacity: 0.7 }}>◎</span> 代码
          <span style={{ color: "#9060ff", opacity: 0.7 }}>◎</span> 谱系
        </div>
      </div>

      {/* 元信息 */}
      <div style={{
        position: "absolute", bottom: 10, right: 10,
        fontFamily: FONT.mono, fontSize: 7.5,
        color: "rgba(40,40,70,0.9)",
        pointerEvents: "none",
        letterSpacing: "0.06em",
      }}>
        可观测宇宙 · {nodeCount > 0 ? nodeCount.toLocaleString() : (activeData?.meta.shown_vertices ?? 0)} 颗星
        {activeData?.meta.total_vertices && activeData.meta.total_vertices > (activeData.meta.shown_vertices ?? 0)
          ? ` / ${activeData.meta.total_vertices.toLocaleString()} total`
          : ""
        }
      </div>

      {/* hover tooltip */}
      {hoveredNode && (
        <div style={{
          position: "absolute", bottom: 32, right: 10,
          background: "rgba(1,1,8,0.90)",
          border: "1px solid rgba(68,136,255,0.18)",
          borderRadius: 7, padding: "9px 15px",
          fontFamily: FONT.mono, fontSize: 11,
          backdropFilter: "blur(14px)",
          pointerEvents: "none",
          display: "flex", flexDirection: "column", gap: 4,
          maxWidth: 290,
        }}>
          <span style={{
            color: fToColor(hoveredNode.f_avg),
            fontWeight: 600, fontSize: 12,
            lineHeight: 1.3,
          }}>
            {hoveredNode.label.length > 38
              ? hoveredNode.label.slice(0, 37) + "…"
              : hoveredNode.label}
          </span>
          <div style={{ display: "flex", gap: 12, color: "rgba(80,80,110,0.9)", fontSize: 9 }}>
            <span>f = {hoveredNode.f_avg.toFixed(1)}</span>
            <span>度 = {hoveredNode.degree}</span>
            <span style={{ color: "rgba(50,50,80,0.9)" }}>
              {hoveredNode.type === "text"   ? "哲学" :
               hoveredNode.type === "code"   ? "代码" : "系统"}
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
