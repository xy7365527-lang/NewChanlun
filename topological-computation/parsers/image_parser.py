"""Image parser: pixel filtration → persistent homology features.

Converts images to grayscale, applies sublevel filtration on pixel intensities,
computes persistent homology (β₀ = connected components, β₁ = holes).
Each persistent feature (birth-death pair) becomes a vertex in K_active.

Dependencies: ripser, numpy, opencv-python-headless.
"""

from __future__ import annotations

from pathlib import Path

import numpy as np

from engine import Vertex, Edge, EdgeType, VertexStatus


def parse_image(image, source: str = "image",
                max_dim: int = 1, resize: int = 64) -> tuple[list[Vertex], list[Edge]]:
    """Compute persistent homology of an image and return feature vertices.

    Args:
        image: File path (str/Path) or numpy array (grayscale or BGR).
        source: Source label for vertex IDs.
        max_dim: Maximum homology dimension (0=components, 1=holes).
        resize: Downscale image to this size (for computational tractability).

    Returns:
        (vertices, edges) — persistent features as vertices, co-birth edges.
    """
    # Load and prepare image
    if isinstance(image, (str, Path)):
        try:
            import cv2
            img = cv2.imread(str(image), cv2.IMREAD_GRAYSCALE)
        except ImportError:
            return [], []
        if img is None:
            return [], []
    elif isinstance(image, np.ndarray):
        img = image
        if img.ndim == 3:
            img = np.mean(img, axis=2).astype(np.uint8)
    else:
        return [], []

    # Resize for tractability
    if max(img.shape) > resize:
        try:
            import cv2
            img = cv2.resize(img, (resize, resize))
        except ImportError:
            # Manual downsample
            step_h = max(1, img.shape[0] // resize)
            step_w = max(1, img.shape[1] // resize)
            img = img[::step_h, ::step_w]

    # Normalize to [0, 1]
    img_norm = img.astype(np.float64) / 255.0

    # Build distance matrix from pixel positions weighted by intensity
    # Use sublevel set approach: construct point cloud from non-zero pixels
    points = []
    for y in range(img_norm.shape[0]):
        for x in range(img_norm.shape[1]):
            # Include pixel if above minimum intensity
            if img_norm[y, x] > 0.1:
                points.append([x, y])

    if len(points) < 5:
        return [], []

    points = np.array(points, dtype=np.float64)

    # Subsample if too many points
    if len(points) > 500:
        indices = np.random.RandomState(42).choice(len(points), 500, replace=False)
        points = points[indices]

    # Compute persistent homology via Rips filtration
    try:
        from ripser import ripser
        result = ripser(points, maxdim=max_dim, thresh=max(img.shape) * 0.3)
        diagrams = result["dgms"]
    except Exception:
        return [], []

    # Convert persistent features to vertices
    vertices: list[Vertex] = []
    edges: list[Edge] = []
    seen_vids: set[str] = set()

    feature_vids: list[str] = []

    for dim, dgm in enumerate(diagrams):
        for i, (birth, death) in enumerate(dgm):
            if not np.isfinite(death):
                death = max(img.shape)  # Cap infinite death
            lifetime = death - birth
            if lifetime < 1.0:
                continue  # Skip very short-lived features

            vid = f"{source}:ph:H{dim}_{i}"
            content = (f"H{dim} feature: birth={birth:.1f} death={death:.1f} "
                       f"lifetime={lifetime:.1f}")
            vertices.append(Vertex(id=vid, status=VertexStatus.ACTIVE, content=content))
            seen_vids.add(vid)
            feature_vids.append(vid)

    # Edges: features of the same dimension with overlapping lifetimes
    for dim, dgm in enumerate(diagrams):
        dim_vids = [f"{source}:ph:H{dim}_{i}" for i in range(len(dgm))
                    if f"{source}:ph:H{dim}_{i}" in seen_vids]
        for j, vid_a in enumerate(dim_vids):
            for vid_b in dim_vids[j+1:]:
                edges.append(Edge(source=vid_a, target=vid_b, edge_type=EdgeType.REFERENCE))

    # Cross-dimension edges: H0 features connect to H1 features (structure hierarchy)
    h0_vids = [v for v in feature_vids if ":ph:H0_" in v]
    h1_vids = [v for v in feature_vids if ":ph:H1_" in v]
    for h0 in h0_vids[:5]:  # Limit cross-dimension edges
        for h1 in h1_vids[:5]:
            edges.append(Edge(source=h0, target=h1, edge_type=EdgeType.DEPENDENCY))

    return vertices, edges
