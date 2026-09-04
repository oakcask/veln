use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use veln_source::SourceFile;

fn first_function(output: &ParseOutput) -> &FunctionDecl {
    match &output.tree.items[0] {
        SyntaxItem::Function(function) => function,
        SyntaxItem::Effect(_)
        | SyntaxItem::Handler(_)
        | SyntaxItem::Type(_)
        | SyntaxItem::Schema(_)
        | SyntaxItem::PublicAlias(_) => {
            panic!("expected function item")
        }
    }
}

mod calls_and_generics;
mod declarations_and_aliases;
mod effects_and_handlers;
mod expressions;
mod lexer_and_fixtures;
mod literals_and_numbers;
mod match_formatting;
mod modules_and_contracts;
mod patterns_and_comments;
mod patterns_and_control_flow;
mod schemas;
