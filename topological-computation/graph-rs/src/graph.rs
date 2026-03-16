use pyo3::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Mirrors Python engine.py VertexStatus.
#[pyclass(eq, eq_int, hash)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum VertexStatus {
    Active,
    Contested,
    Folded,
}

impl VertexStatus {
    fn as_str(&self) -> &'static str {
        match self {
            VertexStatus::Active => "active",
            VertexStatus::Contested => "contested",
            VertexStatus::Folded => "folded",
        }
    }

    fn from_str(s: &str) -> PyResult<Self> {
        match s {
            "active" => Ok(VertexStatus::Active),
            "contested" => Ok(VertexStatus::Contested),
            "folded" => Ok(VertexStatus::Folded),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown VertexStatus: {s}"
            ))),
        }
    }
}

#[pymethods]
impl VertexStatus {
    #[getter]
    fn value(&self) -> &'static str {
        self.as_str()
    }

    fn __repr__(&self) -> String {
        format!("VertexStatus.{}", self.as_str())
    }
}

/// Mirrors Python engine.py EdgeType.
#[pyclass(eq, eq_int, hash)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EdgeType {
    Dependency,
    Negation,
    Sublation,
    Reference,
    Fold,
    Cooccurrence,
    TraversalAssociation,
    Articulated,
}

impl EdgeType {
    fn as_str(&self) -> &'static str {
        match self {
            EdgeType::Dependency => "dependency",
            EdgeType::Negation => "negation",
            EdgeType::Sublation => "sublation",
            EdgeType::Reference => "reference",
            EdgeType::Fold => "fold",
            EdgeType::Cooccurrence => "cooccurrence",
            EdgeType::TraversalAssociation => "traversal_association",
            EdgeType::Articulated => "articulated",
        }
    }

    fn from_str(s: &str) -> PyResult<Self> {
        match s {
            "dependency" => Ok(EdgeType::Dependency),
            "negation" => Ok(EdgeType::Negation),
            "sublation" => Ok(EdgeType::Sublation),
            "reference" => Ok(EdgeType::Reference),
            "fold" => Ok(EdgeType::Fold),
            "cooccurrence" => Ok(EdgeType::Cooccurrence),
            "traversal_association" => Ok(EdgeType::TraversalAssociation),
            "articulated" => Ok(EdgeType::Articulated),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown EdgeType: {s}"
            ))),
        }
    }

    fn is_material(&self) -> bool {
        matches!(self, EdgeType::Cooccurrence | EdgeType::TraversalAssociation)
    }

    fn is_concept(&self) -> bool {
        !self.is_material()
    }
}

#[pymethods]
impl EdgeType {
    #[getter]
    fn value(&self) -> &'static str {
        self.as_str()
    }

    fn __repr__(&self) -> String {
        format!("EdgeType.{}", self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Mirrors Python engine.py Vertex (frozen dataclass).
#[pyclass(get_all)]
#[derive(Clone, Debug)]
pub struct Vertex {
    pub id: String,
    pub status: VertexStatus,
    pub content: Option<String>,
    pub created_at: u64,
}

#[pymethods]
impl Vertex {
    #[new]
    #[pyo3(signature = (id, status=None, content=None, created_at=0))]
    fn new(
        id: String,
        status: Option<VertexStatus>,
        content: Option<String>,
        created_at: u64,
    ) -> Self {
        Vertex {
            id,
            status: status.unwrap_or(VertexStatus::Active),
            content,
            created_at,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Vertex(id={:?}, status={:?}, content={:?}, created_at={})",
            self.id,
            self.status.as_str(),
            self.content,
            self.created_at
        )
    }
}

/// Mirrors Python engine.py Edge (frozen dataclass).
#[pyclass(get_all)]
#[derive(Clone, Debug)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub edge_type: EdgeType,
    pub created_at: u64,
    pub surface: Option<String>,
    pub context: Option<String>,
}

#[pymethods]
impl Edge {
    #[new]
    #[pyo3(signature = (source, target, edge_type, created_at=0, surface=None, context=None))]
    fn new(
        source: String,
        target: String,
        edge_type: EdgeType,
        created_at: u64,
        surface: Option<String>,
        context: Option<String>,
    ) -> Self {
        Edge {
            source,
            target,
            edge_type,
            created_at,
            surface,
            context,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Edge({:?} -> {:?}, type={:?})",
            self.source,
            self.target,
            self.edge_type.as_str()
        )
    }
}

// ---------------------------------------------------------------------------
// Edge key type (for O(1) membership test)
// ---------------------------------------------------------------------------

type EdgeKey = (String, String, EdgeType);

fn edge_key(e: &Edge) -> EdgeKey {
    (e.source.clone(), e.target.clone(), e.edge_type.clone())
}

// ---------------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------------

/// Directed typed graph with dual-view (K_full / K_active).
///
/// Internally mutable for Rust efficiency. Python-facing mutation methods
/// return new Graph objects (clone + mutate) to preserve Python-side
/// immutable semantics matching engine.py.
#[pyclass]
#[derive(Clone, Debug)]
pub struct Graph {
    vertices: HashMap<String, Vertex>,
    edges: Vec<Edge>,
    adj_out: HashMap<String, Vec<usize>>,  // vertex_id -> edge indices
    adj_in: HashMap<String, Vec<usize>>,   // vertex_id -> edge indices
    active_ids: HashSet<String>,
    edge_keys: HashSet<EdgeKey>,
}

impl Graph {
    /// Rebuild adjacency indices from edges vec.
    fn rebuild_adjacency(&mut self) {
        self.adj_out.clear();
        self.adj_in.clear();
        for (i, e) in self.edges.iter().enumerate() {
            self.adj_out.entry(e.source.clone()).or_default().push(i);
            self.adj_in.entry(e.target.clone()).or_default().push(i);
        }
    }

    /// Rebuild active_ids from vertices.
    fn rebuild_active_ids(&mut self) {
        self.active_ids = self
            .vertices
            .values()
            .filter(|v| v.status != VertexStatus::Folded)
            .map(|v| v.id.clone())
            .collect();
    }

    /// Rebuild edge_keys from edges.
    fn rebuild_edge_keys(&mut self) {
        self.edge_keys = self.edges.iter().map(edge_key).collect();
    }

    /// Get edge by index.
    fn edge_at(&self, idx: usize) -> &Edge {
        &self.edges[idx]
    }

    /// Active concept-layer edges (excludes material layer, excludes folded vertex edges).
    fn active_concept_edges(&self) -> Vec<&Edge> {
        self.edges
            .iter()
            .filter(|e| {
                self.active_ids.contains(&e.source)
                    && self.active_ids.contains(&e.target)
                    && e.edge_type.is_concept()
            })
            .collect()
    }

    /// All active edges (including material layer).
    fn all_active_edges_inner(&self) -> Vec<&Edge> {
        self.edges
            .iter()
            .filter(|e| {
                self.active_ids.contains(&e.source) && self.active_ids.contains(&e.target)
            })
            .collect()
    }

    /// Undirected edge set for beta_1 (concept-layer only, no self-loops).
    fn undirected_active_edges_inner(&self) -> HashSet<(String, String)> {
        let mut result: HashSet<(String, String)> = HashSet::new();
        for e in &self.edges {
            if self.active_ids.contains(&e.source)
                && self.active_ids.contains(&e.target)
                && e.source != e.target
                && e.edge_type.is_concept()
            {
                let (a, b) = if e.source < e.target {
                    (e.source.clone(), e.target.clone())
                } else {
                    (e.target.clone(), e.source.clone())
                };
                result.insert((a, b));
            }
        }
        result
    }

    /// Count self-loops among active concept edges.
    fn self_loop_count(&self) -> usize {
        self.edges
            .iter()
            .filter(|e| {
                e.source == e.target
                    && self.active_ids.contains(&e.source)
                    && e.edge_type.is_concept()
            })
            .count()
    }

    /// Connected components via union-find.
    fn connected_components(vids: &[String], edges: &HashSet<(String, String)>) -> usize {
        let mut parent: HashMap<&str, &str> = HashMap::new();
        for v in vids {
            parent.insert(v.as_str(), v.as_str());
        }

        fn find<'a>(parent: &mut HashMap<&'a str, &'a str>, x: &'a str) -> &'a str {
            let mut x = x;
            while parent[x] != x {
                let grandparent = parent[parent[x]];
                parent.insert(x, grandparent);
                x = grandparent;
            }
            x
        }

        for (a, b) in edges {
            let ra = find(&mut parent, a.as_str());
            let rb = find(&mut parent, b.as_str());
            if ra != rb {
                parent.insert(ra, rb);
            }
        }

        let roots: HashSet<&str> = vids.iter().map(|v| find(&mut parent, v.as_str())).collect();
        roots.len()
    }

    /// BFS has_path on active subgraph (directed).
    fn has_path_inner(&self, source: &str, target: &str) -> bool {
        if !self.active_ids.contains(source) || !self.active_ids.contains(target) {
            return false;
        }
        let mut visited: HashSet<&str> = HashSet::new();
        let mut queue: VecDeque<&str> = VecDeque::new();
        queue.push_back(source);
        while let Some(cur) = queue.pop_front() {
            if cur == target && cur != source {
                return true;
            }
            if visited.contains(cur) {
                continue;
            }
            visited.insert(cur);
            if let Some(indices) = self.adj_out.get(cur) {
                for &idx in indices {
                    let e = self.edge_at(idx);
                    if self.active_ids.contains(&e.target) && !visited.contains(e.target.as_str()) {
                        queue.push_back(self.edges[idx].target.as_str());
                    }
                }
            }
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Python methods
// ---------------------------------------------------------------------------

#[pymethods]
impl Graph {
    #[new]
    #[pyo3(signature = (vertices=None, edges=None))]
    fn new(vertices: Option<Vec<Vertex>>, edges: Option<Vec<Edge>>) -> Self {
        let mut g = Graph {
            vertices: HashMap::new(),
            edges: Vec::new(),
            adj_out: HashMap::new(),
            adj_in: HashMap::new(),
            active_ids: HashSet::new(),
            edge_keys: HashSet::new(),
        };

        if let Some(verts) = vertices {
            for v in verts {
                g.vertices.insert(v.id.clone(), v);
            }
        }
        if let Some(es) = edges {
            g.edges = es;
        }

        g.rebuild_adjacency();
        g.rebuild_active_ids();
        g.rebuild_edge_keys();
        g
    }

    // -- accessors ----------------------------------------------------------

    /// Return dict of vertices {id: Vertex}.
    #[getter]
    fn vertices(&self) -> HashMap<String, Vertex> {
        self.vertices.clone()
    }

    /// Return list of edges.
    #[getter]
    fn edges(&self) -> Vec<Edge> {
        self.edges.clone()
    }

    /// Look up a single vertex by id.
    fn vertex(&self, vid: &str) -> Option<Vertex> {
        self.vertices.get(vid).cloned()
    }

    /// Return list of active (non-folded) vertex ids.
    fn active_vertex_ids(&self) -> Vec<String> {
        self.active_ids.iter().cloned().collect()
    }

    /// O(1) check whether an edge with (source, target, edge_type) exists.
    fn has_edge_key(&self, source: &str, target: &str, edge_type: EdgeType) -> bool {
        self.edge_keys
            .contains(&(source.to_string(), target.to_string(), edge_type))
    }

    /// Return edge key set as list of (source, target, edge_type_str) tuples.
    #[getter]
    fn get_edge_keys(&self) -> Vec<(String, String, String)> {
        self.edge_keys
            .iter()
            .map(|(s, t, et)| (s.clone(), t.clone(), et.as_str().to_string()))
            .collect()
    }

    /// Return active concept-layer edges (excludes material layer + folded vertices).
    fn active_edges(&self) -> Vec<Edge> {
        self.active_concept_edges().into_iter().cloned().collect()
    }

    /// Return ALL active edges (including material layer).
    fn all_active_edges(&self) -> Vec<Edge> {
        self.all_active_edges_inner().into_iter().cloned().collect()
    }

    /// Return sorted neighbor ids (outgoing + incoming) among active vertices.
    fn neighbors(&self, vid: &str) -> Vec<String> {
        let mut result = self.neighbor_set(vid);
        let mut sorted: Vec<String> = result.drain().collect();
        sorted.sort();
        sorted
    }

    /// Return set of neighbor ids among active vertices.
    fn neighbor_set(&self, vid: &str) -> HashSet<String> {
        let mut result: HashSet<String> = HashSet::new();
        if let Some(indices) = self.adj_out.get(vid) {
            for &idx in indices {
                let e = self.edge_at(idx);
                if self.active_ids.contains(&e.target) {
                    result.insert(e.target.clone());
                }
            }
        }
        if let Some(indices) = self.adj_in.get(vid) {
            for &idx in indices {
                let e = self.edge_at(idx);
                if self.active_ids.contains(&e.source) {
                    result.insert(e.source.clone());
                }
            }
        }
        result
    }

    /// Return sorted outgoing neighbor ids among active vertices.
    fn out_neighbors(&self, vid: &str) -> Vec<String> {
        let mut s = self.out_neighbor_set(vid);
        let mut sorted: Vec<String> = s.drain().collect();
        sorted.sort();
        sorted
    }

    /// Return set of outgoing neighbor ids among active vertices.
    fn out_neighbor_set(&self, vid: &str) -> HashSet<String> {
        let mut result: HashSet<String> = HashSet::new();
        if let Some(indices) = self.adj_out.get(vid) {
            for &idx in indices {
                let e = self.edge_at(idx);
                if self.active_ids.contains(&e.target) {
                    result.insert(e.target.clone());
                }
            }
        }
        result
    }

    /// Return sorted incoming neighbor ids among active vertices.
    fn in_neighbors(&self, vid: &str) -> Vec<String> {
        let mut s = self.in_neighbor_set(vid);
        let mut sorted: Vec<String> = s.drain().collect();
        sorted.sort();
        sorted
    }

    /// Return set of incoming neighbor ids among active vertices.
    fn in_neighbor_set(&self, vid: &str) -> HashSet<String> {
        let mut result: HashSet<String> = HashSet::new();
        if let Some(indices) = self.adj_in.get(vid) {
            for &idx in indices {
                let e = self.edge_at(idx);
                if self.active_ids.contains(&e.source) {
                    result.insert(e.source.clone());
                }
            }
        }
        result
    }

    /// BFS has_path on active subgraph (directed).
    fn has_path(&self, source: &str, target: &str) -> bool {
        self.has_path_inner(source, target)
    }

    /// Return (vertex_ids, edges) within `radius` hops of `center`.
    #[pyo3(signature = (center, radius=1))]
    fn local_subgraph(&self, center: &str, radius: usize) -> (Vec<String>, Vec<Edge>) {
        let mut verts: HashSet<String> = HashSet::new();
        verts.insert(center.to_string());
        for _ in 0..radius {
            let mut new_verts: HashSet<String> = HashSet::new();
            for v in &verts {
                for n in self.neighbors(v.as_str()) {
                    if self.active_ids.contains(&n) {
                        new_verts.insert(n);
                    }
                }
            }
            verts.extend(new_verts);
        }
        let edges: Vec<Edge> = self
            .edges
            .iter()
            .filter(|e| verts.contains(&e.source) && verts.contains(&e.target))
            .cloned()
            .collect();
        let mut sorted_verts: Vec<String> = verts.into_iter().collect();
        sorted_verts.sort();
        (sorted_verts, edges)
    }

    // -- self_loops & undirected projection ----------------------------------

    /// Return self-loops in the active graph.
    fn self_loops(&self) -> Vec<Edge> {
        self.edges
            .iter()
            .filter(|e| {
                e.source == e.target
                    && self.active_ids.contains(&e.source)
            })
            .cloned()
            .collect()
    }

    /// Return undirected edge set from active directed edges (no self-loops).
    /// Each edge is returned as a sorted pair [a, b] where a < b.
    /// Excludes material layer edges — beta_1 measures concept-layer topology only.
    fn undirected_active_edges(&self) -> Vec<(String, String)> {
        self.undirected_active_edges_inner()
            .into_iter()
            .collect()
    }

    // -- beta_1 -------------------------------------------------------------

    /// Compute beta_1 of the active subgraph.
    /// beta_1 = |E_undirected| - |V_active| + connected_components + self_loops
    fn beta_1(&self) -> usize {
        let active_vids: Vec<String> = self.active_ids.iter().cloned().collect();
        if active_vids.is_empty() {
            return 0;
        }
        let undirected = self.undirected_active_edges_inner();
        let n_loops = self.self_loop_count();
        let n_v = active_vids.len();
        let n_e = undirected.len();
        let c = Self::connected_components(&active_vids, &undirected);
        n_e - n_v + c + n_loops
    }

    // -- mutation (returns new Graph) ----------------------------------------

    /// Add a vertex, return new Graph.
    fn add_vertex(&self, v: Vertex) -> Self {
        let mut g = self.clone();
        let is_active = v.status != VertexStatus::Folded;
        let vid = v.id.clone();
        g.vertices.insert(vid.clone(), v);
        if is_active {
            g.active_ids.insert(vid);
        }
        g
    }

    /// Add a single edge, return new Graph.
    fn add_edge(&self, e: Edge) -> Self {
        let mut g = self.clone();
        let idx = g.edges.len();
        let key = edge_key(&e);
        g.adj_out.entry(e.source.clone()).or_default().push(idx);
        g.adj_in.entry(e.target.clone()).or_default().push(idx);
        g.edge_keys.insert(key);
        g.edges.push(e);
        g
    }

    /// Add multiple edges in one operation, return new Graph.
    fn add_edges_batch(&self, edges: Vec<Edge>) -> Self {
        if edges.is_empty() {
            return self.clone();
        }
        let mut g = self.clone();
        for e in edges {
            let idx = g.edges.len();
            let key = edge_key(&e);
            g.adj_out.entry(e.source.clone()).or_default().push(idx);
            g.adj_in.entry(e.target.clone()).or_default().push(idx);
            g.edge_keys.insert(key);
            g.edges.push(e);
        }
        g
    }

    /// Add multiple vertices and edges in one operation, return new Graph.
    fn add_vertices_and_edges_batch(&self, vertices: Vec<Vertex>, edges: Vec<Edge>) -> Self {
        let mut g = self.clone();
        for v in vertices {
            let is_active = v.status != VertexStatus::Folded;
            let vid = v.id.clone();
            g.vertices.insert(vid.clone(), v);
            if is_active {
                g.active_ids.insert(vid);
            }
        }
        for e in edges {
            let idx = g.edges.len();
            let key = edge_key(&e);
            g.adj_out.entry(e.source.clone()).or_default().push(idx);
            g.adj_in.entry(e.target.clone()).or_default().push(idx);
            g.edge_keys.insert(key);
            g.edges.push(e);
        }
        g
    }

    /// Set vertex status, return new Graph.
    fn set_vertex_status(&self, vid: &str, status: VertexStatus) -> PyResult<Self> {
        let mut g = self.clone();
        let v = g
            .vertices
            .get(vid)
            .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err(format!("Vertex {vid} not found")))?
            .clone();
        let old_active = v.status != VertexStatus::Folded;
        let new_active = status != VertexStatus::Folded;
        let updated = Vertex {
            id: v.id,
            status,
            content: v.content,
            created_at: v.created_at,
        };
        g.vertices.insert(vid.to_string(), updated);
        if old_active && !new_active {
            g.active_ids.remove(vid);
        } else if !old_active && new_active {
            g.active_ids.insert(vid.to_string());
        }
        Ok(g)
    }

    /// Merge `remove` into `keep`. Redirect all edges, mark `remove` as folded.
    fn merge_vertices(&self, keep: &str, remove: &str) -> Self {
        let mut g = self.clone();

        // Mark removed vertex as folded
        if let Some(v) = g.vertices.get(remove).cloned() {
            g.vertices.insert(
                remove.to_string(),
                Vertex {
                    id: v.id,
                    status: VertexStatus::Folded,
                    content: v.content,
                    created_at: v.created_at,
                },
            );
        }

        // Redirect edges
        let new_edges: Vec<Edge> = g
            .edges
            .iter()
            .map(|e| {
                let src = if e.source == remove {
                    keep.to_string()
                } else {
                    e.source.clone()
                };
                let tgt = if e.target == remove {
                    keep.to_string()
                } else {
                    e.target.clone()
                };
                Edge {
                    source: src,
                    target: tgt,
                    edge_type: e.edge_type.clone(),
                    created_at: e.created_at,
                    surface: e.surface.clone(),
                    context: e.context.clone(),
                }
            })
            .collect();

        g.edges = new_edges;
        g.rebuild_adjacency();
        g.rebuild_active_ids();
        g.rebuild_edge_keys();
        g
    }

    // -- diagnostics --------------------------------------------------------

    fn __repr__(&self) -> String {
        format!(
            "Graph(vertices={}, edges={}, active={}, beta_1={})",
            self.vertices.len(),
            self.edges.len(),
            self.active_ids.len(),
            self.beta_1()
        )
    }

    fn __len__(&self) -> usize {
        self.vertices.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_v(id: &str) -> Vertex {
        Vertex {
            id: id.to_string(),
            status: VertexStatus::Active,
            content: None,
            created_at: 0,
        }
    }

    fn make_folded_v(id: &str) -> Vertex {
        Vertex {
            id: id.to_string(),
            status: VertexStatus::Folded,
            content: None,
            created_at: 0,
        }
    }

    fn make_edge(src: &str, tgt: &str, et: EdgeType) -> Edge {
        Edge {
            source: src.to_string(),
            target: tgt.to_string(),
            edge_type: et,
            created_at: 0,
            surface: None,
            context: None,
        }
    }

    fn make_edge_with_surface(src: &str, tgt: &str, et: EdgeType, surface: &str) -> Edge {
        Edge {
            source: src.to_string(),
            target: tgt.to_string(),
            edge_type: et,
            created_at: 0,
            surface: Some(surface.to_string()),
            context: None,
        }
    }

    // -- Empty graph --------------------------------------------------------

    #[test]
    fn test_empty_graph() {
        let g = Graph::new(None, None);
        assert_eq!(g.vertices.len(), 0);
        assert_eq!(g.edges.len(), 0);
        assert_eq!(g.beta_1(), 0);
        assert!(g.active_vertex_ids().is_empty());
        assert!(g.active_edges().is_empty());
        assert!(g.all_active_edges().is_empty());
        assert!(g.self_loops().is_empty());
        assert!(g.undirected_active_edges().is_empty());
    }

    // -- Vertex operations --------------------------------------------------

    #[test]
    fn test_add_vertex() {
        let g = Graph::new(None, None);
        let g2 = g.add_vertex(make_v("a"));
        assert_eq!(g2.vertices.len(), 1);
        assert!(g2.active_ids.contains("a"));
        // Original unchanged (immutable semantics)
        assert_eq!(g.vertices.len(), 0);
    }

    #[test]
    fn test_add_folded_vertex() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_folded_v("a"));
        assert_eq!(g.vertices.len(), 1);
        assert!(!g.active_ids.contains("a"));
        assert!(g.active_vertex_ids().is_empty());
    }

    #[test]
    fn test_vertex_lookup() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let v = g.vertex("a");
        assert!(v.is_some());
        assert_eq!(v.unwrap().id, "a");
        assert!(g.vertex("nonexistent").is_none());
    }

    #[test]
    fn test_vertex_with_content() {
        let g = Graph::new(None, None);
        let v = Vertex {
            id: "a".to_string(),
            status: VertexStatus::Active,
            content: Some("test content".to_string()),
            created_at: 42,
        };
        let g = g.add_vertex(v);
        let retrieved = g.vertex("a").unwrap();
        assert_eq!(retrieved.content, Some("test content".to_string()));
        assert_eq!(retrieved.created_at, 42);
    }

    // -- Edge operations ----------------------------------------------------

    #[test]
    fn test_add_edge_and_neighbors() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        assert_eq!(g.edges.len(), 1);
        assert!(g.has_edge_key("a", "b", EdgeType::Dependency));
        assert!(!g.has_edge_key("b", "a", EdgeType::Dependency));
        assert_eq!(g.neighbors("a"), vec!["b".to_string()]);
        assert_eq!(g.neighbors("b"), vec!["a".to_string()]);
        assert_eq!(g.out_neighbors("a"), vec!["b".to_string()]);
        assert!(g.out_neighbors("b").is_empty());
        assert_eq!(g.in_neighbors("b"), vec!["a".to_string()]);
        assert!(g.in_neighbors("a").is_empty());
    }

    #[test]
    fn test_edge_with_surface_and_context() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let e = Edge {
            source: "a".to_string(),
            target: "b".to_string(),
            edge_type: EdgeType::Dependency,
            created_at: 10,
            surface: Some("negates".to_string()),
            context: Some("A negates B in chapter 3".to_string()),
        };
        let g = g.add_edge(e);
        let edges = g.edges.clone();
        assert_eq!(edges[0].surface, Some("negates".to_string()));
        assert_eq!(edges[0].context, Some("A negates B in chapter 3".to_string()));
    }

    #[test]
    fn test_multiple_edge_types_same_pair() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Negation));
        assert_eq!(g.edges.len(), 2);
        assert!(g.has_edge_key("a", "b", EdgeType::Dependency));
        assert!(g.has_edge_key("a", "b", EdgeType::Negation));
        assert!(!g.has_edge_key("a", "b", EdgeType::Reference));
    }

    // -- Neighbor sets (deduplicated) ---------------------------------------

    #[test]
    fn test_neighbor_set_deduplication() {
        // Multiple edges from a->b should not produce duplicate neighbors
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Negation));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Reference));
        assert_eq!(g.out_neighbors("a"), vec!["b".to_string()]);
        assert_eq!(g.out_neighbor_set("a").len(), 1);
    }

    #[test]
    fn test_neighbors_exclude_folded() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g = g.set_vertex_status("b", VertexStatus::Folded).unwrap();
        // b is folded, so a has no active neighbors
        assert!(g.neighbors("a").is_empty());
        assert!(g.out_neighbors("a").is_empty());
    }

    // -- Active edges -------------------------------------------------------

    #[test]
    fn test_active_edges_exclude_material() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Cooccurrence));
        let g = g.add_edge(make_edge("a", "b", EdgeType::TraversalAssociation));
        // active_edges: only concept layer
        assert_eq!(g.active_edges().len(), 1);
        assert_eq!(g.active_edges()[0].edge_type, EdgeType::Dependency);
        // all_active_edges: includes material layer
        assert_eq!(g.all_active_edges().len(), 3);
    }

    #[test]
    fn test_active_edges_exclude_folded_vertex_edges() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_vertex(make_v("c"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g = g.add_edge(make_edge("b", "c", EdgeType::Negation));
        let g = g.set_vertex_status("b", VertexStatus::Folded).unwrap();
        // Both edges touch b (folded), so no active edges
        assert!(g.active_edges().is_empty());
        assert!(g.all_active_edges().is_empty());
    }

    // -- Self-loops ---------------------------------------------------------

    #[test]
    fn test_self_loops() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_edge(make_edge("a", "a", EdgeType::Dependency));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Negation));
        let loops = g.self_loops();
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].source, "a");
        assert_eq!(loops[0].target, "a");
    }

    #[test]
    fn test_self_loops_include_material() {
        // Python self_loops() includes all edge types
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_edge(make_edge("a", "a", EdgeType::Cooccurrence));
        assert_eq!(g.self_loops().len(), 1);
    }

    #[test]
    fn test_self_loops_exclude_folded() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_edge(make_edge("a", "a", EdgeType::Dependency));
        let g = g.set_vertex_status("a", VertexStatus::Folded).unwrap();
        assert!(g.self_loops().is_empty());
    }

    // -- Undirected active edges --------------------------------------------

    #[test]
    fn test_undirected_active_edges() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        // Both directions => one undirected edge
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g = g.add_edge(make_edge("b", "a", EdgeType::Negation));
        let und = g.undirected_active_edges();
        assert_eq!(und.len(), 1);
    }

    #[test]
    fn test_undirected_excludes_self_loops() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_edge(make_edge("a", "a", EdgeType::Dependency));
        assert!(g.undirected_active_edges().is_empty());
    }

    #[test]
    fn test_undirected_excludes_material() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Cooccurrence));
        assert!(g.undirected_active_edges().is_empty());
    }

    // -- Edge keys property -------------------------------------------------

    #[test]
    fn test_edge_keys_getter() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let keys = g.get_edge_keys();
        assert_eq!(keys.len(), 1);
        let (s, t, et) = &keys[0];
        assert_eq!(s, "a");
        assert_eq!(t, "b");
        assert_eq!(et, "dependency");
    }

    // -- Beta_1 computation -------------------------------------------------

    #[test]
    fn test_beta_1_triangle() {
        // Triangle: a->b->c->a => beta_1 = 1
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_vertex(make_v("c"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g = g.add_edge(make_edge("b", "c", EdgeType::Dependency));
        let g = g.add_edge(make_edge("c", "a", EdgeType::Dependency));
        // undirected edges: {a,b}, {b,c}, {a,c} = 3
        // vertices = 3, components = 1
        // beta_1 = 3 - 3 + 1 = 1
        assert_eq!(g.beta_1(), 1);
    }

    #[test]
    fn test_beta_1_tree() {
        // Tree: a->b, a->c => beta_1 = 0
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_vertex(make_v("c"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g = g.add_edge(make_edge("a", "c", EdgeType::Dependency));
        // undirected: {a,b}, {a,c} = 2, V=3, C=1
        // beta_1 = 2 - 3 + 1 = 0
        assert_eq!(g.beta_1(), 0);
    }

    #[test]
    fn test_beta_1_with_self_loop() {
        // a->b, a->a (self-loop) => beta_1 = 1
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g = g.add_edge(make_edge("a", "a", EdgeType::Negation));
        // undirected: {a,b} = 1, V=2, C=1, self_loops=1
        // beta_1 = 1 - 2 + 1 + 1 = 1
        assert_eq!(g.beta_1(), 1);
    }

    #[test]
    fn test_beta_1_disconnected() {
        // Two disconnected vertices => beta_1 = 0
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        // E=0, V=2, C=2
        // beta_1 = 0 - 2 + 2 = 0
        assert_eq!(g.beta_1(), 0);
    }

    #[test]
    fn test_beta_1_two_triangles_sharing_edge() {
        // a-b-c-a and b-c-d-b => beta_1 = 2
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_vertex(make_v("c"));
        let g = g.add_vertex(make_v("d"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g = g.add_edge(make_edge("b", "c", EdgeType::Dependency));
        let g = g.add_edge(make_edge("c", "a", EdgeType::Dependency));
        let g = g.add_edge(make_edge("c", "d", EdgeType::Dependency));
        let g = g.add_edge(make_edge("d", "b", EdgeType::Dependency));
        // undirected: {a,b}, {b,c}, {a,c}, {c,d}, {b,d} = 5
        // V=4, C=1
        // beta_1 = 5 - 4 + 1 = 2
        assert_eq!(g.beta_1(), 2);
    }

    #[test]
    fn test_material_edges_excluded_from_beta_1() {
        // Two vertices with only a COOCCURRENCE edge => beta_1 = 0
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Cooccurrence));
        assert_eq!(g.beta_1(), 0);
    }

    #[test]
    fn test_beta_1_material_self_loop_excluded() {
        // Self-loop with material type should not count in beta_1
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_edge(make_edge("a", "a", EdgeType::Cooccurrence));
        assert_eq!(g.beta_1(), 0);
    }

    // -- has_path -----------------------------------------------------------

    #[test]
    fn test_has_path() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_vertex(make_v("c"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g = g.add_edge(make_edge("b", "c", EdgeType::Dependency));
        assert!(g.has_path("a", "c"));
        assert!(!g.has_path("c", "a"));
    }

    #[test]
    fn test_has_path_through_folded() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_vertex(make_v("c"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g = g.add_edge(make_edge("b", "c", EdgeType::Dependency));
        let g = g.set_vertex_status("b", VertexStatus::Folded).unwrap();
        // b is folded, path a->b->c is broken
        assert!(!g.has_path("a", "c"));
    }

    #[test]
    fn test_has_path_nonexistent_vertices() {
        let g = Graph::new(None, None);
        assert!(!g.has_path("x", "y"));
    }

    #[test]
    fn test_has_path_same_vertex() {
        // has_path(a, a) should be false (Python: cur == target && cur != source)
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        assert!(!g.has_path("a", "a"));
    }

    #[test]
    fn test_has_path_self_loop() {
        // Even with self-loop, has_path(a, a) requires going through loop
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_edge(make_edge("a", "a", EdgeType::Dependency));
        // In current BFS: source is visited first, then target check fails for same vertex
        // But the self-loop edge goes a->a, so queue gets 'a' again, but it's in visited
        assert!(!g.has_path("a", "a"));
    }

    // -- set_vertex_status --------------------------------------------------

    #[test]
    fn test_set_vertex_status() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        assert!(g.active_ids.contains("a"));
        let g = g.set_vertex_status("a", VertexStatus::Folded).unwrap();
        assert!(!g.active_ids.contains("a"));
    }

    #[test]
    fn test_set_vertex_status_contested() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.set_vertex_status("a", VertexStatus::Contested).unwrap();
        // Contested is still "active" (not folded)
        assert!(g.active_ids.contains("a"));
        let v = g.vertex("a").unwrap();
        assert_eq!(v.status, VertexStatus::Contested);
    }

    #[test]
    fn test_set_vertex_status_unfold() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_folded_v("a"));
        assert!(!g.active_ids.contains("a"));
        let g = g.set_vertex_status("a", VertexStatus::Active).unwrap();
        assert!(g.active_ids.contains("a"));
    }

    #[test]
    fn test_set_vertex_status_not_found() {
        let g = Graph::new(None, None);
        let result = g.set_vertex_status("nonexistent", VertexStatus::Active);
        assert!(result.is_err());
    }

    // -- merge_vertices -----------------------------------------------------

    #[test]
    fn test_merge_vertices() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_vertex(make_v("c"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g = g.add_edge(make_edge("b", "c", EdgeType::Negation));
        let g = g.merge_vertices("a", "b");
        // b is now folded, edges redirected: a->a (self-loop), a->c
        assert!(!g.active_ids.contains("b"));
        assert!(g.has_edge_key("a", "a", EdgeType::Dependency));
        assert!(g.has_edge_key("a", "c", EdgeType::Negation));
    }

    #[test]
    fn test_merge_preserves_surface() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_vertex(make_v("c"));
        let g = g.add_edge(make_edge_with_surface("b", "c", EdgeType::Dependency, "implies"));
        let g = g.merge_vertices("a", "b");
        let edges = g.edges.clone();
        // Redirected edge a->c should preserve surface
        assert_eq!(edges[0].source, "a");
        assert_eq!(edges[0].target, "c");
        assert_eq!(edges[0].surface, Some("implies".to_string()));
    }

    // -- Batch operations ---------------------------------------------------

    #[test]
    fn test_add_edges_batch() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_vertex(make_v("c"));
        let edges = vec![
            make_edge("a", "b", EdgeType::Dependency),
            make_edge("b", "c", EdgeType::Reference),
        ];
        let g = g.add_edges_batch(edges);
        assert_eq!(g.edges.len(), 2);
        assert!(g.has_edge_key("a", "b", EdgeType::Dependency));
        assert!(g.has_edge_key("b", "c", EdgeType::Reference));
    }

    #[test]
    fn test_add_edges_batch_empty() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g2 = g.add_edges_batch(vec![]);
        assert_eq!(g2.vertices.len(), 1);
        assert_eq!(g2.edges.len(), 0);
    }

    #[test]
    fn test_add_vertices_and_edges_batch() {
        let g = Graph::new(None, None);
        let verts = vec![make_v("a"), make_v("b"), make_v("c")];
        let edges = vec![
            make_edge("a", "b", EdgeType::Dependency),
            make_edge("b", "c", EdgeType::Reference),
        ];
        let g = g.add_vertices_and_edges_batch(verts, edges);
        assert_eq!(g.vertices.len(), 3);
        assert_eq!(g.edges.len(), 2);
        assert!(g.has_edge_key("a", "b", EdgeType::Dependency));
        assert!(g.has_edge_key("b", "c", EdgeType::Reference));
    }

    #[test]
    fn test_add_vertices_and_edges_batch_no_edges() {
        let g = Graph::new(None, None);
        let verts = vec![make_v("a"), make_v("b")];
        let g = g.add_vertices_and_edges_batch(verts, vec![]);
        assert_eq!(g.vertices.len(), 2);
        assert_eq!(g.edges.len(), 0);
        assert!(g.active_ids.contains("a"));
        assert!(g.active_ids.contains("b"));
    }

    // -- local_subgraph -----------------------------------------------------

    #[test]
    fn test_local_subgraph() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_vertex(make_v("c"));
        let g = g.add_vertex(make_v("d"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g = g.add_edge(make_edge("b", "c", EdgeType::Dependency));
        let g = g.add_edge(make_edge("c", "d", EdgeType::Dependency));
        let (verts, edges) = g.local_subgraph("b", 1);
        assert!(verts.contains(&"a".to_string()));
        assert!(verts.contains(&"b".to_string()));
        assert!(verts.contains(&"c".to_string()));
        assert!(!verts.contains(&"d".to_string()));
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_local_subgraph_radius_0() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let (verts, edges) = g.local_subgraph("a", 0);
        assert_eq!(verts, vec!["a".to_string()]);
        // No edges because b is not in vertex set
        assert!(edges.is_empty());
    }

    #[test]
    fn test_local_subgraph_radius_2() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        let g = g.add_vertex(make_v("c"));
        let g = g.add_vertex(make_v("d"));
        let g = g.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g = g.add_edge(make_edge("b", "c", EdgeType::Dependency));
        let g = g.add_edge(make_edge("c", "d", EdgeType::Dependency));
        let (verts, _edges) = g.local_subgraph("a", 2);
        assert!(verts.contains(&"a".to_string()));
        assert!(verts.contains(&"b".to_string()));
        assert!(verts.contains(&"c".to_string()));
        assert!(!verts.contains(&"d".to_string()));
    }

    // -- Constructor with initial data --------------------------------------

    #[test]
    fn test_constructor_with_data() {
        let verts = vec![make_v("a"), make_v("b")];
        let edges = vec![make_edge("a", "b", EdgeType::Dependency)];
        let g = Graph::new(Some(verts), Some(edges));
        assert_eq!(g.vertices.len(), 2);
        assert_eq!(g.edges.len(), 1);
        assert!(g.has_edge_key("a", "b", EdgeType::Dependency));
        assert!(g.active_ids.contains("a"));
        assert!(g.active_ids.contains("b"));
    }

    // -- __repr__ and __len__ -----------------------------------------------

    #[test]
    fn test_repr() {
        let g = Graph::new(None, None);
        let g = g.add_vertex(make_v("a"));
        let repr = g.__repr__();
        assert!(repr.contains("vertices=1"));
        assert!(repr.contains("active=1"));
    }

    #[test]
    fn test_len() {
        let g = Graph::new(None, None);
        assert_eq!(g.__len__(), 0);
        let g = g.add_vertex(make_v("a"));
        let g = g.add_vertex(make_v("b"));
        assert_eq!(g.__len__(), 2);
    }

    // -- Enum methods -------------------------------------------------------

    #[test]
    fn test_edge_type_material_concept() {
        assert!(EdgeType::Cooccurrence.is_material());
        assert!(EdgeType::TraversalAssociation.is_material());
        assert!(!EdgeType::Dependency.is_material());
        assert!(!EdgeType::Negation.is_material());
        assert!(!EdgeType::Sublation.is_material());
        assert!(!EdgeType::Reference.is_material());
        assert!(!EdgeType::Fold.is_material());
        assert!(!EdgeType::Articulated.is_material());

        assert!(EdgeType::Dependency.is_concept());
        assert!(!EdgeType::Cooccurrence.is_concept());
    }

    #[test]
    fn test_vertex_status_from_str() {
        assert_eq!(VertexStatus::from_str("active").unwrap(), VertexStatus::Active);
        assert_eq!(VertexStatus::from_str("contested").unwrap(), VertexStatus::Contested);
        assert_eq!(VertexStatus::from_str("folded").unwrap(), VertexStatus::Folded);
        assert!(VertexStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_edge_type_from_str() {
        assert_eq!(EdgeType::from_str("dependency").unwrap(), EdgeType::Dependency);
        assert_eq!(EdgeType::from_str("negation").unwrap(), EdgeType::Negation);
        assert_eq!(EdgeType::from_str("sublation").unwrap(), EdgeType::Sublation);
        assert_eq!(EdgeType::from_str("reference").unwrap(), EdgeType::Reference);
        assert_eq!(EdgeType::from_str("fold").unwrap(), EdgeType::Fold);
        assert_eq!(EdgeType::from_str("cooccurrence").unwrap(), EdgeType::Cooccurrence);
        assert_eq!(EdgeType::from_str("traversal_association").unwrap(), EdgeType::TraversalAssociation);
        assert_eq!(EdgeType::from_str("articulated").unwrap(), EdgeType::Articulated);
        assert!(EdgeType::from_str("invalid").is_err());
    }

    // -- All edge types in beta_1 -------------------------------------------

    #[test]
    fn test_all_concept_edge_types_count_in_beta_1() {
        let concept_types = vec![
            EdgeType::Dependency,
            EdgeType::Negation,
            EdgeType::Sublation,
            EdgeType::Reference,
            EdgeType::Fold,
            EdgeType::Articulated,
        ];
        for et in concept_types {
            let g = Graph::new(None, None);
            let g = g.add_vertex(make_v("a"));
            let g = g.add_vertex(make_v("b"));
            let g = g.add_vertex(make_v("c"));
            let g = g.add_edge(make_edge("a", "b", et.clone()));
            let g = g.add_edge(make_edge("b", "c", et.clone()));
            let g = g.add_edge(make_edge("c", "a", et));
            assert_eq!(g.beta_1(), 1, "Triangle with concept edge type should have beta_1 = 1");
        }
    }

    // -- Immutability semantics ---------------------------------------------

    #[test]
    fn test_immutable_semantics_add_vertex() {
        let g1 = Graph::new(None, None);
        let g2 = g1.add_vertex(make_v("a"));
        assert_eq!(g1.vertices.len(), 0);
        assert_eq!(g2.vertices.len(), 1);
    }

    #[test]
    fn test_immutable_semantics_add_edge() {
        let g1 = Graph::new(None, None);
        let g1 = g1.add_vertex(make_v("a"));
        let g1 = g1.add_vertex(make_v("b"));
        let g2 = g1.add_edge(make_edge("a", "b", EdgeType::Dependency));
        assert_eq!(g1.edges.len(), 0);
        assert_eq!(g2.edges.len(), 1);
    }

    #[test]
    fn test_immutable_semantics_set_status() {
        let g1 = Graph::new(None, None);
        let g1 = g1.add_vertex(make_v("a"));
        let g2 = g1.set_vertex_status("a", VertexStatus::Folded).unwrap();
        assert!(g1.active_ids.contains("a"));
        assert!(!g2.active_ids.contains("a"));
    }

    #[test]
    fn test_immutable_semantics_merge() {
        let g1 = Graph::new(None, None);
        let g1 = g1.add_vertex(make_v("a"));
        let g1 = g1.add_vertex(make_v("b"));
        let g1 = g1.add_edge(make_edge("a", "b", EdgeType::Dependency));
        let g2 = g1.merge_vertices("a", "b");
        // g1 should be unchanged
        assert!(g1.active_ids.contains("b"));
        assert!(!g2.active_ids.contains("b"));
    }
}
