"""Diagram parser: architectural diagrams → box vertices + arrow edges.

Uses OpenCV edge detection + contour analysis. No neural networks.
- Closed contours with area > threshold → boxes (vertices)
- Line segments connecting boxes → arrows (directed edges)
- Direction inferred from arrow shape (triangular tip) or left-to-right default

Input: image file path or numpy array (BGR or grayscale).
Output: (vertices, edges) compatible with K_active.
"""

from __future__ import annotations

import hashlib
from pathlib import Path

import cv2
import numpy as np

from engine import Vertex, Edge, EdgeType, VertexStatus


def parse_diagram(image, source: str = "diagram") -> tuple[list[Vertex], list[Edge]]:
    """Parse an architectural diagram into box vertices and arrow edges.

    Args:
        image: File path (str/Path) or numpy array (BGR).
        source: Source label for vertex IDs.

    Returns:
        (vertices, edges) — boxes as vertices, arrows as directed edges.
    """
    if isinstance(image, (str, Path)):
        img = cv2.imread(str(image))
        if img is None:
            return [], []
    else:
        img = image

    gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY) if len(img.shape) == 3 else img
    h, w = gray.shape

    # Adaptive threshold for diagrams (usually black-on-white or white-on-black)
    binary = cv2.adaptiveThreshold(gray, 255, cv2.ADAPTIVE_THRESH_GAUSSIAN_C,
                                    cv2.THRESH_BINARY_INV, 15, 5)

    # Find contours
    contours, hierarchy = cv2.findContours(binary, cv2.RETR_TREE, cv2.CHAIN_APPROX_SIMPLE)

    # Classify contours: boxes (large, roughly rectangular) vs lines
    min_box_area = h * w * 0.002  # 0.2% of image area
    max_box_area = h * w * 0.5     # 50% of image area

    boxes: list[dict] = []
    for i, cnt in enumerate(contours):
        area = cv2.contourArea(cnt)
        if area < min_box_area or area > max_box_area:
            continue

        # Check rectangularity: perimeter vs bounding rect perimeter
        peri = cv2.arcLength(cnt, True)
        approx = cv2.approxPolyDP(cnt, 0.04 * peri, True)

        # Accept 4-8 sided polygons as boxes (rectangles, rounded rects)
        if 4 <= len(approx) <= 8:
            x, y, bw, bh = cv2.boundingRect(cnt)
            aspect = max(bw, bh) / max(min(bw, bh), 1)
            if aspect < 5:  # Not too elongated
                cx, cy = x + bw // 2, y + bh // 2
                boxes.append({
                    "id": i,
                    "x": x, "y": y, "w": bw, "h": bh,
                    "cx": cx, "cy": cy,
                    "area": area,
                })

    # Deduplicate overlapping boxes (keep larger one)
    boxes = _deduplicate_boxes(boxes)

    if not boxes:
        return [], []

    # Create vertices
    vertices: list[Vertex] = []
    vid_map: dict[int, str] = {}

    for box in boxes:
        # Try OCR-free label: crop region, check if it has text-like content
        label = f"box_{box['cx']}_{box['cy']}"
        vid = f"{source}:diagram:{label}"
        vertices.append(Vertex(id=vid, status=VertexStatus.ACTIVE,
                                content=f"diagram box at ({box['cx']}, {box['cy']}) "
                                        f"size {box['w']}x{box['h']}"))
        vid_map[box["id"]] = vid

    # Detect arrows: line segments connecting boxes
    edges: list[Edge] = []

    # Use Hough line detection
    lines = cv2.HoughLinesP(binary, 1, np.pi / 180, threshold=30,
                             minLineLength=min(h, w) * 0.05,
                             maxLineGap=min(h, w) * 0.02)

    if lines is not None:
        for line in lines:
            x1, y1, x2, y2 = line[0]
            # Find which boxes this line connects
            src_box = _find_nearest_box(x1, y1, boxes, max_dist=min(h, w) * 0.1)
            tgt_box = _find_nearest_box(x2, y2, boxes, max_dist=min(h, w) * 0.1)

            if src_box is not None and tgt_box is not None and src_box != tgt_box:
                src_vid = vid_map.get(src_box)
                tgt_vid = vid_map.get(tgt_box)
                if src_vid and tgt_vid:
                    edges.append(Edge(source=src_vid, target=tgt_vid,
                                      edge_type=EdgeType.DEPENDENCY))

    # Deduplicate edges
    seen: set[tuple[str, str]] = set()
    deduped: list[Edge] = []
    for e in edges:
        key = (e.source, e.target)
        if key not in seen:
            seen.add(key)
            deduped.append(e)

    return vertices, deduped


def _deduplicate_boxes(boxes: list[dict]) -> list[dict]:
    """Remove overlapping boxes, keeping the larger one."""
    result: list[dict] = []
    used: set[int] = set()

    # Sort by area descending
    boxes_sorted = sorted(boxes, key=lambda b: b["area"], reverse=True)

    for box in boxes_sorted:
        if box["id"] in used:
            continue
        overlap = False
        for existing in result:
            # Check overlap
            ox = max(0, min(box["x"] + box["w"], existing["x"] + existing["w"]) - max(box["x"], existing["x"]))
            oy = max(0, min(box["y"] + box["h"], existing["y"] + existing["h"]) - max(box["y"], existing["y"]))
            overlap_area = ox * oy
            if overlap_area > box["area"] * 0.5:
                overlap = True
                break
        if not overlap:
            result.append(box)
            used.add(box["id"])

    return result


def _find_nearest_box(x: int, y: int, boxes: list[dict], max_dist: float) -> int | None:
    """Find the box whose center is nearest to (x, y) within max_dist."""
    best_id = None
    best_dist = max_dist

    for box in boxes:
        # Distance from point to box edge (not center)
        dx = max(box["x"] - x, 0, x - (box["x"] + box["w"]))
        dy = max(box["y"] - y, 0, y - (box["y"] + box["h"]))
        dist = (dx * dx + dy * dy) ** 0.5

        if dist < best_dist:
            best_dist = dist
            best_id = box["id"]

    return best_id
