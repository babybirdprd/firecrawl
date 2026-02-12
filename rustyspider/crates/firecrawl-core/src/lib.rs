#![deny(clippy::all)]

pub use crate::crawler::*;
pub use crate::engpicker::*;
pub use crate::html::*;
pub use crate::pdf::*;
pub use crate::scraper::*;
pub use crate::queue::*;

pub use crate::document::{DocumentConverter, DocumentType};

pub mod crawler;
mod document;
mod engpicker;
mod html;
mod pdf;
pub mod scraper;
mod utils;
pub mod queue;
pub mod crawl;
pub mod webhook;
pub mod storage;
pub mod worker;

pub use serde::{Deserialize, Serialize};
