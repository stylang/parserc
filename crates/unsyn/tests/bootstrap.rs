use std::{fs, path::PathBuf};

use parserc::{Input, syntax::Syntax};
use unsyn::{
    input::TokenStream,
    syntax::{Crate, Item},
};
use walkdir::WalkDir;

#[test]
fn bootstrap() {
    let root_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("unsyn")
        .canonicalize()
        .unwrap();

    for entry in WalkDir::new(root_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        println!("{:?}", entry.path());

        let content = fs::read_to_string(entry.path()).unwrap();

        let mut token_stream = TokenStream::from(content.as_str());

        let c = Crate::parse(&mut token_stream).expect(&format!("parse {:?}", entry.path()));

        assert_eq!(token_stream.len(), 0);

        for item in c.items.iter() {
            let Item::Use(syn, _) = item else {
                continue;
            };

            println!("{:?}", syn.to_span());
        }
    }
}
