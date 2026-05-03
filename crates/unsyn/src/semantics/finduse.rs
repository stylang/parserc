use crate::{
    errors::UnsynError,
    input::UnsynInput,
    syntax::{Module, UseDeclaration},
    visit::Visitor,
};

/// A visitor to locate all use statements within a module.
#[derive(Default)]
struct FindUse<F> {
    f: F,
    errors: Vec<UnsynError>,
}

impl<I, F> Visitor<I> for FindUse<F>
where
    I: UnsynInput,
    F: FnMut(UseDeclaration<I>) -> Result<(), UnsynError>,
{
    fn visit_item_use(
        &mut self,
        use_declaration: &mut UseDeclaration<I>,
        _: &mut crate::token::punct::Semi<I>,
    ) {
        if let Err(err) = (self.f)(use_declaration.clone()) {
            self.errors.push(err);
        }
    }

    fn visit_item_s(&mut self, _: &mut crate::token::S<I>) {}

    fn visit_item_outer_doc(&mut self, _: &mut crate::syntax::OuterDoc<I>) {}

    fn visit_item_stmt(&mut self, _: &mut crate::syntax::Stmt<I>) {}
}

/// locate all use statements within a module.
#[inline]
pub fn find_use_stmts<I, F>(module: &mut Module<I>, f: F) -> Result<(), Vec<UnsynError>>
where
    I: UnsynInput,
    F: FnMut(UseDeclaration<I>) -> Result<(), UnsynError>,
{
    let mut find = FindUse {
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
