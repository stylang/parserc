use crate::{
    errors::CompileError, input::UnsynInput, syntax::Module, token::ident::Ident, visit::Visitor,
};

/// A visitor to locate ident within a module.
#[derive(Default)]
struct LinkModule<F> {
    f: F,
    errors: Vec<CompileError>,
}

impl<I, F> Visitor<I> for LinkModule<F>
where
    I: UnsynInput,
    F: FnMut(Ident<I>) -> Result<(), CompileError>,
{
    fn visit_item_s(&mut self, _: &mut crate::token::S<I>) {}

    fn visit_item_mod(
        &mut self,
        _: &mut crate::syntax::ModuleDeclaration<I>,
        _: &mut crate::token::punct::Semi<I>,
    ) {
    }

    fn visit_item_use(
        &mut self,
        _: &mut crate::syntax::UseDeclaration<I>,
        _: &mut crate::token::punct::Semi<I>,
    ) {
    }

    fn visit_item_outer_doc(&mut self, _: &mut crate::syntax::OuterDoc<I>) {}

    fn visit_item_stmt(&mut self, stmt: &mut crate::syntax::Stmt<I>) {
        match stmt {
            crate::syntax::Stmt::Whitespace {
                keyword: _,
                ident: _,
                arrow_right: _,
                expr,
                semi: _,
            }
            | crate::syntax::Stmt::Lexer {
                keyword: _,
                ident: _,
                arrow_right: _,
                expr,
                semi: _,
            }
            | crate::syntax::Stmt::Syntax {
                keyword: _,
                ident: _,
                arrow_right: _,
                expr,
                semi: _,
            } => {
                self.visit_expr(expr);
            }
        }
    }

    fn visit_token_ident(&mut self, ident: &mut Ident<I>) {
        if let Err(err) = (self.f)(ident.clone()) {
            self.errors.push(err);
        }
    }
}

/// Resolve symbols for a module.
pub fn linksym<I, F>(module: &mut Module<I>, f: F) -> Result<(), Vec<CompileError>>
where
    I: UnsynInput,
    F: FnMut(Ident<I>) -> Result<(), CompileError>,
{
    let mut find = LinkModule {
        f,
        errors: Default::default(),
    };

    find.visit_module(module);

    if find.errors.is_empty() {
        Ok(())
    } else {
        Err(find.errors)
    }
}

#[cfg(test)]
mod tests {
    use parserc::syntax::Syntax;

    use crate::{input::Chars, syntax::Module};

    use super::*;

    #[test]
    fn linkmodule() {
        let mut module = Module::parse(&mut Chars::new(include_str!("../../unsyn.syn"))).unwrap();

        let mut counter = 0;

        linksym(&mut module, |_| {
            counter += 1;
            Ok(())
        })
        .unwrap();

        assert_eq!(counter, 76);
    }
}
