#![allow(dead_code)]
use mogrify::{MogrificationError, Mogrify};
use std::collections::HashMap;

struct RawInput {
    name: String,
    maybe_foo: Option<String>,
    missing_foo: Option<String>,
    items: Vec<String>,
    map: HashMap<String, String>,
}

#[derive(Eq, PartialEq, Hash)]
struct StringWrap(String);
impl TryFrom<String> for StringWrap {
    type Error = MogrificationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if &value == "error" {
            Err(MogrificationError::new("invalid string"))
        } else {
            Ok(StringWrap(value))
        }
    }
}

#[derive(Mogrify)]
#[mogrify(RawInput)]
struct MogrifiedInput {
    name: StringWrap,
    #[mogrify(require)]
    maybe_foo: StringWrap,
    missing_foo: Option<StringWrap>,
    items: Vec<StringWrap>,
    map: HashMap<StringWrap, StringWrap>,
}

fn main() {
    let raw = RawInput {
        name: "".to_string(),
        maybe_foo: None,
        missing_foo: None,
        items: vec![],
        map: HashMap::new(),
    };

    let _: Result<MogrifiedInput, MogrificationError> = raw.try_into();
}
