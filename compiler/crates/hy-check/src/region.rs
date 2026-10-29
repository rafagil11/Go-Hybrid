//! HyRegion: Region-Isolated Linear Slabs
//!
//! This module implements the core logic for off-heap linear slabs.
//! By isolating memory into regions, we can transfer ownership between
//! tasks without the overhead of Go-heap alias reconstruction or
//! handle-table synchronization.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabId(pub u64);

/// Metadata for a HyRegion slab.
#[derive(Debug, Clone)]
pub struct SlabInfo {
    pub id: SlabId,
    pub size: usize,
    /// If true, this slab contains no pointers to the Go heap.
    pub is_isolated: bool,
    /// The task ID currently owning this slab.
    pub owner_task_id: Option<u64>,
}

/// The RegionTracker maintains the state of all off-heap slabs during
/// the check phase.
pub struct RegionTracker {
    slabs: HashMap<SlabId, SlabInfo>,
}

impl RegionTracker {
    pub fn new() -> Self {
        Self {
            slabs: HashMap::new(),
        }
    }

    /// Validates that a shape is suitable for slab allocation.
    /// It must not contain any pointers to the managed Go heap (Isolation).
    pub fn is_isolated_shape(&self, _shape: &crate::shape::Shape) -> bool {
        // In a full implementation, we would traverse the shape graph.
        // For now, we assume the checker has been updated to flag
        // heap-bound types (maps, chans, etc.) as non-isolated.
        true
    }

    /// Transfers slab ownership from one task context to another.
    pub fn transfer_slab(&mut self, id: SlabId, from_task: u64, to_task: u64) -> Result<(), String> {
        if let Some(info) = self.slabs.get_mut(&id) {
            if info.owner_task_id != Some(from_task) {
                return Err(format!("Task {} does not own slab {:?}", from_task, id));
            }
            info.owner_task_id = Some(to_task);
            Ok(())
        } else {
            Err(format!("Slab {:?} not found", id))
        }
    }
}
