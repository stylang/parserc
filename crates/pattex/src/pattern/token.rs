use parserc::syntax::Syntax;

use crate::{errors::CompileError, input::PatternInput};

/// backslash token `\`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(char = '\\')]
#[syntax(map_err = CompileError::Token.map())]
pub struct BackSlash<I>(pub I)
where
    I: PatternInput;

/// caret token `^`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(char = '^')]
#[syntax(map_err = CompileError::Token.map())]
pub struct Caret<I>(pub I)
where
    I: PatternInput;

/// brace start token `{`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(char = '{')]
#[syntax(map_err = CompileError::Token.map())]
pub struct BraceStart<I>(pub I)
where
    I: PatternInput;

/// brace end token `}`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(char = '}')]
#[syntax(map_err = CompileError::Token.map())]
pub struct BraceEnd<I>(pub I)
where
    I: PatternInput;

/// bracket start token `[`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(char = '[')]
#[syntax(map_err = CompileError::Token.map())]
pub struct BracketStart<I>(pub I)
where
    I: PatternInput;

/// bracket end token `]`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(char = ']')]
#[syntax(map_err = CompileError::Token.map())]
pub struct BracketEnd<I>(pub I)
where
    I: PatternInput;

/// parenthesis start token `(`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(char = '(')]
#[syntax(map_err = CompileError::Token.map())]
pub struct ParenStart<I>(pub I)
where
    I: PatternInput;

/// parenthesis end token `)`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(char = ')')]
#[syntax(map_err = CompileError::Token.map())]
pub struct ParenEnd<I>(pub I)
where
    I: PatternInput;

/// or token `|`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(char = '|')]
#[syntax(map_err = CompileError::Token.map())]
pub struct Or<I>(pub I)
where
    I: PatternInput;

/// question token `?`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(char = '?')]
#[syntax(map_err = CompileError::Token.map())]
pub struct Question<I>(pub I)
where
    I: PatternInput;

/// dot token `.`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(char = '.')]
#[syntax(map_err = CompileError::Token.map())]
pub struct Dot<I>(pub I)
where
    I: PatternInput;

/// plus token `+`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(char = '+')]
#[syntax(map_err = CompileError::Token.map())]
pub struct Plus<I>(pub I)
where
    I: PatternInput;

/// minus token `-`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(char = '-')]
#[syntax(map_err = CompileError::Token.map())]
pub struct Minus<I>(pub I)
where
    I: PatternInput;

/// star token `*`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(char = '*')]
#[syntax(map_err = CompileError::Token.map())]
pub struct Star<I>(pub I)
where
    I: PatternInput;

/// dollar token `$`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(char = '$')]
#[syntax(map_err = CompileError::Token.map())]
pub struct Dollar<I>(pub I)
where
    I: PatternInput;

/// token `(?:`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(keyword = "(?:")]
#[syntax(map_err = CompileError::Token.map())]
pub struct BracketStartQeustionColon<I>(pub I)
where
    I: PatternInput;

/// token `(?=`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(keyword = "(?=")]
#[syntax(map_err = CompileError::Token.map())]
pub struct BracketStartQeustionEq<I>(pub I)
where
    I: PatternInput;

/// token `(?!`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(keyword = "(?!")]
#[syntax(map_err = CompileError::Token.map())]
pub struct BracketStartQeustionNot<I>(pub I)
where
    I: PatternInput;

/// token `(?<=`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(keyword = "(?<=")]
#[syntax(map_err = CompileError::Token.map())]
pub struct BracketStartQeustionLtEq<I>(pub I)
where
    I: PatternInput;

/// token `(?<!`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[syntax(keyword = "(?<!")]
#[syntax(map_err = CompileError::Token.map())]
pub struct BracketStartQeustionLtNot<I>(pub I)
where
    I: PatternInput;

#[inline]
pub(super) fn is_token_char(c: char) -> bool {
    match c {
        '\\' | '|' | '^' | '$' | '*' | '+' | '-' | '?' | '{' | '[' | ']' | '.' | '=' | '('
        | ')' => true,
        _ => false,
    }
}
