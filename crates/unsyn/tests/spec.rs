use std::{fs, path::PathBuf};

use unsyn::input::Chars;
use walkdir::WalkDir;

pub fn run_spec<F>(name: &str, f: F)
where
    F: Fn(&mut Chars<'_>) -> serde_json::Value,
{
    let spec_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("spec")
        .join(name)
        .canonicalize()
        .unwrap();

    println!("run spec {:?}", spec_dir);

    for synfile in WalkDir::new(spec_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension() // 获取扩展名
                .and_then(|s| s.to_str()) // 转换为 &str
                .map(|s| s == "syn") // 比较后缀
                .unwrap_or(false)
        })
    {
        println!("parse {:?}", synfile.path());

        let content = fs::read_to_string(synfile.path()).unwrap();

        let mut input = Chars::new(content.as_str());

        let s = f(&mut input);

        let jsonfile = synfile.path().with_extension("json");

        let content = fs::read_to_string(jsonfile).unwrap();

        let expect = serde_json::from_str::<serde_json::Value>(&content).unwrap();

        assert_eq!(s, expect);
    }
}
