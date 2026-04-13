//! Recursive split tree for terminal pane layout.
//!
//! A split tree organises terminal tabs into a hierarchy of horizontal and
//! vertical splits. Each leaf holds a `tab_id`; internal nodes hold a direction,
//! a ratio (0.0–1.0 position of the divider), and exactly two children.

use serde::{Deserialize, Serialize};

use rustacle_plugin_api::ModuleError;

// ── Types ───────────────────────────────────────────────────────────────────

/// Direction of a split divider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// A node in the split tree: either a leaf (single tab) or a split with two
/// children.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SplitChild {
    Leaf {
        tab_id: String,
    },
    Split {
        id: String,
        direction: SplitDirection,
        /// Position of the divider between 0.0 and 1.0.
        ratio: f64,
        children: Vec<SplitChild>,
    },
}

/// The top-level split tree. When there is only one tab, `root` is a single
/// `Leaf`. When the user splits, it becomes a `Split` node.
pub struct SplitTree {
    root: Option<SplitChild>,
    next_id: u64,
}

// ── SplitTree impl ─────────────────────────────────────────────────────────

impl SplitTree {
    #[must_use]
    pub fn new() -> Self {
        Self {
            root: None,
            next_id: 0,
        }
    }

    /// Insert a lone tab as the root (used when opening the first tab).
    pub fn insert_leaf(&mut self, tab_id: &str) {
        if self.root.is_none() {
            self.root = Some(SplitChild::Leaf {
                tab_id: tab_id.to_string(),
            });
        } else {
            // If root already exists, we don't overwrite — caller should use
            // `split_tab` instead.
            tracing::warn!(tab_id, "insert_leaf called but root already exists");
        }
    }

    /// Split an existing tab, creating a new split node with the original tab
    /// and a new tab side-by-side.
    ///
    /// Returns `(split_node_id, new_tab_id_placeholder)` — the caller must
    /// supply the actual new `tab_id` from `TabManager::open_tab`.
    ///
    /// # Errors
    /// Returns `ModuleError::InvalidInput` if `tab_id` is not found in the
    /// tree.
    pub fn split_tab(
        &mut self,
        tab_id: &str,
        new_tab_id: &str,
        direction: SplitDirection,
    ) -> Result<String, ModuleError> {
        let Some(root) = &mut self.root else {
            return Err(ModuleError::InvalidInput {
                reason: "split tree is empty".to_string(),
            });
        };

        self.next_id += 1;
        let node_id = format!("split-{}", self.next_id);

        if !Self::replace_leaf(root, tab_id, new_tab_id, direction, &node_id) {
            return Err(ModuleError::InvalidInput {
                reason: format!("tab not found in split tree: {tab_id}"),
            });
        }

        Ok(node_id)
    }

    /// Remove a leaf from the tree. If the leaf's parent split has only one
    /// remaining child, that child is promoted to replace the split (collapse).
    ///
    /// # Errors
    /// Returns `ModuleError::InvalidInput` if `tab_id` is not found.
    ///
    /// # Panics
    /// Cannot panic — the `unwrap` is guarded by a prior `is_none` check.
    pub fn close_leaf(&mut self, tab_id: &str) -> Result<(), ModuleError> {
        if self.root.is_none() {
            return Err(ModuleError::InvalidInput {
                reason: "split tree is empty".to_string(),
            });
        }

        // Special case: root is the leaf itself.
        let is_root_leaf = matches!(
            &self.root,
            Some(SplitChild::Leaf { tab_id: id }) if id == tab_id
        );
        if is_root_leaf {
            self.root = None;
            return Ok(());
        }

        // INVARIANT: root is Some — checked above and not the target leaf.
        let root = self.root.as_mut().unwrap();
        if !Self::remove_leaf(root, tab_id) {
            return Err(ModuleError::InvalidInput {
                reason: format!("tab not found in split tree: {tab_id}"),
            });
        }

        Ok(())
    }

    /// Update the ratio of a split node.
    ///
    /// # Errors
    /// Returns `ModuleError::InvalidInput` if the node is not found or the
    /// ratio is out of range.
    pub fn resize_split(&mut self, node_id: &str, ratio: f64) -> Result<(), ModuleError> {
        if !(0.05..=0.95).contains(&ratio) {
            return Err(ModuleError::InvalidInput {
                reason: format!("ratio must be between 0.05 and 0.95, got {ratio}"),
            });
        }

        let Some(root) = &mut self.root else {
            return Err(ModuleError::InvalidInput {
                reason: "split tree is empty".to_string(),
            });
        };

        if !Self::set_ratio(root, node_id, ratio) {
            return Err(ModuleError::InvalidInput {
                reason: format!("split node not found: {node_id}"),
            });
        }

        Ok(())
    }

    /// Return a serializable snapshot of the layout for the frontend.
    #[must_use]
    pub fn to_layout(&self) -> Option<SplitChild> {
        self.root.clone()
    }

    // ── Private helpers ─────────────────────────────────────────────────

    /// Find the leaf with `tab_id` and replace it with a split node containing
    /// the original leaf and a new leaf. Returns `true` if found.
    fn replace_leaf(
        node: &mut SplitChild,
        tab_id: &str,
        new_tab_id: &str,
        direction: SplitDirection,
        split_id: &str,
    ) -> bool {
        match node {
            SplitChild::Leaf { tab_id: id } if id == tab_id => {
                let original = SplitChild::Leaf {
                    tab_id: tab_id.to_string(),
                };
                let new_leaf = SplitChild::Leaf {
                    tab_id: new_tab_id.to_string(),
                };
                *node = SplitChild::Split {
                    id: split_id.to_string(),
                    direction,
                    ratio: 0.5,
                    children: vec![original, new_leaf],
                };
                true
            }
            SplitChild::Split { children, .. } => children
                .iter_mut()
                .any(|c| Self::replace_leaf(c, tab_id, new_tab_id, direction, split_id)),
            SplitChild::Leaf { .. } => false,
        }
    }

    /// Remove a leaf and collapse its parent if only one child remains.
    /// Returns `true` if the leaf was found and removed.
    fn remove_leaf(node: &mut SplitChild, tab_id: &str) -> bool {
        let SplitChild::Split { children, .. } = node else {
            return false;
        };

        // Check if any direct child is the target leaf.
        if let Some(idx) = children
            .iter()
            .position(|c| matches!(c, SplitChild::Leaf { tab_id: id } if id == tab_id))
        {
            children.remove(idx);
            // Collapse: if one child remains, promote it.
            if children.len() == 1 {
                // INVARIANT: we just verified len() == 1
                let remaining = children.remove(0);
                *node = remaining;
            }
            return true;
        }

        // Recurse into child splits.
        children.iter_mut().any(|c| Self::remove_leaf(c, tab_id))
    }

    /// Recursively find a split node by ID and update its ratio.
    fn set_ratio(node: &mut SplitChild, node_id: &str, ratio: f64) -> bool {
        match node {
            SplitChild::Split {
                id,
                ratio: r,
                children,
                ..
            } => {
                if id == node_id {
                    *r = ratio;
                    return true;
                }
                children
                    .iter_mut()
                    .any(|c| Self::set_ratio(c, node_id, ratio))
            }
            SplitChild::Leaf { .. } => false,
        }
    }
}

impl Default for SplitTree {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_split() {
        let mut tree = SplitTree::new();
        tree.insert_leaf("tab-1");

        let node_id = tree
            .split_tab("tab-1", "tab-2", SplitDirection::Horizontal)
            .expect("split should succeed");

        assert!(node_id.starts_with("split-"));

        let layout = tree.to_layout().expect("layout should exist");
        match layout {
            SplitChild::Split {
                direction,
                children,
                ..
            } => {
                assert_eq!(direction, SplitDirection::Horizontal);
                assert_eq!(children.len(), 2);
            }
            _ => panic!("expected split node at root"),
        }
    }

    #[test]
    fn close_leaf_collapses_parent() {
        let mut tree = SplitTree::new();
        tree.insert_leaf("tab-1");
        tree.split_tab("tab-1", "tab-2", SplitDirection::Vertical)
            .unwrap();

        tree.close_leaf("tab-2").unwrap();

        let layout = tree.to_layout().expect("layout should exist");
        match layout {
            SplitChild::Leaf { tab_id } => assert_eq!(tab_id, "tab-1"),
            _ => panic!("expected single leaf after collapse"),
        }
    }

    #[test]
    fn nested_splits() {
        let mut tree = SplitTree::new();
        tree.insert_leaf("tab-1");
        tree.split_tab("tab-1", "tab-2", SplitDirection::Horizontal)
            .unwrap();
        tree.split_tab("tab-2", "tab-3", SplitDirection::Vertical)
            .unwrap();

        // Should have: Split(tab-1, Split(tab-2, tab-3))
        let layout = tree.to_layout().unwrap();
        match layout {
            SplitChild::Split { children, .. } => {
                assert_eq!(children.len(), 2);
                assert!(matches!(&children[1], SplitChild::Split { .. }));
            }
            _ => panic!("expected nested splits"),
        }
    }

    #[test]
    fn resize_split() {
        let mut tree = SplitTree::new();
        tree.insert_leaf("tab-1");
        let node_id = tree
            .split_tab("tab-1", "tab-2", SplitDirection::Horizontal)
            .unwrap();

        tree.resize_split(&node_id, 0.7).unwrap();

        let layout = tree.to_layout().unwrap();
        match layout {
            SplitChild::Split { ratio, .. } => {
                assert!((ratio - 0.7).abs() < f64::EPSILON);
            }
            _ => panic!("expected split"),
        }
    }

    #[test]
    fn resize_out_of_range() {
        let mut tree = SplitTree::new();
        tree.insert_leaf("tab-1");
        let node_id = tree
            .split_tab("tab-1", "tab-2", SplitDirection::Horizontal)
            .unwrap();

        assert!(tree.resize_split(&node_id, 0.0).is_err());
        assert!(tree.resize_split(&node_id, 1.0).is_err());
    }

    #[test]
    fn close_last_leaf_empties_tree() {
        let mut tree = SplitTree::new();
        tree.insert_leaf("tab-1");
        tree.close_leaf("tab-1").unwrap();
        assert!(tree.to_layout().is_none());
    }

    #[test]
    fn split_nonexistent_tab_fails() {
        let mut tree = SplitTree::new();
        tree.insert_leaf("tab-1");
        assert!(
            tree.split_tab("tab-999", "tab-2", SplitDirection::Horizontal)
                .is_err()
        );
    }
}
