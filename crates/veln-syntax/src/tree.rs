use veln_source::{SourceSpan, TextRange};

use crate::{BodyLine, FunctionDecl, ModuleDecl, SyntaxItem, Token, TokenKind, UseDecl};

#[derive(Clone, Debug)]
pub struct SyntaxTree {
    pub root: SyntaxNode,
    pub module: Option<ModuleDecl>,
    pub adr_lite_records: Vec<crate::AdrLiteRecord>,
    pub uses: Vec<UseDecl>,
    pub items: Vec<SyntaxItem>,
}

impl SyntaxTree {
    pub fn lossless_tokens(&self) -> impl Iterator<Item = &Token> {
        self.root.lossless_tokens()
    }

    pub fn descendant_nodes(&self) -> impl Iterator<Item = &SyntaxNode> {
        self.root.descendant_nodes()
    }
}

#[derive(Clone, Debug)]
pub struct SyntaxNode {
    pub kind: SyntaxNodeKind,
    pub range: TextRange,
    pub children: Vec<SyntaxElement>,
}

impl SyntaxNode {
    fn root(children: Vec<SyntaxElement>, source_len: usize) -> Self {
        Self {
            kind: SyntaxNodeKind::Root,
            range: TextRange::new(0, source_len),
            children,
        }
    }

    fn new(kind: SyntaxNodeKind, range: TextRange, children: Vec<SyntaxElement>) -> Self {
        Self {
            kind,
            range,
            children,
        }
    }

    pub fn lossless_tokens(&self) -> LosslessTokens<'_> {
        LosslessTokens {
            stack: self.children.iter().rev().collect(),
        }
    }

    pub fn descendant_nodes(&self) -> DescendantNodes<'_> {
        DescendantNodes {
            stack: self.children.iter().rev().collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyntaxNodeKind {
    Root,
    ModuleDecl,
    UseDecl,
    FunctionDecl,
    FunctionSignature,
    ContractClause,
    Body,
    LetStatement,
    ExprLine,
}

#[derive(Clone, Debug)]
pub enum SyntaxElement {
    Node(SyntaxNode),
    Token(Token),
}

pub struct LosslessTokens<'a> {
    stack: Vec<&'a SyntaxElement>,
}

impl<'a> Iterator for LosslessTokens<'a> {
    type Item = &'a Token;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.stack.pop()? {
                SyntaxElement::Token(token) => return Some(token),
                SyntaxElement::Node(node) => {
                    self.stack.extend(node.children.iter().rev());
                }
            }
        }
    }
}

pub struct DescendantNodes<'a> {
    stack: Vec<&'a SyntaxElement>,
}

impl<'a> Iterator for DescendantNodes<'a> {
    type Item = &'a SyntaxNode;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.stack.pop()? {
                SyntaxElement::Token(_) => {}
                SyntaxElement::Node(node) => {
                    self.stack.extend(node.children.iter().rev());
                    return Some(node);
                }
            }
        }
    }
}
pub(crate) fn build_lossless_root(
    tokens: Vec<Token>,
    source_len: usize,
    module: Option<&ModuleDecl>,
    uses: &[UseDecl],
    items: &[SyntaxItem],
) -> SyntaxNode {
    let mut children = Vec::new();
    let mut cursor = 0;
    let mut top_level = Vec::new();

    if let Some(module) = module {
        top_level.push(TopLevelNode::Module(span_range(&module.span)));
    }
    top_level.extend(
        uses.iter()
            .map(|use_decl| TopLevelNode::Use(span_range(&use_decl.span))),
    );
    top_level.extend(items.iter().map(|item| match item {
        SyntaxItem::Function(function) => TopLevelNode::Function(function),
    }));
    top_level.sort_by_key(|node| node.range().start);

    for node in top_level {
        push_tokens_before(&tokens, &mut cursor, node.range().start, &mut children);
        let node_tokens = take_tokens_in_range(&tokens, &mut cursor, node.range());
        children.push(SyntaxElement::Node(match node {
            TopLevelNode::Module(range) => {
                token_node(SyntaxNodeKind::ModuleDecl, range, node_tokens)
            }
            TopLevelNode::Use(range) => token_node(SyntaxNodeKind::UseDecl, range, node_tokens),
            TopLevelNode::Function(function) => build_lossless_function(function, node_tokens),
        }));
    }

    push_remaining_tokens(&tokens, &mut cursor, &mut children);
    SyntaxNode::root(children, source_len)
}

enum TopLevelNode<'a> {
    Module(TextRange),
    Use(TextRange),
    Function(&'a FunctionDecl),
}

impl TopLevelNode<'_> {
    fn range(&self) -> TextRange {
        match self {
            Self::Module(range) | Self::Use(range) => *range,
            Self::Function(function) => span_range(&function.span),
        }
    }
}

fn build_lossless_function(function: &FunctionDecl, tokens: Vec<Token>) -> SyntaxNode {
    let range = span_range(&function.span);
    let mut children = Vec::new();
    let mut cursor = 0;

    if let Some(signature_end) = tokens
        .iter()
        .find(|token| token.kind == TokenKind::Newline)
        .map(|token| token.range.end)
    {
        let signature_tokens = take_tokens_in_range(
            &tokens,
            &mut cursor,
            TextRange::new(range.start, signature_end),
        );
        children.push(SyntaxElement::Node(token_node(
            SyntaxNodeKind::FunctionSignature,
            TextRange::new(range.start, signature_end),
            signature_tokens,
        )));
    }

    for contract in &function.contracts {
        let contract_range = span_range(&contract.span);
        push_tokens_before(&tokens, &mut cursor, contract_range.start, &mut children);
        let contract_tokens = take_tokens_in_range(&tokens, &mut cursor, contract_range);
        children.push(SyntaxElement::Node(token_node(
            SyntaxNodeKind::ContractClause,
            contract_range,
            contract_tokens,
        )));
    }

    if !function.body.is_empty() {
        let mut body_children = Vec::new();
        for line in &function.body {
            let (line_range, kind) = match line {
                BodyLine::Let { span, .. } => (span_range(span), SyntaxNodeKind::LetStatement),
                BodyLine::Expr { span, .. } => (span_range(span), SyntaxNodeKind::ExprLine),
            };
            push_body_tokens_before(&tokens, &mut cursor, line_range.start, &mut body_children);
            let line_tokens = take_tokens_in_range(&tokens, &mut cursor, line_range);
            body_children.push(SyntaxElement::Node(token_node(
                kind,
                line_range,
                line_tokens,
            )));
        }
        while tokens
            .get(cursor)
            .is_some_and(|token| token.kind != TokenKind::End && token.kind != TokenKind::Eof)
        {
            body_children.push(SyntaxElement::Token(tokens[cursor].clone()));
            cursor += 1;
        }
        let body_range = element_children_range(&body_children);
        children.push(SyntaxElement::Node(SyntaxNode::new(
            SyntaxNodeKind::Body,
            body_range,
            body_children,
        )));
    }

    push_remaining_tokens(&tokens, &mut cursor, &mut children);
    SyntaxNode::new(SyntaxNodeKind::FunctionDecl, range, children)
}

fn token_node(kind: SyntaxNodeKind, range: TextRange, tokens: Vec<Token>) -> SyntaxNode {
    SyntaxNode::new(
        kind,
        range,
        tokens.into_iter().map(SyntaxElement::Token).collect(),
    )
}

fn push_tokens_before(
    tokens: &[Token],
    cursor: &mut usize,
    end: usize,
    children: &mut Vec<SyntaxElement>,
) {
    while tokens
        .get(*cursor)
        .is_some_and(|token| token.range.start < end)
    {
        children.push(SyntaxElement::Token(tokens[*cursor].clone()));
        *cursor += 1;
    }
}

fn push_body_tokens_before(
    tokens: &[Token],
    cursor: &mut usize,
    end: usize,
    children: &mut Vec<SyntaxElement>,
) {
    while tokens.get(*cursor).is_some_and(|token| {
        token.range.start < end && token.kind != TokenKind::End && token.kind != TokenKind::Eof
    }) {
        children.push(SyntaxElement::Token(tokens[*cursor].clone()));
        *cursor += 1;
    }
}

fn take_tokens_in_range(tokens: &[Token], cursor: &mut usize, range: TextRange) -> Vec<Token> {
    let mut taken = Vec::new();
    while tokens
        .get(*cursor)
        .is_some_and(|token| token.range.start >= range.start && token.range.end <= range.end)
    {
        taken.push(tokens[*cursor].clone());
        *cursor += 1;
    }
    taken
}

fn push_remaining_tokens(tokens: &[Token], cursor: &mut usize, children: &mut Vec<SyntaxElement>) {
    while let Some(token) = tokens.get(*cursor) {
        children.push(SyntaxElement::Token(token.clone()));
        *cursor += 1;
    }
}

fn element_children_range(children: &[SyntaxElement]) -> TextRange {
    let mut range: Option<TextRange> = None;
    for child in children {
        let child_range = match child {
            SyntaxElement::Node(node) => node.range,
            SyntaxElement::Token(token) => token.range,
        };
        range = Some(match range {
            Some(range) => range.cover(child_range),
            None => child_range,
        });
    }
    range.unwrap_or_default()
}

fn span_range(span: &SourceSpan) -> TextRange {
    TextRange::new(span.start.offset, span.end.offset)
}
