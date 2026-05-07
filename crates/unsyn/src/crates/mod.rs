//! The `crate` is the minimal compilation unit of `unsyn`

use std::{collections::HashMap, fs, mem::transmute, path::Path};

use parserc::{
    sourceinput::{Length, Source, Span, ToSpan},
    syntax::Syntax,
};

use crate::{
    errors::{CompileError, SemanticsKind, SyntaxKind, UnsynError},
    input::Chars,
    semantics::finduse,
    syntax::{self, UseDeclaration},
};

#[derive(Default)]
struct SymbolTable(HashMap<&'static str, HashMap<&'static str, Span>>);

impl SymbolTable {
    #[allow(unused)]
    #[inline]
    pub fn insert(
        &mut self,
        module: &'static str,
        name: &'static str,
        span: Span,
    ) -> Result<(), CompileError> {
        if let Some(previous) = self
            .0
            .entry(module)
            .or_insert_with(|| Default::default())
            .insert(name, span)
        {
            Err(CompileError::Semantics(
                SemanticsKind::NameCollision(previous),
                span,
            ))
        } else {
            Ok(())
        }
    }
}

struct Module {
    buf: String,
    module: syntax::Module<Chars<'static>>,
    use_decls: Vec<UseDeclaration<Chars<'static>>>,
}

impl Module {
    pub fn parse<P>(path: P) -> Result<Self, UnsynError>
    where
        P: AsRef<Path>,
    {
        let buf = fs::read_to_string(&path)?;

        let mut input: Source<&'static str, _> = Chars::new(unsafe { transmute(buf.as_str()) });

        let mut module = syntax::Module::parse(&mut input)
            .map_err(|err| UnsynError::CompileError(path.as_ref().to_owned(), err))?;

        if !input.is_empty() {
            return Err(UnsynError::CompileError(
                path.as_ref().to_owned(),
                CompileError::Semantics(SemanticsKind::Unparsing, input.to_span()),
            ));
        }

        let mut use_decls = vec![];

        finduse(&mut module, |use_declaration| {
            use_decls.push(use_declaration);
            Ok(())
        })
        .map_err(|err| UnsynError::CompileErrors(path.as_ref().to_owned(), err))?;

        Ok(Self {
            buf,
            module,
            use_decls,
        })
    }
}

/// The minimal compilation unit of `unsyn`
#[allow(unused)]
#[derive(Default)]
pub struct Crate {
    /// loaded source files.
    modules: HashMap<String, Module>,
    /// symbols within the scope of this crate
    symbols: SymbolTable,
}

impl Crate {
    /// Build a crate by parsing the entry module file.
    pub fn parse<P>(entry: P) -> Result<Self, UnsynError>
    where
        P: AsRef<Path>,
    {
        let mut c = Crate::default();

        c.load(entry)?;

        Ok(c)
    }

    fn load<P: AsRef<Path>>(&mut self, path: P) -> Result<(), UnsynError> {
        let module = Module::parse(&path)?;

        self.modules
            .insert(path.as_ref().to_str().unwrap().to_owned(), module);

        Ok(())
    }
}
