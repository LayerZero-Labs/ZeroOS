//! Graph BFS Example
//!
//! Demonstrates frontier-sliced parallel BFS.

#![cfg_attr(target_os = "none", no_std)]
#![no_main]

use graph_bfs::{batch_bfs, bfs, multi_source_bfs, reconstruct_path, BfsResult, Graph, MAX_VERTICES};

cfg_if::cfg_if! {
    if #[cfg(target_os = "none")] {
        use platform::println;
    } else {
        use std::println;
    }
}

#[unsafe(no_mangle)]
fn main() -> ! {
    println!("=== Graph BFS Example ===");

    // Test 1: Simple linear graph
    println!("\nTest 1: Linear Graph (0->1->2->3->4)");
    let linear_edges = [(0, 1), (1, 2), (2, 3), (3, 4)];
    let linear_graph = Graph::from_edges(5, &linear_edges);

    let result = bfs(&linear_graph, 0);
    println!("  Source: 0");
    for i in 0..5 {
        println!("  Distance to {}: {}", i, result.distance[i]);
    }
    println!("  Vertices reached: {}", result.num_reached);

    // Test 2: Binary tree graph
    println!("\nTest 2: Binary Tree");
    //       0
    //      / \
    //     1   2
    //    / \ / \
    //   3  4 5  6
    let tree_edges = [
        (0, 1), (0, 2),
        (1, 3), (1, 4),
        (2, 5), (2, 6),
    ];
    let tree_graph = Graph::from_edges(7, &tree_edges);

    let tree_result = bfs(&tree_graph, 0);
    println!("  Level 0 (root): vertex 0, dist={}", tree_result.distance[0]);
    println!("  Level 1: vertices 1,2, dist={},{}", tree_result.distance[1], tree_result.distance[2]);
    println!("  Level 2: vertices 3-6, dist={},{},{},{}",
             tree_result.distance[3], tree_result.distance[4],
             tree_result.distance[5], tree_result.distance[6]);

    // Test 3: Path reconstruction
    println!("\nTest 3: Path Reconstruction");
    let mut path = [0usize; MAX_VERTICES];
    let path_len = reconstruct_path(&tree_result, 6, &mut path);
    // Build path string since we don't have print! without newline
    println!("  Path from 0 to 6: {} -> {} -> {}", path[0], path[1], path[2]);

    // Test 4: Disconnected graph
    println!("\nTest 4: Disconnected Graph");
    // Two components: 0-1-2 and 3-4-5
    let disconnected_edges = [
        (0, 1), (1, 2),
        (3, 4), (4, 5),
    ];
    let disconnected_graph = Graph::from_edges(6, &disconnected_edges);

    let disc_result = bfs(&disconnected_graph, 0);
    println!("  From source 0:");
    println!("    Reachable: {} vertices", disc_result.num_reached);
    println!("    Distance to 2: {}", disc_result.distance[2]);
    println!("    Distance to 5: {} (unreachable)", disc_result.distance[5]);

    // Test 5: Multi-source BFS
    println!("\nTest 5: Multi-Source BFS");
    let sources = [0, 3];
    let multi_result = multi_source_bfs(&disconnected_graph, &sources);
    println!("  Sources: 0 and 3");
    println!("  All vertices reached: {}", multi_result.num_reached);
    for i in 0..6 {
        println!("  Distance to {}: {}", i, multi_result.distance[i]);
    }

    // Test 6: Dense graph (complete graph K5)
    println!("\nTest 6: Complete Graph K5");
    let mut complete_edges = [(0usize, 0usize); 20];
    let mut idx = 0;
    for i in 0..5 {
        for j in 0..5 {
            if i != j {
                complete_edges[idx] = (i, j);
                idx += 1;
            }
        }
    }
    let complete_graph = Graph::from_edges(5, &complete_edges);

    let complete_result = bfs(&complete_graph, 0);
    println!("  From any vertex, all others at distance 1:");
    let all_dist_one = (1..5).all(|i| complete_result.distance[i] == 1);
    println!("  Verified: {}", if all_dist_one { "PASS" } else { "FAIL" });

    // Test 7: Batch BFS (parallel-friendly)
    println!("\nTest 7: Batch BFS (independent searches)");
    let cycle_edges = [(0, 1), (1, 2), (2, 3), (3, 4), (4, 0)]; // 5-cycle
    let cycle_graph = Graph::from_edges(5, &cycle_edges);

    let batch_sources = [0, 1, 2];
    let mut batch_results = [BfsResult::new(), BfsResult::new(), BfsResult::new()];

    batch_bfs(&cycle_graph, &batch_sources, &mut batch_results);

    for (i, result) in batch_results.iter().enumerate() {
        println!("  BFS from {}: max_dist={}",
                 batch_sources[i],
                 result.distance.iter().take(5).max().unwrap_or(&0));
    }

    // Test 8: Larger graph for performance
    println!("\nTest 8: Grid Graph 8x8");
    // Create 8x8 grid graph (4-connected)
    let mut grid_edges = [(0usize, 0usize); 256];
    let mut edge_count = 0;
    for row in 0..8 {
        for col in 0..8 {
            let v = row * 8 + col;
            // Right neighbor
            if col < 7 {
                grid_edges[edge_count] = (v, v + 1);
                edge_count += 1;
            }
            // Down neighbor
            if row < 7 {
                grid_edges[edge_count] = (v, v + 8);
                edge_count += 1;
            }
        }
    }
    let grid_graph = Graph::from_edges(64, &grid_edges[..edge_count]);

    let grid_result = bfs(&grid_graph, 0);
    println!("  Source: (0,0), Target: (7,7)");
    println!("  Distance: {} (Manhattan distance)", grid_result.distance[63]);
    println!("  Expected: 14 (7 right + 7 down)");

    // Reconstruct path
    let mut grid_path = [0usize; MAX_VERTICES];
    let grid_path_len = reconstruct_path(&grid_result, 63, &mut grid_path);
    println!("  Path length: {} vertices", grid_path_len);

    println!("\n=== Graph BFS Example Complete ===");

    platform::exit(0)
}
