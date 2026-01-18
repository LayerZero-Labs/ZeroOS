//! Parallel Graph BFS Implementation
//!
//! Frontier-sliced BFS for parallel execution.
//! Each slice of the frontier can be processed independently.

#![no_std]

/// Maximum number of vertices supported
pub const MAX_VERTICES: usize = 64;
/// Maximum number of edges supported
pub const MAX_EDGES: usize = 256;

/// Compact graph representation using adjacency lists.
/// Edges are stored contiguously, with offsets per vertex.
pub struct Graph {
    /// Number of vertices
    pub num_vertices: usize,
    /// Number of edges
    pub num_edges: usize,
    /// Offset into edges array for each vertex (CSR format)
    pub offsets: [usize; MAX_VERTICES + 1],
    /// Edge destinations (packed adjacency lists)
    pub edges: [usize; MAX_EDGES],
}

impl Graph {
    pub const fn new() -> Self {
        Self {
            num_vertices: 0,
            num_edges: 0,
            offsets: [0; MAX_VERTICES + 1],
            edges: [0; MAX_EDGES],
        }
    }

    /// Add a directed edge from `from` to `to`.
    pub fn add_edge(&mut self, from: usize, to: usize) {
        assert!(from < MAX_VERTICES && to < MAX_VERTICES);

        // This simple implementation requires edges to be added in order
        // For a more robust version, build from edge list at the end
        self.edges[self.num_edges] = to;
        self.num_edges += 1;
    }

    /// Get neighbors of a vertex.
    pub fn neighbors(&self, v: usize) -> &[usize] {
        let start = self.offsets[v];
        let end = self.offsets[v + 1];
        &self.edges[start..end]
    }

    /// Build graph from edge list (simpler API).
    pub fn from_edges(num_vertices: usize, edges: &[(usize, usize)]) -> Self {
        let mut graph = Self::new();
        graph.num_vertices = num_vertices;

        // Count edges per vertex
        let mut counts = [0usize; MAX_VERTICES];
        for &(from, _to) in edges {
            counts[from] += 1;
        }

        // Compute offsets (prefix sum)
        let mut offset = 0;
        for v in 0..num_vertices {
            graph.offsets[v] = offset;
            offset += counts[v];
        }
        graph.offsets[num_vertices] = offset;
        graph.num_edges = offset;

        // Fill in edges (need to track current position per vertex)
        let mut current = [0usize; MAX_VERTICES];
        for v in 0..num_vertices {
            current[v] = graph.offsets[v];
        }

        for &(from, to) in edges {
            let pos = current[from];
            graph.edges[pos] = to;
            current[from] += 1;
        }

        graph
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

/// BFS state and results.
pub struct BfsResult {
    /// Distance from source (-1 if unreachable)
    pub distance: [i32; MAX_VERTICES],
    /// Parent in BFS tree (-1 for source or unreachable)
    pub parent: [i32; MAX_VERTICES],
    /// Number of vertices reached
    pub num_reached: usize,
}

impl BfsResult {
    pub fn new() -> Self {
        Self {
            distance: [-1; MAX_VERTICES],
            parent: [-1; MAX_VERTICES],
            num_reached: 0,
        }
    }
}

impl Default for BfsResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Frontier representation for BFS.
/// Double-buffered for level-synchronous BFS.
pub struct Frontier {
    /// Current frontier vertices
    pub current: [usize; MAX_VERTICES],
    pub current_len: usize,
    /// Next frontier vertices
    pub next: [usize; MAX_VERTICES],
    pub next_len: usize,
}

impl Frontier {
    pub fn new() -> Self {
        Self {
            current: [0; MAX_VERTICES],
            current_len: 0,
            next: [0; MAX_VERTICES],
            next_len: 0,
        }
    }

    pub fn add_to_next(&mut self, v: usize) {
        self.next[self.next_len] = v;
        self.next_len += 1;
    }

    pub fn swap(&mut self) {
        // Swap current and next
        for i in 0..self.next_len {
            self.current[i] = self.next[i];
        }
        self.current_len = self.next_len;
        self.next_len = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.current_len == 0
    }
}

impl Default for Frontier {
    fn default() -> Self {
        Self::new()
    }
}

/// Process a slice of the frontier (parallel-friendly).
/// Each slice can be processed by a different thread.
/// Returns vertices to add to next frontier.
pub fn process_frontier_slice(
    graph: &Graph,
    frontier_slice: &[usize],
    visited: &[bool; MAX_VERTICES],
    current_distance: i32,
) -> ([usize; MAX_VERTICES], usize, [(usize, i32, i32); MAX_VERTICES], usize) {
    let mut new_vertices = [0usize; MAX_VERTICES];
    let mut new_count = 0;
    let mut updates = [(0usize, 0i32, 0i32); MAX_VERTICES]; // (vertex, distance, parent)
    let mut update_count = 0;

    // Process each vertex in this slice
    for &v in frontier_slice {
        // Visit all neighbors
        for &neighbor in graph.neighbors(v) {
            if !visited[neighbor] {
                // Found unvisited vertex
                new_vertices[new_count] = neighbor;
                new_count += 1;

                updates[update_count] = (neighbor, current_distance + 1, v as i32);
                update_count += 1;
            }
        }
    }

    (new_vertices, new_count, updates, update_count)
}

/// Standard BFS from a source vertex.
pub fn bfs(graph: &Graph, source: usize) -> BfsResult {
    let mut result = BfsResult::new();
    let mut visited = [false; MAX_VERTICES];
    let mut frontier = Frontier::new();

    // Initialize source
    result.distance[source] = 0;
    visited[source] = true;
    frontier.current[0] = source;
    frontier.current_len = 1;
    result.num_reached = 1;

    let mut current_distance = 0;

    // Process level by level
    while !frontier.is_empty() {
        // TODO: With threading, partition frontier into slices
        // Each slice can be processed independently
        let (new_verts, new_count, updates, update_count) =
            process_frontier_slice(graph, &frontier.current[..frontier.current_len], &visited, current_distance);

        // Apply updates (this part needs synchronization with threading)
        for i in 0..update_count {
            let (v, dist, parent) = updates[i];
            if !visited[v] {
                visited[v] = true;
                result.distance[v] = dist;
                result.parent[v] = parent;
                frontier.add_to_next(v);
                result.num_reached += 1;
            }
        }

        // Avoid unused warning
        let _ = (new_verts, new_count);

        frontier.swap();
        current_distance += 1;
    }

    result
}

/// Multi-source BFS (useful for connected components).
pub fn multi_source_bfs(graph: &Graph, sources: &[usize]) -> BfsResult {
    let mut result = BfsResult::new();
    let mut visited = [false; MAX_VERTICES];
    let mut frontier = Frontier::new();

    // Initialize all sources
    for &source in sources {
        result.distance[source] = 0;
        visited[source] = true;
        frontier.current[frontier.current_len] = source;
        frontier.current_len += 1;
        result.num_reached += 1;
    }

    let mut current_distance = 0;

    while !frontier.is_empty() {
        let (_, _, updates, update_count) =
            process_frontier_slice(graph, &frontier.current[..frontier.current_len], &visited, current_distance);

        for i in 0..update_count {
            let (v, dist, parent) = updates[i];
            if !visited[v] {
                visited[v] = true;
                result.distance[v] = dist;
                result.parent[v] = parent;
                frontier.add_to_next(v);
                result.num_reached += 1;
            }
        }

        frontier.swap();
        current_distance += 1;
    }

    result
}

/// Batch BFS from multiple independent sources.
/// Each BFS is completely independent (embarrassingly parallel).
pub fn batch_bfs(graph: &Graph, sources: &[usize], results: &mut [BfsResult]) {
    assert_eq!(sources.len(), results.len());

    // Each BFS can be done by a different thread
    for (source, result) in sources.iter().zip(results.iter_mut()) {
        *result = bfs(graph, *source);
    }
}

/// Reconstruct path from source to target using BFS result.
pub fn reconstruct_path(result: &BfsResult, target: usize, path: &mut [usize; MAX_VERTICES]) -> usize {
    if result.distance[target] < 0 {
        return 0; // Unreachable
    }

    let mut len = 0;
    let mut current = target;

    // Build path backwards
    while result.parent[current] >= 0 {
        path[len] = current;
        len += 1;
        current = result.parent[current] as usize;
    }
    path[len] = current; // Add source
    len += 1;

    // Reverse path
    for i in 0..len / 2 {
        path.swap(i, len - 1 - i);
    }

    len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_construction() {
        let edges = [(0, 1), (0, 2), (1, 3), (2, 3)];
        let graph = Graph::from_edges(4, &edges);

        assert_eq!(graph.num_vertices, 4);
        assert_eq!(graph.num_edges, 4);
        assert_eq!(graph.neighbors(0), &[1, 2]);
        assert_eq!(graph.neighbors(1), &[3]);
    }

    #[test]
    fn test_simple_bfs() {
        // Linear graph: 0 -> 1 -> 2 -> 3
        let edges = [(0, 1), (1, 2), (2, 3)];
        let graph = Graph::from_edges(4, &edges);

        let result = bfs(&graph, 0);

        assert_eq!(result.distance[0], 0);
        assert_eq!(result.distance[1], 1);
        assert_eq!(result.distance[2], 2);
        assert_eq!(result.distance[3], 3);
        assert_eq!(result.num_reached, 4);
    }

    #[test]
    fn test_bfs_tree() {
        // Tree:     0
        //          / \
        //         1   2
        //        / \
        //       3   4
        let edges = [(0, 1), (0, 2), (1, 3), (1, 4)];
        let graph = Graph::from_edges(5, &edges);

        let result = bfs(&graph, 0);

        assert_eq!(result.distance[0], 0);
        assert_eq!(result.distance[1], 1);
        assert_eq!(result.distance[2], 1);
        assert_eq!(result.distance[3], 2);
        assert_eq!(result.distance[4], 2);
    }

    #[test]
    fn test_unreachable() {
        // Disconnected: 0 -> 1, 2 -> 3
        let edges = [(0, 1), (2, 3)];
        let graph = Graph::from_edges(4, &edges);

        let result = bfs(&graph, 0);

        assert_eq!(result.distance[0], 0);
        assert_eq!(result.distance[1], 1);
        assert_eq!(result.distance[2], -1); // Unreachable
        assert_eq!(result.distance[3], -1);
        assert_eq!(result.num_reached, 2);
    }

    #[test]
    fn test_path_reconstruction() {
        let edges = [(0, 1), (1, 2), (2, 3)];
        let graph = Graph::from_edges(4, &edges);

        let result = bfs(&graph, 0);

        let mut path = [0usize; MAX_VERTICES];
        let len = reconstruct_path(&result, 3, &mut path);

        assert_eq!(len, 4);
        assert_eq!(&path[..len], &[0, 1, 2, 3]);
    }
}
