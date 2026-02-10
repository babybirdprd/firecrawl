#![deny(clippy::all)]

pub use crate::crawler::*;
pub use crate::engpicker::*;
pub use crate::html::*;
pub use crate::pdf::*;

pub use crate::document::{DocumentConverter, DocumentType};

mod crawler;
mod document;
mod engpicker;
mod html;
mod pdf;
mod utils;

pub use serde::{Deserialize, Serialize};
