//! Every open buffer and every view onto one, with one view in focus.
//!
//! A buffer owns the text, the history and where it came from; a view owns
//! the selections, the scroll position and the area of the screen it was
//! given (ADR 15). Nothing here is ever removed yet, so a plain index is a
//! stable handle.

use super::view::View;
use crate::buffer::Buffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferId(usize);

impl BufferId {
    /// For a test that needs a view without a workspace around it.
    #[cfg(test)]
    pub(crate) const FIRST: BufferId = BufferId(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewId(usize);

#[derive(Debug)]
pub struct Workspace {
    buffers: Vec<Buffer>,
    views: Vec<View>,
    focused: ViewId,
}

impl Workspace {
    /// One buffer, one view onto it, in focus.
    pub fn single(buffer: Buffer) -> Self {
        let view = View::new(BufferId(0), &buffer);
        Self {
            buffers: vec![buffer],
            views: vec![view],
            focused: ViewId(0),
        }
    }

    /// Puts another buffer behind the view in focus, in place of the one it
    /// showed. The view starts over — cursors, scrolling — but keeps its
    /// place on the screen. The buffer replaced is gone.
    pub fn replace(&mut self, buffer: Buffer) -> BufferId {
        let id = self.focused_buffer();
        let area = self.views[self.focused.0].area;
        let mut view = View::new(id, &buffer);
        view.resize(area, buffer.text().line_count());
        self.buffers[id.0] = buffer;
        self.views[self.focused.0] = view;
        id
    }

    /// Another window onto a buffer, with its own cursors (ADR 9).
    pub fn open_view(&mut self, buffer: BufferId) -> ViewId {
        self.views.push(View::new(buffer, &self.buffers[buffer.0]));
        ViewId(self.views.len() - 1)
    }

    pub fn focused(&self) -> ViewId {
        self.focused
    }

    pub fn focused_buffer(&self) -> BufferId {
        self.views[self.focused.0].buffer
    }

    /// Moves the focus, and brings the view up to date with whatever was
    /// done to its buffer through other views while it was away.
    pub fn focus_on(&mut self, id: ViewId) {
        self.focused = id;
        let (view, buffer) = self.focus_mut();
        view.catch_up(buffer);
    }

    pub fn buffer(&self, id: BufferId) -> &Buffer {
        &self.buffers[id.0]
    }

    pub fn buffer_mut(&mut self, id: BufferId) -> &mut Buffer {
        &mut self.buffers[id.0]
    }

    pub fn view(&self, id: ViewId) -> &View {
        &self.views[id.0]
    }

    pub fn view_mut(&mut self, id: ViewId) -> &mut View {
        &mut self.views[id.0]
    }

    /// The view in focus and the buffer it shows, which is what nearly every
    /// command works on. Split three ways by what needs changing, so that a
    /// command reading the text while moving the cursors borrows no more than
    /// it must.
    pub fn focus(&self) -> (&View, &Buffer) {
        let view = &self.views[self.focused.0];
        (view, &self.buffers[view.buffer.0])
    }

    pub fn focus_mut(&mut self) -> (&mut View, &Buffer) {
        let view = &mut self.views[self.focused.0];
        let buffer = view.buffer;
        (view, &self.buffers[buffer.0])
    }

    pub fn focus_edit(&mut self) -> (&mut View, &mut Buffer) {
        let view = &mut self.views[self.focused.0];
        let buffer = view.buffer;
        (view, &mut self.buffers[buffer.0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::EditKind;
    use crate::text::Text;
    use crate::text::selection::SelectionSet;
    use crate::text::transaction::{CursorAfter, Transaction};

    /// Two views on one buffer keep their own cursors (ADR 9), and the one
    /// not in focus is carried along through the edits it did not see.
    #[test]
    fn a_view_left_behind_catches_up_when_it_is_focused_again() {
        let mut workspace = Workspace::single(Buffer::from_text(Text::from_str("abc def")));
        let first = workspace.focused();
        let second = workspace.open_view(BufferId(0));

        workspace.focus_on(second);
        workspace.focus_mut().0.selections = SelectionSet::single(4);

        // Back in the first view, insert at the very start.
        workspace.focus_on(first);
        let (view, buffer) = workspace.focus_edit();
        let transaction =
            Transaction::edit(&SelectionSet::single(0), CursorAfter::EndOfChange, |_| {
                (0..0, "___".to_owned())
            });
        view.selections = buffer.apply(transaction, EditKind::Other);
        view.mark_current(buffer);
        assert_eq!(view.selections.primary().head, 3);

        // The second view still says 4 — until it is looked at.
        assert_eq!(workspace.view(second).selections.primary().head, 4);
        workspace.focus_on(second);
        assert_eq!(workspace.view(second).selections.primary().head, 7);
        assert_eq!(workspace.view(first).selections.primary().head, 3);
    }
}
