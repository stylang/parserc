use parserc::syntax::Syntax;
use unsyn::syntax::Stmt;

mod spec;

#[test]
fn spec_stmts() {
    spec::run_spec("stmt", |input| {
        serde_json::to_value(Stmt::parse(input).unwrap()).unwrap()
    });
}
