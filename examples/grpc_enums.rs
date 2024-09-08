#![allow(dead_code)]
use mogrify::Mogrify;

enum RawInput {
    First(()),
    Second(()),
    Third(u32),
}

#[derive(Mogrify)]
#[mogrify(RawInput, grpc)]
enum MogrifiedInput {
    First,
    Second,
    Third(u32),
}

fn main() {
    let raw = RawInput::Second(());

    let _: MogrifiedInput = raw.try_into().expect("successful conversion");
}
