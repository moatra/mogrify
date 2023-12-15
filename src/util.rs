use crate::{MogrificationError, Pathed};
use std::any::Any;
use std::collections::HashMap;
use std::error::Error;
use std::hash::Hash;
use std::str::FromStr;

pub fn capture_error<T, E>(errors: &mut Vec<E>, result: Result<T, E>) -> Option<T> {
    match result {
        Ok(t) => Some(t),
        Err(err) => {
            errors.push(err);
            None
        }
    }
}

pub fn force_parse<From: AsRef<str>, Into, Err>(source: From) -> Result<Into, Err>
where
    Err: Any + Error + Send + Sync + 'static,
    Into: FromStr<Err = Err>,
{
    source.as_ref().parse()
}

pub fn mogrify_raw<From, Into, Err>(from: From) -> Result<Into, MogrificationError>
where
    Err: Any + Error + Send + Sync + 'static,
    Into: TryFrom<From, Error = Err>,
{
    from.try_into().map_err(MogrificationError::wrapping)
}

pub fn mogrify_raw_with<From, Into, Err>(
    with: impl Fn(From) -> Result<Into, Err>,
    value: From,
) -> Result<Into, MogrificationError>
where
    Err: Any + Error + Send + Sync + 'static,
{
    with(value).map_err(MogrificationError::wrapping)
}

pub fn mogrify_require<T>(from: Option<T>) -> Result<T, MogrificationError> {
    match from {
        None => Err(MogrificationError::new("Value is required")),
        Some(value) => Ok(value),
    }
}

pub fn mogrify_opt<From, Into, Err>(from: Option<From>) -> Result<Option<Into>, MogrificationError>
where
    Err: Any + Error + Send + Sync + 'static,
    Into: TryFrom<From, Error = Err>,
{
    from.map(|value| mogrify_raw(value)).transpose()
}

pub fn mogrify_opt_with<From, Into, Err>(
    with: impl Fn(From) -> Result<Into, Err>,
    from: Option<From>,
) -> Result<Option<Into>, MogrificationError>
where
    Err: Any + Error + Send + Sync + 'static,
{
    from.map(|value| mogrify_raw_with(with, value)).transpose()
}

pub fn mogrify_vec<From, Into, Err>(from: Vec<From>) -> Result<Vec<Into>, MogrificationError>
where
    Err: Any + Error + Send + Sync + 'static,
    Into: TryFrom<From, Error = Err>,
{
    mogrify_vec_with(<Into as TryFrom<From>>::try_from, from)
}
pub fn mogrify_vec_with<From, Into, Err>(
    with: impl Fn(From) -> Result<Into, Err>,
    from: Vec<From>,
) -> Result<Vec<Into>, MogrificationError>
where
    Err: Any + Error + Send + Sync + 'static,
{
    let mut errors = Vec::new();
    let mut successes = Vec::with_capacity(from.len());

    for (i, value) in from.into_iter().enumerate() {
        match mogrify_raw_with(&with, value) {
            Ok(into) => successes.push(into),
            Err(err) => errors.push(err.at_index(i)),
        }
    }

    MogrificationError::condense(errors)?;
    Ok(successes)
}

pub fn mogrify_map<KeyFrom, KeyInto, KeyErr, ValueFrom, ValueInto, ValueErr>(
    from: HashMap<KeyFrom, ValueFrom>,
) -> Result<HashMap<KeyInto, ValueInto>, MogrificationError>
where
    KeyErr: Any + Error + Send + Sync + 'static,
    KeyFrom: ToString,
    KeyInto: TryFrom<KeyFrom, Error = KeyErr> + Hash + Eq,
    ValueErr: Any + Error + Send + Sync + 'static,
    ValueInto: TryFrom<ValueFrom, Error = ValueErr>,
{
    mogrify_map_with(<ValueInto as TryFrom<ValueFrom>>::try_from, from)
}

pub fn mogrify_map_with<KeyFrom, KeyInto, KeyErr, ValueFrom, ValueInto, ValueErr>(
    with: impl Fn(ValueFrom) -> Result<ValueInto, ValueErr>,
    from: HashMap<KeyFrom, ValueFrom>,
) -> Result<HashMap<KeyInto, ValueInto>, MogrificationError>
where
    KeyErr: Any + Error + Send + Sync + 'static,
    KeyFrom: ToString,
    KeyInto: TryFrom<KeyFrom, Error = KeyErr> + Hash + Eq,
    ValueErr: Any + Error + Send + Sync + 'static,
{
    let mut errors = Vec::new();
    let mut successes = HashMap::new();

    for (key, value) in from.into_iter() {
        let string_key = key.to_string();
        let into_key = capture_error(&mut errors, mogrify_raw(key).at_key(&string_key));
        let into_value = capture_error(
            &mut errors,
            mogrify_raw_with(&with, value).at_key(&string_key),
        );
        if let (Some(into_key), Some(into_value)) = (into_key, into_value) {
            successes.insert(into_key, into_value);
        }
    }
    MogrificationError::condense(errors)?;
    Ok(successes)
}
