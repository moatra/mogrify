use crate::MogrificationError;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub(crate) enum PathPart {
    Field(String),
    Index(usize),
    Key(String),
}

#[derive(Debug)]
pub(crate) struct PathTracker {
    pub(crate) parts: Vec<PathPart>,
}
impl PathTracker {
    pub(crate) fn new() -> PathTracker {
        PathTracker {
            parts: Vec::with_capacity(3), // seems reasonable nesting level
        }
    }
}

impl Display for PathTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // validation happens at the leaf nodes, then moves up, so later fields are actually earlier paths (hence the .rev())
        for part in self.parts.iter().rev() {
            match &part {
                PathPart::Field(field) => write!(f, ".{field}")?,
                PathPart::Index(i) => write!(f, "[{i}]")?,
                PathPart::Key(k) => write!(f, "[\"{k}\"]")?,
            }
        }
        Ok(())
    }
}

pub trait Pathed {
    fn at_field(self, field_name: &str) -> Self;
    fn at_index(self, index: usize) -> Self;
    fn at_key(self, key_name: &str) -> Self;
}

impl Pathed for &mut MogrificationError {
    fn at_field(self, field_name: &str) -> Self {
        for err in self.failures.iter_mut() {
            err.path.parts.push(PathPart::Field(field_name.to_string()));
        }
        self
    }

    fn at_index(self, index: usize) -> Self {
        for err in self.failures.iter_mut() {
            err.path.parts.push(PathPart::Index(index));
        }
        self
    }

    fn at_key(self, key_name: &str) -> Self {
        for err in self.failures.iter_mut() {
            err.path.parts.push(PathPart::Key(key_name.to_string()));
        }
        self
    }
}
impl Pathed for MogrificationError {
    fn at_field(mut self, field_name: &str) -> Self {
        (&mut self).at_field(field_name);
        self
    }

    fn at_index(mut self, index: usize) -> Self {
        (&mut self).at_index(index);
        self
    }

    fn at_key(mut self, key_name: &str) -> Self {
        (&mut self).at_key(key_name);
        self
    }
}
impl<T> Pathed for Result<T, MogrificationError> {
    fn at_field(self, field_name: &str) -> Self {
        match self {
            Err(err) => Err(err.at_field(field_name)),
            _ => self,
        }
    }

    fn at_index(self, index: usize) -> Self {
        match self {
            Err(err) => Err(err.at_index(index)),
            _ => self,
        }
    }

    fn at_key(self, key_name: &str) -> Self {
        match self {
            Err(err) => Err(err.at_key(key_name)),
            _ => self,
        }
    }
}
