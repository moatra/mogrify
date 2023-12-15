use crate::failure::MogrifyFailure;
use crate::path::PathTracker;
use std::any::Any;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct MogrificationError {
    pub(crate) failures: Vec<MogrifyFailure>,
}

impl MogrificationError {
    pub fn new<S: Into<String>>(msg: S) -> MogrificationError {
        MogrificationError {
            failures: vec![MogrifyFailure {
                path: PathTracker::new(),
                message: msg.into(),
                underlying: None,
            }],
        }
    }
    pub fn new_with<S: Into<String>, Err: Error + Send + Sync + 'static>(
        msg: S,
        err: Err,
    ) -> MogrificationError {
        MogrificationError {
            failures: vec![MogrifyFailure {
                path: PathTracker::new(),
                message: msg.into(),
                underlying: Some(Box::new(err)),
            }],
        }
    }
    pub fn wrapping<T: Any + Error + Send + Sync + 'static>(
        mut underlying: T,
    ) -> MogrificationError {
        let as_maybe_mogrify = &mut underlying as &mut dyn Any;
        match as_maybe_mogrify.downcast_mut::<MogrificationError>() {
            None => MogrificationError {
                failures: vec![MogrifyFailure {
                    path: PathTracker::new(),
                    message: "mogrify failure".to_string(),
                    underlying: Some(Box::new(underlying)),
                }],
            },
            Some(underlying) => {
                let mut known = MogrificationError { failures: vec![] };
                std::mem::swap(&mut known.failures, &mut underlying.failures);
                known
            }
        }
    }
    pub fn failures(&self) -> &Vec<MogrifyFailure> {
        &self.failures
    }
    pub fn into_box(self) -> Box<dyn Error + Send + Sync + 'static> {
        Box::new(self)
    }
    pub fn condense(errors: Vec<MogrificationError>) -> Result<(), MogrificationError> {
        if errors.is_empty() {
            Ok(())
        } else {
            Err(MogrificationError {
                failures: errors.into_iter().flat_map(|e| e.failures).collect(),
            })
        }
    }
    // todo: remove once we remove the last usage of ValidationError
    pub fn collect(&mut self, mut other: MogrificationError) {
        self.failures.append(&mut other.failures);
    }
}
impl Display for MogrificationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.failures.len() == 1 {
            let err = self.failures.get(0).unwrap();
            std::fmt::Display::fmt(&err, f)
        } else {
            // todo: test for formatting
            writeln!(f, "found {} mogrify failures", self.failures.len())?;
            for details in &self.failures {
                writeln!(f, "    -> {details}")?
            }
            Ok(())
        }
    }
}

impl Error for MogrificationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if self.failures.len() == 1 {
            let details: &MogrifyFailure = self.failures.get(0).unwrap();
            details.source()
        } else {
            None
        }
    }
}
