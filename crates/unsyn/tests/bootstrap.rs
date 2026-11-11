use std::{fs, path::PathBuf};

use parserc::{Input, syntax::Syntax};
use unsyn::{input::TokenStream, syntax::Crate};

#[test]
fn bootstrap() {
    let filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap.uns")
        .canonicalize()
        .unwrap();

    let content = fs::read_to_string(&filepath).unwrap();

    let mut token_stream = TokenStream::from(content.as_str());

    let c = Crate::parse(&mut token_stream).expect(&format!("parse {:?}", filepath));

    assert_eq!(token_stream.len(), 0);

    for item in c.items.iter() {
        println!("{:?}", item.to_span());
    }
}
