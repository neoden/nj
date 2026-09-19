//! Every open buffer and every view onto one, with one view in focus.
//!
//! A buffer owns the text, the history and where it came from; a view owns
//! the selections, the scroll position and the area of the screen it was
//! given (ADR 15). Nothing here is ever removed yet, so a plain index is a
//! stable handle.

use crate::buffer::Buffer;
use crate::view::View;

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
        Self {
            buffers: vec![buffer],
            views: vec![View::new(BufferId(0))],
            focused: ViewId(0),
        }
    }

    pub fn focused(&self) -> ViewId {
        self.focused
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
