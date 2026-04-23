use parserc::syntax::Syntax;
use unsyn::syntax::Expr;

mod spec;

#[test]
fn spec_stmts() {
    spec::run_spec("expr", |input| {
        serde_json::to_value(Expr::parse(input).unwrap()).unwrap()
    });
}
