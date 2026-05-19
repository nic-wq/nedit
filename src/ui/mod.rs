mod colors;
pub mod icons;
mod layout;
mod render;
mod welcome;

// We re-export core UI functions to provide a simplified API for the rest of the app,
// hiding the complexity of layout calculations and specific component rendering.
pub use colors::{get_colors, UIColors};
pub use layout::centered_rect;
pub use render::render;
