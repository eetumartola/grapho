use grapho_core::Graph;

use crate::node_graph::{NodeGraphLayout, NodeGraphState};

#[derive(Clone)]
pub(super) struct UndoSnapshot {
    pub(super) graph: Graph,
    pub(super) layout: NodeGraphLayout,
}

pub(super) struct UndoStack {
    past: Vec<UndoSnapshot>,
    future: Vec<UndoSnapshot>,
}

impl UndoStack {
    pub(super) fn new() -> Self {
        Self {
            past: Vec::new(),
            future: Vec::new(),
        }
    }

    pub(super) fn clear(&mut self) {
        self.past.clear();
        self.future.clear();
    }

    pub(super) fn snapshot(&self, graph: &Graph, node_graph: &NodeGraphState) -> UndoSnapshot {
        UndoSnapshot {
            graph: graph.clone(),
            layout: node_graph.layout_snapshot(),
        }
    }

    pub(super) fn push(&mut self, snapshot: UndoSnapshot) {
        self.past.push(snapshot);
        self.future.clear();
    }

    pub(super) fn undo(&mut self, current: UndoSnapshot) -> Option<UndoSnapshot> {
        let prev = self.past.pop()?;
        self.future.push(current);
        Some(prev)
    }

    pub(super) fn redo(&mut self, current: UndoSnapshot) -> Option<UndoSnapshot> {
        let next = self.future.pop()?;
        self.past.push(current);
        Some(next)
    }
}
