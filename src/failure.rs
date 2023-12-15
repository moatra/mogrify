use crate::path::PathTracker;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct MogrifyFailure {
    pub(crate) path: PathTracker,
    pub(crate) message: String,
    pub(crate) underlying: Option<Box<dyn Error + Send + Sync + 'static>>, // todo: Code?  Link?
}
impl MogrifyFailure {
    pub fn path(&self) -> String {
        self.path.to_string()
    }
    pub fn message(&self) -> &str {
        self.message.as_str()
    }
}

impl Display for MogrifyFailure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.path.parts.is_empty() {
            write!(f, "{}", &self.message)
        } else {
            write!(f, "{} (at: {})", &self.message, &self.path)
        }
    }
}

impl Error for MogrifyFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.underlying
            .as_ref()
            .map(|b| b.as_ref() as &(dyn Error + 'static))
    }
}
