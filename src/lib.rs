#![allow(dead_code, unused_imports, unused_variables)]

pub mod cli;
pub mod config;
pub mod error;
pub mod range;
pub mod reader;
pub mod sed_compat;
pub mod whitelist;

pub use error::RipError;
