"""Chart parser: scatter/line chart data → Rips complex via Delaunay triangulation.

Extracts data points from chart images (peak detection on plot region),
clusters with DBSCAN, constructs Delaunay triangulation as simplicial complex.

Output: vertices (data points/clusters), edges (Delaunay neighbors),
        optional 2-simplices (triangles) as higher-order structure.

Dependencies: scikit-learn (DBSCAN), scipy (Delaunay).
"""

from __future__ import annotations

from pathlib import Path

import numpy as np

from engine import Vertex, Edge, EdgeType, VertexStatus


def parse_chart(image_or_points, source: str = "chart") -> tuple[list[Vertex], list[Edge]]:
    """Parse chart data into simplicial complex vertices and edges.

    Args:
        image_or_points: File path (str/Path) for image, or numpy array of shape (N, 2)
                         for direct point cloud input.
        source: Source label for vertex IDs.

    Returns:
        (vertices, edges) — points/clusters as vertices, Delaunay neighbors as edges.
    """
    if isinstance(image_or_points, np.ndarray) and image_or_points.ndim == 2:
        points = image_or_points
    elif isinstance(image_or_points, (str, Path)):
        points = _extract_points_from_image(str(image_or_points))
    else:
        return [], []

    if points is None or len(points) < 3:
        return [], []

    # Cluster with DBSCAN if too many points
    if len(points) > 200:
        points = _cluster_points(points, max_clusters=100)

    if len(points) < 3:
        return [], []

    # Delaunay triangulation
    try:
        from scipy.spatial import Delaunay
        tri = Delaunay(points)
    except Exception:
        return [], []

    # Create vertices
    vertices: list[Vertex] = []
    vid_map: dict[int, str] = {}

    for i, pt in enumerate(points):
        vid = f"{source}:chart:pt_{i}"
        vertices.append(Vertex(
            id=vid, status=VertexStatus.ACTIVE,
            content=f"data point ({pt[0]:.2f}, {pt[1]:.2f})",
        ))
        vid_map[i] = vid

    # Edges from Delaunay simplices
    edges: list[Edge] = []
    seen_edges: set[tuple[str, str]] = set()

    for simplex in tri.simplices:
        for j in range(3):
            a, b = int(simplex[j]), int(simplex[(j + 1) % 3])
            vid_a, vid_b = vid_map[a], vid_map[b]
            key = (min(vid_a, vid_b), max(vid_a, vid_b))
            if key not in seen_edges:
                seen_edges.add(key)
                edges.append(Edge(source=vid_a, target=vid_b, edge_type=EdgeType.REFERENCE))

    return vertices, edges


def _extract_points_from_image(image_path: str) -> np.ndarray | None:
    """Extract data point positions from a chart image.

    Uses color thresholding + contour detection to find plotted points.
    """
    try:
        import cv2
    except ImportError:
        return None

    img = cv2.imread(image_path)
    if img is None:
        return None

    gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
    h, w = gray.shape

    # Threshold to find dark points on light background
    _, binary = cv2.threshold(gray, 0, 255, cv2.THRESH_BINARY_INV + cv2.THRESH_OTSU)

    # Find contours (data points are small blobs)
    contours, _ = cv2.findContours(binary, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)

    min_area = 4
    max_area = h * w * 0.01  # Points shouldn't be > 1% of image

    points: list[tuple[float, float]] = []
    for cnt in contours:
        area = cv2.contourArea(cnt)
        if min_area <= area <= max_area:
            M = cv2.moments(cnt)
            if M["m00"] > 0:
                cx = M["m10"] / M["m00"]
                cy = M["m01"] / M["m00"]
                # Normalize to [0, 1]
                points.append((cx / w, 1.0 - cy / h))  # Flip y for chart coords

    if not points:
        return None

    return np.array(points)


def _cluster_points(points: np.ndarray, max_clusters: int = 100) -> np.ndarray:
    """Cluster dense point clouds to reduce vertex count."""
    from sklearn.cluster import DBSCAN

    # Adaptive eps from point density
    from scipy.spatial.distance import pdist
    dists = pdist(points)
    eps = np.percentile(dists, 5) if len(dists) > 0 else 0.1

    db = DBSCAN(eps=eps, min_samples=2).fit(points)
    labels = db.labels_

    # Compute cluster centers
    unique_labels = set(labels)
    centers: list[np.ndarray] = []

    for label in unique_labels:
        if label == -1:
            # Noise points: include as individual points
            noise_mask = labels == -1
            for pt in points[noise_mask]:
                centers.append(pt)
        else:
            mask = labels == label
            centers.append(points[mask].mean(axis=0))

    result = np.array(centers)

    # If still too many, subsample
    if len(result) > max_clusters:
        indices = np.random.choice(len(result), max_clusters, replace=False)
        result = result[indices]

    return result
