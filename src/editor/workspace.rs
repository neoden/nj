//! Every open buffer and every view onto one, with one view in focus.
//!
//! A buffer owns the text, the history and where it came from; a view owns
//! the selections, the scroll position and the area of the screen it was
//! given. A handle is a number that is never handed out twice, so a buffer
//! that is gone cannot be mistaken for the one that took its place.

use std::collections::HashMap;

use super::view::View;
use crate::buffer::Buffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferId(u64);

impl BufferId {
    /// For a test that needs a view without a workspace around it.
    #[cfg(test)]
    pub(crate) const FIRST: BufferId = BufferId(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewId(u64);

#[derive(Debug)]
pub struct Workspace {
    buffers: HashMap<BufferId, Buffer>,
    views: HashMap<ViewId, View>,
    focused: ViewId,
    /// The next handle, for buffers and views alike.
    next: u64,
}

impl Workspace {
    /// One buffer, one view onto it, in focus.
    pub fn single(buffer: Buffer) -> Self {
        let mut workspace = Self {
            buffers: HashMap::new(),
            views: HashMap::new(),
            focused: ViewId(0),
            next: 0,
        };
        let buffer = workspace.open_buffer(buffer);
        workspace.focused = workspace.open_view(buffer);
        workspace
    }

    fn fresh(&mut self) -> u64 {
        self.next += 1;
        self.next - 1
    }

    pub fn open_buffer(&mut self, buffer: Buffer) -> BufferId {
        let id = BufferId(self.fresh());
        self.buffers.insert(id, buffer);
        id
    }

    /// Another window onto a buffer, with its own cursors.
    pub fn open_view(&mut self, buffer: BufferId) -> ViewId {
        let id = ViewId(self.fresh());
        self.views
            .insert(id, View::new(buffer, &self.buffers[&buffer]));
        id
    }

    /// Puts another buffer where the view in focus was. The new view starts
    /// over — cursors, scrolling — but takes the old one's place on the
    /// screen. The old view is gone, and so is its buffer unless another
    /// view still shows it.
    pub fn replace(&mut self, buffer: Buffer) -> BufferId {
        let (old_view, old_buffer) = (self.focused, self.focused_buffer());
        let area = self.views[&old_view].area;

        let id = self.open_buffer(buffer);
        self.focused = self.open_view(id);
        let (view, buffer) = self.focus_mut();
        view.resize(area, buffer);

        self.views.remove(&old_view);
        if !self.views.values().any(|view| view.buffer == old_buffer) {
            self.buffers.remove(&old_buffer);
        }
        id
    }

    pub fn focused(&self) -> ViewId {
        self.focused
    }

    pub fn focused_buffer(&self) -> BufferId {
        self.views[&self.focused].buffer
    }

    /// Moves the focus, and brings the view up to date with whatever was
    /// done to its buffer through other views while it was away.
    pub fn focus_on(&mut self, id: ViewId) {
        self.focused = id;
        let (view, buffer) = self.focus_mut();
        view.catch_up(buffer);
    }

    pub fn buffer(&self, id: BufferId) -> &Buffer {
        &self.buffers[&id]
    }

    pub fn buffer_mut(&mut self, id: BufferId) -> &mut Buffer {
        self.buffers.get_mut(&id).expect("no such buffer")
    }

    pub fn view(&self, id: ViewId) -> &View {
        &self.views[&id]
    }

    pub fn view_mut(&mut self, id: ViewId) -> &mut View {
        self.views.get_mut(&id).expect("no such view")
    }

    /// The view in focus and the buffer it shows, which is what nearly every
    /// command works on. Split three ways by what needs changing, so that a
    /// command reading the text while moving the cursors borrows no more than
    /// it must.
    pub fn focus(&self) -> (&View, &Buffer) {
        let view = &self.views[&self.focused];
        (view, &self.buffers[&view.buffer])
    }

    pub fn focus_mut(&mut self) -> (&mut View, &Buffer) {
        let view = self.views.get_mut(&self.focused).expect("no such view");
        let buffer = view.buffer;
        (view, &self.buffers[&buffer])
    }

    pub fn focus_edit(&mut self) -> (&mut View, &mut Buffer) {
        let view = self.views.get_mut(&self.focused).expect("no such view");
        let buffer = self.buffers.get_mut(&view.buffer).expect("no such buffer");
        (view, buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::EditKind;
    use crate::text::Text;
    use crate::text::selection::SelectionSet;
    use crate::text::transaction::{CursorAfter, Transaction};

    /// Two views on one buffer keep their own cursors, and the one
    /// not in focus is carried along through the edits it did not see.
    #[test]
    fn a_view_left_behind_catches_up_when_it_is_focused_again() {
        let mut workspace = Workspace::single(Buffer::from_text(Text::from_str("abc def")));
        let first = workspace.focused();
        let second = workspace.open_view(workspace.focused_buffer());

        workspace.focus_on(second);
        workspace.focus_mut().0.put(SelectionSet::single(4));

        // Back in the first view, insert at the very start.
        workspace.focus_on(first);
        let (view, buffer) = workspace.focus_edit();
        let transaction =
            Transaction::edit(&SelectionSet::single(0), CursorAfter::EndOfChange, |_| {
                (0..0, "___".to_owned())
            });
        let placed = buffer.apply(transaction, EditKind::Other);
        view.put(placed);
        view.mark_current(buffer);
        assert_eq!(view.selections().primary().head, 3);

        // The second view still says 4 — until it is looked at.
        assert_eq!(workspace.view(second).selections().primary().head, 4);
        workspace.focus_on(second);
        assert_eq!(workspace.view(second).selections().primary().head, 7);
        assert_eq!(workspace.view(first).selections().primary().head, 3);
    }

    /// A buffer that took another's place has a handle of its own: nothing
    /// that remembered the old one can take the new one for it.
    #[test]
    fn a_replaced_buffer_is_gone_and_the_new_one_has_a_new_handle() {
        let mut workspace = Workspace::single(Buffer::from_text(Text::from_str("old")));
        let (old_view, old_buffer) = (workspace.focused(), workspace.focused_buffer());

        let new_buffer = workspace.replace(Buffer::from_text(Text::from_str("new")));

        assert_ne!(new_buffer, old_buffer);
        assert_ne!(workspace.focused(), old_view);
        assert_eq!(workspace.focused_buffer(), new_buffer);
        assert!(!workspace.buffers.contains_key(&old_buffer));
        assert!(!workspace.views.contains_key(&old_view));
        assert_eq!(workspace.focus().1.text().to_string(), "new");
    }
}
