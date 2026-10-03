//! Layout engine for architecture diagrams: compound graph + hints in, layout out.
pub mod check;
pub mod engine;
pub mod geom;
pub mod groups;
pub mod hier;
pub mod js;
pub mod lanes;
pub mod layout;
pub mod model;
pub mod normalize;
pub mod place;
pub mod ports;
pub mod repair;
pub mod route;
pub mod skeleton;
pub mod tracks;

pub use engine::{layout, Options};
pub use model::{Hints, InputGraph, LayoutResult};
