use crate::ast::types::Type;
use crate::error::ParserError;
use crate::parser::Rule;
use crate::prelude::ParserResult;
use pest::iterators::Pair;

/// Function parameter
///
/// Can be either Stream or Simple depending on how
/// it was declared.
#[derive(Debug, Clone)]
pub enum FunctionParam {
    Stream(Type),
    Simple(Type),
}

impl FunctionParam {
    pub fn try_parse(pair: Pair<'_, Rule>) -> ParserResult<Self> {
        let inner_pair = pair.into_inner().next().ok_or(ParserError::NoNextToken)?;

        let type_pair = inner_pair
            .clone()
            .into_inner()
            .next()
            .ok_or(ParserError::NoNextToken)?;
        let ty = Type::try_parse(type_pair)?;

        match inner_pair.as_rule() {
            Rule::stream_decl => Ok(FunctionParam::Stream(ty)),
            Rule::type_decl => Ok(FunctionParam::Simple(ty)),
            _ => Err(ParserError::UnexpectedRule(inner_pair.as_rule())),
        }
    }
}

/// Function return
///
/// Can be either Stream or Simple depending on how
/// it was declared.
#[derive(Debug, Clone)]
pub enum FunctionReturn {
    Stream(Type),
    Simple(Type),
}

impl FunctionReturn {
    pub fn try_parse(pair: Pair<'_, Rule>) -> ParserResult<Self> {
        let inner_pair = pair.into_inner().next().ok_or(ParserError::NoNextToken)?;

        let type_pair = inner_pair
            .clone()
            .into_inner()
            .next()
            .ok_or(ParserError::NoNextToken)?;
        let ty = Type::try_parse(type_pair)?;

        match inner_pair.as_rule() {
            Rule::stream_decl => Ok(FunctionReturn::Stream(ty)),
            Rule::type_decl => Ok(FunctionReturn::Simple(ty)),
            _ => Err(ParserError::UnexpectedRule(inner_pair.as_rule())),
        }
    }
}

/// Function definition
///
/// Composed of the function name, its input parameter and
/// its return type, which might be optional.
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub param: FunctionParam,
    pub return_type: Option<FunctionReturn>,
}

impl Function {
    pub fn try_parse(pair: Pair<'_, Rule>) -> ParserResult<Self> {
        let mut inner_pair = pair.into_inner();

        let name = inner_pair
            .next()
            .ok_or(ParserError::NoNextToken)?
            .as_str()
            .to_string();

        let param = FunctionParam::try_parse(inner_pair.next().ok_or(ParserError::NoNextToken)?)?;

        let return_type = inner_pair
            .next()
            .map(FunctionReturn::try_parse)
            .transpose()?;

        Ok(Self {
            name,
            param,
            return_type,
        })
    }
}

/// Interface definition
///
/// Composed of its name and a vector of functions.
#[derive(Debug, Clone)]
pub struct Interface {
    pub name: String,
    pub functions: Vec<Function>,
}

impl Interface {
    pub fn try_parse(pair: Pair<'_, Rule>) -> ParserResult<Self> {
        let mut inner_pair = pair.into_inner();
        let name = inner_pair
            .next()
            .ok_or(ParserError::NoNextToken)?
            .as_str()
            .to_owned();
        let body = inner_pair.next().ok_or(ParserError::NoNextToken)?;

        let functions = body
            .into_inner()
            .map(Function::try_parse)
            .collect::<ParserResult<_>>()?;

        Ok(Self { name, functions })
    }
}
