use crate::{
    errors::CompileError,
    input::UnsynInput,
    syntax::{Module, Stmt},
    visit::Visitor,
};

/// File-level symbol analyzer
struct FindStmts<F> {
    f: F,
    errors: Vec<CompileError>,
}

impl<F, I> Visitor<I> for FindStmts<F>
where
    I: UnsynInput,
    F: FnMut(Stmt<I>) -> Result<(), CompileError>,
{
    #[inline]
    fn visit_item_stmt(&mut self, stmt: &mut Stmt<I>) {
        if let Err(err) = (self.f)(stmt.clone()) {
            self.errors.push(err);
        }
    }
}

/// locate all symbols defined in a module.
#[inline]
pub fn findsym<I, F>(module: &mut Module<I>, f: F) -> Result<(), Vec<CompileError>>
where
    I: UnsynInput,
    F: FnMut(Stmt<I>) -> Result<(), CompileError>,
{
    let mut find = FindStmts {
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
    fn filesymbols() {
        let mut module = Module::parse(&mut Chars::new(include_str!("../../unsyn.syn"))).unwrap();

        let mut counter = 0;

        findsym(&mut module, |_| {
            counter += 1;
            Ok(())
        })
        .unwrap();

        assert_eq!(counter, 37)
    }
}
