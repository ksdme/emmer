mod item;
pub mod stack;

pub use item::Item;
pub use stack::Stack;

use crate::ui::renderables::Rect;

/// Represents an object that can be hit-tested against the rect that the layout
/// placed it in.
pub trait HitTarget {
    /// The rect the object occupies in the current layout, if it is interactable.
    fn target(&self) -> Option<Rect>;

    /// Returns a boolean indicating if (x, y) is on or within the target.
    fn hit(&self, at: (f64, f64)) -> bool {
        self.target().is_some_and(|target| target.contains(at))
    }
}
