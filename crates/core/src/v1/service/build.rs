use std::{borrow::Cow, path::{Path, PathBuf}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceBuild<'a> {
    Rust(RustBuild<'a>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustBuild<'a> {
    pub path: Option<Cow<'a, PathBuf>>,
    pub target: Option<Cow<'a, str>>,
}
