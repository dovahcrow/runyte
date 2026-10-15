// SPDX-License-Identifier: MPL-2.0

//! Pane-owned selection and viewport positions for buffer visits.

use crate::{jumplist::SelectionSemantics, selection::Selection, text::Transaction};

#[derive(Clone, Debug)]
pub(super) struct ViewPosition<S = Selection> {
    pub selection: S,
    pub scroll_row: usize,
    pub scroll_wrap: usize,
    pub scroll_col: usize,
    pub row_prefix_scroll: usize,
    pub semantics: SelectionSemantics,
    pub preserve_scroll: bool,
}
impl<S> ViewPosition<S> {
    pub(super) fn capture_with(pane: &super::Pane, selection: S) -> Self {
        Self {
            selection,
            scroll_row: pane.scroll_row,
            scroll_wrap: pane.scroll_wrap,
            scroll_col: pane.scroll_col,
            row_prefix_scroll: pane.row_prefix_scroll,
            semantics: pane.selection_semantics(),
            preserve_scroll: true,
        }
    }
    pub(super) fn restore_with(&self, pane: &mut super::Pane, selection: Selection) {
        pane.replace_selection(selection);
        pane.mark_selection_semantics(self.semantics);
        pane.scroll_row = self.scroll_row;
        pane.scroll_wrap = self.scroll_wrap;
        pane.scroll_col = self.scroll_col;
        pane.row_prefix_scroll = self.row_prefix_scroll;
        pane.preserve_scroll = self.preserve_scroll;
    }
}
impl ViewPosition {
    pub(super) fn capture(pane: &super::Pane) -> Self {
        Self::capture_with(pane, pane.selection.clone())
    }
    pub(super) fn map(&mut self, transaction: &Transaction) {
        self.selection = self.selection.map(transaction);
        self.preserve_scroll = false;
    }

    pub(super) fn restore(&self, pane: &mut super::Pane) {
        self.restore_with(pane, self.selection.clone());
    }
}
