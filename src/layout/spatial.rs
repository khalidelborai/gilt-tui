//! Spatial map for efficient hit testing.
//!
//! [`SpatialMap`] maintains a list of node regions ordered by z-order (insertion
//! order) and provides hit-testing queries to find which nodes are at a given
//! point or overlap a given region.

use std::collections::HashMap;

use crate::dom::node::NodeId;
use crate::geometry::{Offset, Region};

/// A spatial map that stores node regions and supports hit-testing queries.
///
/// Internally stores `(NodeId, Region)` pairs ordered by z-order, where later
/// entries are considered "in front" of earlier ones (painter's order). This
/// ordering is derived from depth-first traversal order during layout, which
/// naturally produces the correct visual stacking.
pub struct SpatialMap {
    /// Entries ordered by z-order (last = frontmost).
    entries: Vec<(NodeId, Region)>,
}

impl SpatialMap {
    /// Create an empty spatial map.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Rebuild the spatial map from a set of layout results.
    ///
    /// The iteration order of the `HashMap` is arbitrary, but typically you'd
    /// want to call this with layouts from `LayoutEngine::get_all_layouts()`.
    /// The insertion order becomes the z-order: nodes inserted later are
    /// considered frontmost.
    ///
    /// For deterministic z-ordering (e.g. from a depth-first DOM walk), use
    /// [`update_ordered`].
    pub fn update(&mut self, layouts: &HashMap<NodeId, Region>) {
        self.entries.clear();
        self.entries.reserve(layouts.len());
        for (&node_id, &region) in layouts {
            self.entries.push((node_id, region));
        }
    }

    /// Rebuild the spatial map from an ordered list of `(NodeId, Region)` pairs.
    ///
    /// The order of the slice defines z-order: the last entry is frontmost.
    /// This is useful when you have a deterministic traversal order (e.g. from
    /// a depth-first DOM walk).
    pub fn update_ordered(&mut self, entries: &[(NodeId, Region)]) {
        self.entries.clear();
        self.entries.reserve(entries.len());
        self.entries.extend_from_slice(entries);
    }

    /// Return all nodes whose region contains the given point, ordered
    /// front-to-back (frontmost first).
    ///
    /// The frontmost node is the one inserted last (highest z-order).
    pub fn hit_test(&self, point: Offset) -> Vec<NodeId> {
        let mut result: Vec<NodeId> = self
            .entries
            .iter()
            .filter(|(_, region)| region.contains(point.x, point.y))
            .map(|(id, _)| *id)
            .collect();
        // Reverse so frontmost (last inserted) is first.
        result.reverse();
        result
    }

    /// Return the frontmost node at the given point, or `None` if no node
    /// contains that point.
    ///
    /// This is equivalent to `hit_test(point).first().copied()` but more
    /// efficient since it stops at the first match from the back.
    pub fn node_at(&self, point: Offset) -> Option<NodeId> {
        self.entries
            .iter()
            .rev()
            .find(|(_, region)| region.contains(point.x, point.y))
            .map(|(id, _)| *id)
    }

    /// Return all nodes whose region overlaps the given region.
    ///
    /// Results are in front-to-back order (frontmost first).
    pub fn nodes_in_region(&self, region: &Region) -> Vec<NodeId> {
        let mut result: Vec<NodeId> = self
            .entries
            .iter()
            .filter(|(_, r)| r.overlaps(*region))
            .map(|(id, _)| *id)
            .collect();
        // Reverse so frontmost (last inserted) is first.
        result.reverse();
        result
    }

    /// Number of entries in the spatial map.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the spatial map is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for SpatialMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::node::NodeData;
    use crate::dom::tree::Dom;

    /// Helper to create NodeIds via a Dom (since NodeId comes from slotmap).
    fn make_ids(count: usize) -> (Dom, Vec<NodeId>) {
        let mut dom = Dom::new();
        let ids: Vec<NodeId> = (0..count)
            .map(|i| {
                if i == 0 {
                    dom.insert(NodeData::new(format!("N{i}")))
                } else {
                    // Insert as root-level for simplicity.
                    dom.insert(NodeData::new(format!("N{i}")))
                }
            })
            .collect();
        (dom, ids)
    }

    #[test]
    fn new_is_empty() {
        let map = SpatialMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn default_is_empty() {
        let map = SpatialMap::default();
        assert!(map.is_empty());
    }

    #[test]
    fn update_from_hashmap() {
        let (_dom, ids) = make_ids(2);
        let mut layouts = HashMap::new();
        layouts.insert(ids[0], Region::new(0, 0, 10, 10));
        layouts.insert(ids[1], Region::new(5, 5, 10, 10));

        let mut map = SpatialMap::new();
        map.update(&layouts);
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn update_ordered() {
        let (_dom, ids) = make_ids(3);
        let entries = vec![
            (ids[0], Region::new(0, 0, 20, 20)),
            (ids[1], Region::new(5, 5, 10, 10)),
            (ids[2], Region::new(8, 8, 5, 5)),
        ];

        let mut map = SpatialMap::new();
        map.update_ordered(&entries);
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn hit_test_single_node() {
        let (_dom, ids) = make_ids(1);
        let entries = vec![(ids[0], Region::new(0, 0, 10, 10))];

        let mut map = SpatialMap::new();
        map.update_ordered(&entries);

        let hits = map.hit_test(Offset::new(5, 5));
        assert_eq!(hits, vec![ids[0]]);

        let misses = map.hit_test(Offset::new(10, 10));
        assert!(misses.is_empty());
    }

    #[test]
    fn hit_test_overlapping_nodes_front_to_back() {
        let (_dom, ids) = make_ids(3);
        let entries = vec![
            (ids[0], Region::new(0, 0, 20, 20)),  // back
            (ids[1], Region::new(5, 5, 10, 10)),   // middle
            (ids[2], Region::new(8, 8, 5, 5)),     // front
        ];

        let mut map = SpatialMap::new();
        map.update_ordered(&entries);

        // Point at (9, 9) is inside all three.
        let hits = map.hit_test(Offset::new(9, 9));
        assert_eq!(hits, vec![ids[2], ids[1], ids[0]]); // front-to-back

        // Point at (6, 6) is inside ids[0] and ids[1] but not ids[2].
        let hits2 = map.hit_test(Offset::new(6, 6));
        assert_eq!(hits2, vec![ids[1], ids[0]]);

        // Point at (1, 1) is only inside ids[0].
        let hits3 = map.hit_test(Offset::new(1, 1));
        assert_eq!(hits3, vec![ids[0]]);
    }

    #[test]
    fn node_at_returns_frontmost() {
        let (_dom, ids) = make_ids(2);
        let entries = vec![
            (ids[0], Region::new(0, 0, 20, 20)),
            (ids[1], Region::new(5, 5, 10, 10)),
        ];

        let mut map = SpatialMap::new();
        map.update_ordered(&entries);

        assert_eq!(map.node_at(Offset::new(7, 7)), Some(ids[1]));
        assert_eq!(map.node_at(Offset::new(1, 1)), Some(ids[0]));
        assert_eq!(map.node_at(Offset::new(25, 25)), None);
    }

    #[test]
    fn node_at_no_nodes() {
        let map = SpatialMap::new();
        assert_eq!(map.node_at(Offset::new(0, 0)), None);
    }

    #[test]
    fn nodes_in_region_basic() {
        let (_dom, ids) = make_ids(3);
        let entries = vec![
            (ids[0], Region::new(0, 0, 10, 10)),
            (ids[1], Region::new(20, 20, 10, 10)),
            (ids[2], Region::new(5, 5, 20, 20)),
        ];

        let mut map = SpatialMap::new();
        map.update_ordered(&entries);

        // Region that overlaps ids[0] and ids[2] but not ids[1].
        let query = Region::new(0, 0, 8, 8);
        let result = map.nodes_in_region(&query);
        assert_eq!(result, vec![ids[2], ids[0]]); // front-to-back
    }

    #[test]
    fn nodes_in_region_no_overlap() {
        let (_dom, ids) = make_ids(1);
        let entries = vec![(ids[0], Region::new(0, 0, 10, 10))];

        let mut map = SpatialMap::new();
        map.update_ordered(&entries);

        let query = Region::new(100, 100, 5, 5);
        let result = map.nodes_in_region(&query);
        assert!(result.is_empty());
    }

    #[test]
    fn nodes_in_region_all_overlap() {
        let (_dom, ids) = make_ids(2);
        let entries = vec![
            (ids[0], Region::new(0, 0, 50, 50)),
            (ids[1], Region::new(10, 10, 30, 30)),
        ];

        let mut map = SpatialMap::new();
        map.update_ordered(&entries);

        let query = Region::new(15, 15, 5, 5);
        let result = map.nodes_in_region(&query);
        assert_eq!(result, vec![ids[1], ids[0]]);
    }

    #[test]
    fn update_replaces_previous() {
        let (_dom, ids) = make_ids(2);
        let entries1 = vec![
            (ids[0], Region::new(0, 0, 10, 10)),
            (ids[1], Region::new(20, 20, 10, 10)),
        ];

        let mut map = SpatialMap::new();
        map.update_ordered(&entries1);
        assert_eq!(map.len(), 2);

        // Update with only one entry.
        let entries2 = vec![(ids[0], Region::new(0, 0, 5, 5))];
        map.update_ordered(&entries2);
        assert_eq!(map.len(), 1);

        // ids[1] should no longer be findable.
        assert_eq!(map.node_at(Offset::new(22, 22)), None);
    }

    #[test]
    fn hit_test_edge_cases() {
        let (_dom, ids) = make_ids(1);
        let entries = vec![(ids[0], Region::new(5, 5, 10, 10))];

        let mut map = SpatialMap::new();
        map.update_ordered(&entries);

        // Top-left corner (inclusive).
        assert_eq!(map.hit_test(Offset::new(5, 5)), vec![ids[0]]);
        // Bottom-right corner (exclusive: 5+10=15, so 14 is last inclusive).
        assert_eq!(map.hit_test(Offset::new(14, 14)), vec![ids[0]]);
        // Just outside.
        assert!(map.hit_test(Offset::new(15, 14)).is_empty());
        assert!(map.hit_test(Offset::new(14, 15)).is_empty());
        assert!(map.hit_test(Offset::new(4, 5)).is_empty());
        assert!(map.hit_test(Offset::new(5, 4)).is_empty());
    }

    #[test]
    fn zero_size_region_not_hittable() {
        let (_dom, ids) = make_ids(1);
        let entries = vec![(ids[0], Region::new(5, 5, 0, 0))];

        let mut map = SpatialMap::new();
        map.update_ordered(&entries);

        assert!(map.hit_test(Offset::new(5, 5)).is_empty());
        assert_eq!(map.node_at(Offset::new(5, 5)), None);
    }
}
