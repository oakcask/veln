//! Lexer, parser, lossless tree, and formatting input.

use veln_source::{SourceFile, SourceSpan, TextRange};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Whitespace,
    Comment,
    Ident,
    Hole,
    String,
    Int,
    Float,
    Newline,
    Eof,
    Invalid,
    Pub,
    Fn,
    Effects,
    Let,
    End,
    Require,
    Ensure,
    Mod,
    Use,
    Match,
    Or,
    And,
    Not,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Dot,
    DoubleColon,
    Arrow,
    FatArrow,
    PipeGreater,
    Question,
    Underscore,
    Equal,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Plus,
    Minus,
    Star,
    Slash,
}

impl TokenKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Whitespace => "whitespace",
            Self::Comment => "comment",
            Self::Ident => "identifier",
            Self::Hole => "hole",
            Self::String => "string",
            Self::Int => "integer",
            Self::Float => "float",
            Self::Newline => "newline",
            Self::Eof => "end of file",
            Self::Invalid => "invalid token",
            Self::Pub => "pub",
            Self::Fn => "fn",
            Self::Effects => "effects",
            Self::Let => "let",
            Self::End => "end",
            Self::Require => "require",
            Self::Ensure => "ensure",
            Self::Mod => "mod",
            Self::Use => "use",
            Self::Match => "match",
            Self::Or => "or",
            Self::And => "and",
            Self::Not => "not",
            Self::LParen => "(",
            Self::RParen => ")",
            Self::LBracket => "[",
            Self::RBracket => "]",
            Self::LBrace => "{",
            Self::RBrace => "}",
            Self::Comma => ",",
            Self::Colon => ":",
            Self::Dot => ".",
            Self::DoubleColon => "::",
            Self::Arrow => "->",
            Self::FatArrow => "=>",
            Self::PipeGreater => "|>",
            Self::Question => "?",
            Self::Underscore => "_",
            Self::Equal => "=",
            Self::EqualEqual => "==",
            Self::BangEqual => "!=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub range: TextRange,
}

impl Token {
    fn eof(offset: usize) -> Self {
        Self {
            kind: TokenKind::Eof,
            text: String::new(),
            range: TextRange::at(offset),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Lexed {
    pub tokens: Vec<Token>,
}

#[derive(Clone, Debug)]
pub struct SyntaxTree {
    pub root: SyntaxNode,
    pub module: Option<ModuleDecl>,
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

#[derive(Clone, Debug)]
pub struct ModuleDecl {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct UseDecl {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub enum SyntaxItem {
    Function(FunctionDecl),
}

#[derive(Clone, Debug)]
pub struct FunctionDecl {
    pub visibility: Visibility,
    pub name: Option<String>,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub effects: Option<Vec<String>>,
    pub contracts: Vec<ContractClause>,
    pub body: Vec<BodyLine>,
    pub span: SourceSpan,
    pub end_present: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: Option<String>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct ContractClause {
    pub kind: ContractKind,
    pub text: String,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractKind {
    Require,
    Ensure,
}

#[derive(Clone, Debug)]
pub enum BodyLine {
    Let {
        name: Option<String>,
        annotation: Option<String>,
        expr: Expr,
        span: SourceSpan,
    },
    Expr {
        expr: Expr,
        span: SourceSpan,
    },
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Missing,
    Hole {
        name: Option<String>,
        satisfy: Option<SatisfyClause>,
    },
    NamePath(Vec<String>),
    StringLiteral(String),
    IntLiteral(String),
    FloatLiteral(String),
    Unit,
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Try(Box<Expr>),
    Record(Vec<RecordField>),
    List(Vec<Expr>),
    Prefix {
        op: PrefixOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Clone, Debug)]
pub struct SatisfyClause {
    pub candidate: Option<String>,
    pub predicate: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct RecordField {
    pub name: String,
    pub expr: Expr,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefixOp {
    Not,
    Negate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    PipeGreater,
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Debug)]
pub struct ParseOutput {
    pub tree: SyntaxTree,
    pub diagnostics: Vec<ParseDiagnostic>,
}

#[derive(Clone, Debug)]
pub struct ParseDiagnostic {
    pub id: &'static str,
    pub message: String,
    pub span: Option<SourceSpan>,
    pub parser_context: &'static str,
    pub unexpected: UnexpectedToken,
    pub expected: Vec<&'static str>,
    pub recovery: Recovery,
}

#[derive(Clone, Debug)]
pub struct UnexpectedToken {
    pub kind: String,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Recovery {
    pub strategy: RecoveryStrategy,
    pub anchor: Option<String>,
    pub dropped_token_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryStrategy {
    None,
    SkipToken,
    InsertToken,
    CloseBlock,
    SynchronizeToAnchor,
}

impl RecoveryStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::SkipToken => "skip_token",
            Self::InsertToken => "insert_token",
            Self::CloseBlock => "close_block",
            Self::SynchronizeToAnchor => "synchronize_to_anchor",
        }
    }
}

impl TokenKind {
    fn is_trivia(&self) -> bool {
        matches!(self, Self::Whitespace | Self::Comment)
    }
}

fn build_lossless_root(
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

fn normalize_collected_text(parts: Vec<String>) -> String {
    parts
        .join(" ")
        .replace(" :: ", "::")
        .replace(" (", "(")
        .replace("( ", "(")
        .replace(" )", ")")
        .replace("[ ", "[")
        .replace(" ]", "]")
        .replace(" ,", ",")
}

pub fn lex(source: &SourceFile) -> Lexed {
    let text = source.text();
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();

    while let Some((start, ch)) = chars.next() {
        match ch {
            ' ' | '\t' | '\r' => tokens.push(read_whitespace(text, start, ch, &mut chars)),
            '\n' => tokens.push(token(TokenKind::Newline, "\n", start, start + 1)),
            '/' if chars.peek().is_some_and(|(_, next)| *next == '/') => {
                tokens.push(read_comment(text, start, &mut chars));
            }
            '"' => tokens.push(read_string(text, start, &mut chars)),
            '0'..='9' => tokens.push(read_number(text, start, ch, &mut chars)),
            'A'..='Z' | 'a'..='z' => {
                tokens.push(read_ident_or_keyword(text, start, ch, &mut chars))
            }
            '_' => tokens.push(read_underscore_or_ident(text, start, &mut chars)),
            '(' => tokens.push(token(TokenKind::LParen, "(", start, start + 1)),
            ')' => tokens.push(token(TokenKind::RParen, ")", start, start + 1)),
            '[' => tokens.push(token(TokenKind::LBracket, "[", start, start + 1)),
            ']' => tokens.push(token(TokenKind::RBracket, "]", start, start + 1)),
            '{' => tokens.push(token(TokenKind::LBrace, "{", start, start + 1)),
            '}' => tokens.push(token(TokenKind::RBrace, "}", start, start + 1)),
            ',' => tokens.push(token(TokenKind::Comma, ",", start, start + 1)),
            '.' => tokens.push(token(TokenKind::Dot, ".", start, start + 1)),
            ':' if chars.peek().is_some_and(|(_, next)| *next == ':') => {
                chars.next();
                tokens.push(token(TokenKind::DoubleColon, "::", start, start + 2));
            }
            ':' => tokens.push(token(TokenKind::Colon, ":", start, start + 1)),
            '-' if chars.peek().is_some_and(|(_, next)| *next == '>') => {
                chars.next();
                tokens.push(token(TokenKind::Arrow, "->", start, start + 2));
            }
            '-' => tokens.push(token(TokenKind::Minus, "-", start, start + 1)),
            '=' if chars.peek().is_some_and(|(_, next)| *next == '>') => {
                chars.next();
                tokens.push(token(TokenKind::FatArrow, "=>", start, start + 2));
            }
            '=' if chars.peek().is_some_and(|(_, next)| *next == '=') => {
                chars.next();
                tokens.push(token(TokenKind::EqualEqual, "==", start, start + 2));
            }
            '=' => tokens.push(token(TokenKind::Equal, "=", start, start + 1)),
            '!' if chars.peek().is_some_and(|(_, next)| *next == '=') => {
                chars.next();
                tokens.push(token(TokenKind::BangEqual, "!=", start, start + 2));
            }
            '<' if chars.peek().is_some_and(|(_, next)| *next == '=') => {
                chars.next();
                tokens.push(token(TokenKind::LessEqual, "<=", start, start + 2));
            }
            '<' => tokens.push(token(TokenKind::Less, "<", start, start + 1)),
            '>' if chars.peek().is_some_and(|(_, next)| *next == '=') => {
                chars.next();
                tokens.push(token(TokenKind::GreaterEqual, ">=", start, start + 2));
            }
            '>' => tokens.push(token(TokenKind::Greater, ">", start, start + 1)),
            '|' if chars.peek().is_some_and(|(_, next)| *next == '>') => {
                chars.next();
                tokens.push(token(TokenKind::PipeGreater, "|>", start, start + 2));
            }
            '?' => tokens.push(token(TokenKind::Question, "?", start, start + 1)),
            '+' => tokens.push(token(TokenKind::Plus, "+", start, start + 1)),
            '*' => tokens.push(token(TokenKind::Star, "*", start, start + 1)),
            '/' => tokens.push(token(TokenKind::Slash, "/", start, start + 1)),
            _ => tokens.push(token(
                TokenKind::Invalid,
                ch.to_string(),
                start,
                start + ch.len_utf8(),
            )),
        }
    }

    tokens.push(Token::eof(source.len()));
    Lexed { tokens }
}

pub fn parse(source: &SourceFile) -> ParseOutput {
    let lexed = lex(source);
    Parser::new(source, lexed.tokens).parse()
}

pub fn format_tree(tree: &SyntaxTree) -> String {
    if tree
        .lossless_tokens()
        .any(|token| token.kind == TokenKind::Comment)
    {
        return tree
            .lossless_tokens()
            .filter(|token| token.kind != TokenKind::Eof)
            .map(|token| token.text.as_str())
            .collect();
    }

    let mut out = String::new();
    if let Some(module) = &tree.module {
        push_line(&mut out, format_args!("mod {}", module.name));
    }
    for use_decl in &tree.uses {
        push_line(&mut out, format_args!("use {}", use_decl.name));
    }
    if (tree.module.is_some() || !tree.uses.is_empty()) && !tree.items.is_empty() {
        out.push('\n');
    }

    for (index, item) in tree.items.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let SyntaxItem::Function(function) = item;
        format_function(&mut out, function);
    }

    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn format_function(out: &mut String, function: &FunctionDecl) {
    if function.visibility == Visibility::Public {
        out.push_str("pub ");
    }
    out.push_str("fn ");
    out.push_str(function.name.as_deref().unwrap_or("<missing>"));
    out.push('(');
    for (index, param) in function.params.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&param.name);
        if let Some(ty) = &param.ty {
            out.push_str(": ");
            out.push_str(ty);
        }
    }
    out.push(')');
    if let Some(return_type) = &function.return_type {
        out.push_str(" -> ");
        out.push_str(return_type);
    }
    if let Some(effects) = &function.effects {
        out.push_str(" effects [");
        for (index, effect) in effects.iter().enumerate() {
            if index > 0 {
                out.push_str(", ");
            }
            out.push_str(effect);
        }
        out.push(']');
    }
    out.push('\n');

    for contract in &function.contracts {
        out.push_str("  ");
        out.push_str(match contract.kind {
            ContractKind::Require => "require",
            ContractKind::Ensure => "ensure",
        });
        if !contract.text.is_empty() {
            out.push(' ');
            out.push_str(&contract.text);
        }
        out.push('\n');
    }

    for line in &function.body {
        out.push_str("  ");
        match line {
            BodyLine::Let {
                name,
                annotation,
                expr,
                ..
            } => {
                out.push_str("let ");
                out.push_str(name.as_deref().unwrap_or("<missing>"));
                if let Some(annotation) = annotation {
                    out.push_str(": ");
                    out.push_str(annotation);
                }
                out.push_str(" = ");
                out.push_str(&format_expr(expr));
            }
            BodyLine::Expr { expr, .. } => out.push_str(&format_expr(expr)),
        }
        out.push('\n');
    }
    out.push_str("end\n");
}

fn push_line(out: &mut String, args: std::fmt::Arguments<'_>) {
    use std::fmt::Write as _;

    out.write_fmt(args)
        .expect("writing to String should not fail");
    out.push('\n');
}

fn format_expr(expr: &Expr) -> String {
    format_expr_prec(expr, 0, ExprSide::Root)
}

#[derive(Clone, Copy)]
enum ExprSide {
    Root,
    Left,
    Right,
}

fn format_expr_prec(expr: &Expr, parent_prec: u8, side: ExprSide) -> String {
    let prec = expr_prec(expr);
    let mut rendered = match &expr.kind {
        ExprKind::Missing => "_".to_string(),
        ExprKind::Hole { name, satisfy } => {
            let mut text = String::from("_");
            if let Some(name) = name {
                text.push_str(name);
            }
            if let Some(satisfy) = satisfy {
                text.push_str(" satisfy");
                if let Some(candidate) = &satisfy.candidate {
                    text.push(' ');
                    text.push_str(candidate);
                }
                if !satisfy.predicate.is_empty() {
                    text.push_str(" => ");
                    text.push_str(&satisfy.predicate);
                }
            }
            text
        }
        ExprKind::NamePath(segments) => segments.join("::"),
        ExprKind::StringLiteral(value)
        | ExprKind::IntLiteral(value)
        | ExprKind::FloatLiteral(value) => value.clone(),
        ExprKind::Unit => "()".to_string(),
        ExprKind::Call { callee, args } => {
            let args = args.iter().map(format_expr).collect::<Vec<_>>().join(", ");
            format!(
                "{}({args})",
                format_expr_prec(callee, expr_prec(expr), ExprSide::Left)
            )
        }
        ExprKind::Try(inner) => format!(
            "{}?",
            format_expr_prec(inner, expr_prec(expr), ExprSide::Left)
        ),
        ExprKind::Record(fields) => {
            if fields.is_empty() {
                return "{}".to_string();
            }
            let fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, format_expr(&field.expr)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {fields} }}")
        }
        ExprKind::List(items) => {
            let items = items.iter().map(format_expr).collect::<Vec<_>>().join(", ");
            format!("[{items}]")
        }
        ExprKind::Prefix { op, expr: inner } => match op {
            PrefixOp::Not => format!(
                "not {}",
                format_expr_prec(inner, expr_prec(expr), ExprSide::Right)
            ),
            PrefixOp::Negate => format!(
                "-{}",
                format_expr_prec(inner, expr_prec(expr), ExprSide::Right)
            ),
        },
        ExprKind::Binary { op, left, right } => {
            let op_text = binary_op_text(*op);
            format!(
                "{} {op_text} {}",
                format_expr_prec(left, expr_prec(expr), ExprSide::Left),
                format_expr_prec(right, expr_prec(expr), ExprSide::Right)
            )
        }
    };

    let needs_parens = match side {
        ExprSide::Root | ExprSide::Left => prec < parent_prec,
        ExprSide::Right => prec <= parent_prec && matches!(expr.kind, ExprKind::Binary { .. }),
    };
    if needs_parens {
        rendered.insert(0, '(');
        rendered.push(')');
    }
    rendered
}

fn expr_prec(expr: &Expr) -> u8 {
    match &expr.kind {
        ExprKind::Binary { op, .. } => match op {
            BinaryOp::PipeGreater => 1,
            BinaryOp::Or => 3,
            BinaryOp::And => 5,
            BinaryOp::Equal | BinaryOp::NotEqual => 7,
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => 9,
            BinaryOp::Add | BinaryOp::Subtract => 11,
            BinaryOp::Multiply | BinaryOp::Divide => 13,
        },
        ExprKind::Prefix { .. } => 15,
        ExprKind::Call { .. } | ExprKind::Try(_) => 17,
        _ => 19,
    }
}

fn binary_op_text(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::PipeGreater => "|>",
        BinaryOp::Or => "or",
        BinaryOp::And => "and",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
    }
}

fn read_whitespace(
    text: &str,
    start: usize,
    first: char,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Token {
    let mut end = start + first.len_utf8();
    while let Some((index, next)) = chars.peek().copied() {
        if matches!(next, ' ' | '\t' | '\r') {
            chars.next();
            end = index + next.len_utf8();
        } else {
            break;
        }
    }
    Token {
        kind: TokenKind::Whitespace,
        text: text[start..end].to_string(),
        range: TextRange::new(start, end),
    }
}

fn read_comment(
    text: &str,
    start: usize,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Token {
    let mut end = start;
    while let Some((index, next)) = chars.peek().copied() {
        if next == '\n' {
            break;
        }
        chars.next();
        end = index + next.len_utf8();
    }
    Token {
        kind: TokenKind::Comment,
        text: text[start..end].to_string(),
        range: TextRange::new(start, end),
    }
}

fn read_string(
    text: &str,
    start: usize,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Token {
    let mut end = start + 1;
    let mut escaped = false;
    for (index, ch) in chars.by_ref() {
        end = index + ch.len_utf8();
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            break;
        }
    }
    Token {
        kind: TokenKind::String,
        text: text[start..end].to_string(),
        range: TextRange::new(start, end),
    }
}

fn read_number(
    text: &str,
    start: usize,
    first: char,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Token {
    let mut end = start + first.len_utf8();
    let mut is_float = false;
    while let Some((index, next)) = chars.peek().copied() {
        if next.is_ascii_digit() {
            chars.next();
            end = index + next.len_utf8();
        } else {
            break;
        }
    }
    if chars.peek().is_some_and(|(_, next)| *next == '.') {
        let mut lookahead = chars.clone();
        lookahead.next();
        if lookahead
            .peek()
            .is_some_and(|(_, next)| next.is_ascii_digit())
        {
            is_float = true;
            chars.next();
            end += 1;
            while let Some((index, next)) = chars.peek().copied() {
                if next.is_ascii_digit() {
                    chars.next();
                    end = index + next.len_utf8();
                } else {
                    break;
                }
            }
        }
    }
    Token {
        kind: if is_float {
            TokenKind::Float
        } else {
            TokenKind::Int
        },
        text: text[start..end].to_string(),
        range: TextRange::new(start, end),
    }
}

fn read_ident_or_keyword(
    text: &str,
    start: usize,
    first: char,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Token {
    let mut end = start + first.len_utf8();
    while let Some((index, next)) = chars.peek().copied() {
        if is_ident_continue(next) {
            chars.next();
            end = index + next.len_utf8();
        } else {
            break;
        }
    }
    let token_text = &text[start..end];
    let kind = match token_text {
        "pub" => TokenKind::Pub,
        "fn" => TokenKind::Fn,
        "effects" => TokenKind::Effects,
        "let" => TokenKind::Let,
        "end" => TokenKind::End,
        "require" => TokenKind::Require,
        "ensure" => TokenKind::Ensure,
        "mod" => TokenKind::Mod,
        "use" => TokenKind::Use,
        "match" => TokenKind::Match,
        "or" => TokenKind::Or,
        "and" => TokenKind::And,
        "not" => TokenKind::Not,
        _ => TokenKind::Ident,
    };
    Token {
        kind,
        text: token_text.to_string(),
        range: TextRange::new(start, end),
    }
}

fn read_underscore_or_ident(
    text: &str,
    start: usize,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Token {
    let mut end = start + 1;
    let mut has_suffix = false;
    while let Some((index, next)) = chars.peek().copied() {
        if is_ident_continue(next) {
            chars.next();
            end = index + next.len_utf8();
            has_suffix = true;
        } else {
            break;
        }
    }
    Token {
        kind: if has_suffix {
            TokenKind::Hole
        } else {
            TokenKind::Underscore
        },
        text: text[start..end].to_string(),
        range: TextRange::new(start, end),
    }
}

fn token(kind: TokenKind, text: impl Into<String>, start: usize, end: usize) -> Token {
    Token {
        kind,
        text: text.into(),
        range: TextRange::new(start, end),
    }
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

struct Parser<'a> {
    source: &'a SourceFile,
    tokens: Vec<Token>,
    lossless_tokens: Vec<Token>,
    cursor: usize,
    diagnostics: Vec<ParseDiagnostic>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a SourceFile, tokens: Vec<Token>) -> Self {
        let parse_tokens = tokens
            .iter()
            .filter(|token| !token.kind.is_trivia())
            .cloned()
            .collect();
        Self {
            source,
            tokens: parse_tokens,
            lossless_tokens: tokens,
            cursor: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse(mut self) -> ParseOutput {
        let module = if self.at(TokenKind::Mod) {
            Some(
                self.parse_named_header(TokenKind::Mod, "module_declaration")
                    .0,
            )
        } else {
            None
        };

        let mut uses = Vec::new();
        while self.at(TokenKind::Use) {
            uses.push(self.parse_named_header(TokenKind::Use, "use_declaration").1);
        }

        let mut items = Vec::new();
        while !self.at(TokenKind::Eof) {
            self.eat_newlines();
            if self.at(TokenKind::Eof) {
                break;
            }
            if self.at(TokenKind::Pub) || self.at(TokenKind::Fn) {
                items.push(SyntaxItem::Function(self.parse_function()));
            } else {
                self.error_current(
                    "parse.expected_item",
                    "expected a function declaration",
                    "module",
                    vec!["pub", "fn"],
                    RecoveryStrategy::SynchronizeToAnchor,
                    Some("fn"),
                );
                self.synchronize_to_item();
            }
        }

        let root = build_lossless_root(
            self.lossless_tokens,
            self.source.len(),
            module.as_ref(),
            &uses,
            &items,
        );

        ParseOutput {
            tree: SyntaxTree {
                root,
                module,
                uses,
                items,
            },
            diagnostics: self.diagnostics,
        }
    }

    fn parse_named_header(
        &mut self,
        keyword: TokenKind,
        context: &'static str,
    ) -> (ModuleDecl, UseDecl) {
        let start = self.expect(keyword, context, vec!["keyword"]).range;
        let name = self.parse_module_name(context);
        let end = self.expect_newline(context).range;
        let span = self.source.span(start.cover(end));
        (
            ModuleDecl {
                name: name.clone(),
                span: span.clone(),
            },
            UseDecl { name, span },
        )
    }

    fn parse_function(&mut self) -> FunctionDecl {
        let start = self.current().range;
        let visibility = if self.eat(TokenKind::Pub).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        self.expect(TokenKind::Fn, "function_declaration", vec!["fn"]);
        let name = self.expect_ident("function_declaration", "function name");
        self.expect(TokenKind::LParen, "function_parameters", vec!["("]);
        let params = self.parse_params();
        self.expect(TokenKind::RParen, "function_parameters", vec![")"]);

        let return_type = self.eat(TokenKind::Arrow).map(|_| {
            self.collect_type_until(
                "function_return",
                &[TokenKind::Effects, TokenKind::Newline, TokenKind::Eof],
            )
        });

        let effects = if self.eat(TokenKind::Effects).is_some() {
            Some(self.parse_effect_list())
        } else {
            None
        };
        self.expect_newline("function_declaration");

        let mut contracts = Vec::new();
        while self.at(TokenKind::Require) || self.at(TokenKind::Ensure) {
            contracts.push(self.parse_contract());
        }

        let mut body = Vec::new();
        let mut end_present = false;
        while !self.at(TokenKind::Eof) {
            self.eat_newlines();
            if self.at(TokenKind::End) {
                self.bump();
                end_present = true;
                if self.at(TokenKind::Newline) {
                    self.bump();
                }
                break;
            }
            if self.at(TokenKind::Eof) {
                break;
            }
            body.push(self.parse_body_line());
        }

        if !end_present {
            self.error_current(
                "parse.expected_end",
                "expected `end` to close function declaration",
                "function_body",
                vec!["end"],
                RecoveryStrategy::CloseBlock,
                Some("end"),
            );
        }

        let end = self.previous().map_or(start, |token| token.range);
        FunctionDecl {
            visibility,
            name,
            params,
            return_type,
            effects,
            contracts,
            body,
            span: self.source.span(start.cover(end)),
            end_present,
        }
    }

    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            let start = self.current().range;
            let name = self.expect_ident("function_parameters", "parameter name");
            let ty = self.eat(TokenKind::Colon).map(|_| {
                self.collect_type_until(
                    "function_parameters",
                    &[TokenKind::Comma, TokenKind::RParen, TokenKind::Eof],
                )
            });
            let end = self.previous().map_or(start, |token| token.range);
            params.push(Param {
                name: name.unwrap_or_default(),
                ty,
                span: self.source.span(start.cover(end)),
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        params
    }

    fn parse_effect_list(&mut self) -> Vec<String> {
        self.expect(TokenKind::LBracket, "effect_declaration", vec!["["]);
        let mut effects = Vec::new();
        while !self.at(TokenKind::RBracket) && !self.at(TokenKind::Eof) {
            if let Some(effect) = self.expect_ident("effect_declaration", "effect name") {
                effects.push(effect);
            }
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::RBracket, "effect_declaration", vec!["]"]);
        effects
    }

    fn parse_contract(&mut self) -> ContractClause {
        let start_token = self.bump();
        let kind = match start_token.kind {
            TokenKind::Require => ContractKind::Require,
            _ => ContractKind::Ensure,
        };
        let (text, end) = self.collect_until_newline();
        ContractClause {
            kind,
            text,
            span: self.source.span(start_token.range.cover(end)),
        }
    }

    fn parse_body_line(&mut self) -> BodyLine {
        let start = self.current().range;
        if self.at(TokenKind::Let) {
            self.bump();
            let name = self.expect_ident("let_statement", "binding name");
            let annotation = if self.eat(TokenKind::Colon).is_some() {
                Some(self.collect_type_until(
                    "let_statement",
                    &[TokenKind::Equal, TokenKind::Newline, TokenKind::Eof],
                ))
            } else {
                None
            };
            self.expect(TokenKind::Equal, "let_statement", vec!["="]);
            let (expr, end) = self.parse_expr_until_newline("let_statement");
            BodyLine::Let {
                name,
                annotation,
                expr,
                span: self.source.span(start.cover(end)),
            }
        } else {
            let (expr, end) = self.parse_expr_until_newline("expression_line");
            BodyLine::Expr {
                expr,
                span: self.source.span(start.cover(end)),
            }
        }
    }

    fn parse_module_name(&mut self, context: &'static str) -> String {
        let mut name = self
            .expect_ident(context, "module name")
            .unwrap_or_else(|| "<missing>".to_string());
        while self.eat(TokenKind::Dot).is_some() {
            if let Some(segment) = self.expect_ident(context, "module name segment") {
                name.push('.');
                name.push_str(&segment);
            }
        }
        name
    }

    fn collect_type_until(&mut self, _context: &'static str, stop: &[TokenKind]) -> String {
        let mut parts = Vec::new();
        let mut depth = 0usize;
        while !self.at(TokenKind::Eof) {
            if depth == 0 && stop.iter().any(|kind| self.at(kind.clone())) {
                break;
            }
            match self.current().kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
            parts.push(self.bump().text);
        }
        parts
            .join(" ")
            .replace(" :: ", "::")
            .replace(" (", "(")
            .replace("( ", "(")
            .replace(" )", ")")
            .replace("[ ", "[")
            .replace(" ]", "]")
            .replace(" ,", ",")
    }

    fn collect_until_newline(&mut self) -> (String, TextRange) {
        let start = self.current().range;
        let mut end = start;
        let mut parts = Vec::new();
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            let token = self.bump();
            end = token.range;
            parts.push(token.text);
        }
        if self.at(TokenKind::Newline) {
            end = self.bump().range;
        }
        (
            parts
                .join(" ")
                .replace(" :: ", "::")
                .replace(" (", "(")
                .replace("( ", "(")
                .replace(" )", ")")
                .replace("[ ", "[")
                .replace(" ]", "]")
                .replace(" ,", ","),
            start.cover(end),
        )
    }

    fn parse_expr_until_newline(&mut self, context: &'static str) -> (Expr, TextRange) {
        let start = self.current().range;
        let mut end = start;
        let mut tokens = Vec::new();
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            let token = self.bump();
            end = token.range;
            if token.kind == TokenKind::Invalid {
                self.diagnostics.push(ParseDiagnostic {
                    id: "parse.invalid_token",
                    message: "invalid token in expression".to_string(),
                    span: Some(self.source.span(token.range)),
                    parser_context: context,
                    unexpected: UnexpectedToken {
                        kind: token.kind.label().to_string(),
                        text: token.text.clone(),
                    },
                    expected: vec!["expression"],
                    recovery: Recovery {
                        strategy: RecoveryStrategy::SkipToken,
                        anchor: Some("newline".to_string()),
                        dropped_token_count: 1,
                    },
                });
            } else {
                tokens.push(token);
            }
        }
        if self.at(TokenKind::Newline) {
            end = self.bump().range;
        }

        let expr = ExprParser::new(self.source, &tokens).parse();
        (expr, start.cover(end))
    }

    fn expect_ident(&mut self, context: &'static str, expected: &'static str) -> Option<String> {
        if self.at(TokenKind::Ident) {
            Some(self.bump().text)
        } else {
            self.error_current(
                "parse.expected_identifier",
                format!("expected {expected}"),
                context,
                vec![expected],
                RecoveryStrategy::InsertToken,
                None,
            );
            None
        }
    }

    fn expect_newline(&mut self, context: &'static str) -> Token {
        if self.at(TokenKind::Newline) {
            self.bump()
        } else if self.at(TokenKind::Eof) {
            self.current().clone()
        } else {
            self.error_current(
                "parse.expected_newline",
                "expected a newline",
                context,
                vec!["newline"],
                RecoveryStrategy::InsertToken,
                None,
            );
            self.current().clone()
        }
    }

    fn expect(
        &mut self,
        kind: TokenKind,
        context: &'static str,
        expected: Vec<&'static str>,
    ) -> Token {
        if self.at(kind) {
            self.bump()
        } else {
            self.error_current(
                "parse.expected_token",
                format!("expected {}", expected.join(" or ")),
                context,
                expected,
                RecoveryStrategy::InsertToken,
                None,
            );
            self.current().clone()
        }
    }

    fn error_current(
        &mut self,
        id: &'static str,
        message: impl Into<String>,
        parser_context: &'static str,
        expected: Vec<&'static str>,
        strategy: RecoveryStrategy,
        anchor: Option<&'static str>,
    ) {
        let current = self.current();
        self.diagnostics.push(ParseDiagnostic {
            id,
            message: message.into(),
            span: Some(self.source.span(current.range)),
            parser_context,
            unexpected: UnexpectedToken {
                kind: current.kind.label().to_string(),
                text: current.text.clone(),
            },
            expected,
            recovery: Recovery {
                strategy,
                anchor: anchor.map(str::to_string),
                dropped_token_count: 0,
            },
        });
    }

    fn synchronize_to_item(&mut self) {
        let start = self.cursor;
        while !self.at(TokenKind::Eof)
            && !self.at(TokenKind::Pub)
            && !self.at(TokenKind::Fn)
            && !self.at(TokenKind::End)
        {
            self.bump();
        }
        if let Some(last) = self.diagnostics.last_mut() {
            last.recovery.dropped_token_count = self.cursor.saturating_sub(start);
        }
    }

    fn eat_newlines(&mut self) {
        while self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump())
        } else {
            None
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn previous(&self) -> Option<&Token> {
        self.cursor
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
    }

    fn bump(&mut self) -> Token {
        let token = self.current().clone();
        if token.kind != TokenKind::Eof {
            self.cursor += 1;
        }
        token
    }
}

struct ExprParser<'a> {
    source: &'a SourceFile,
    tokens: &'a [Token],
    cursor: usize,
}

impl<'a> ExprParser<'a> {
    fn new(source: &'a SourceFile, tokens: &'a [Token]) -> Self {
        Self {
            source,
            tokens,
            cursor: 0,
        }
    }

    fn parse(mut self) -> Expr {
        self.parse_expr(0)
    }

    fn parse_expr(&mut self, min_bp: u8) -> Expr {
        let mut lhs = self.parse_prefix();

        loop {
            if self.at(TokenKind::Question) {
                let token = self.bump();
                lhs = Expr {
                    span: self.source.span(lhs_range(&lhs).cover(token.range)),
                    kind: ExprKind::Try(Box::new(lhs)),
                };
                continue;
            }

            let Some((op, left_bp, right_bp)) = self.current_binary_op() else {
                break;
            };
            if left_bp < min_bp {
                break;
            }
            self.bump();
            let rhs = self.parse_expr(right_bp);
            let span = self.source.span(lhs_range(&lhs).cover(lhs_range(&rhs)));
            lhs = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                },
                span,
            };
        }

        lhs
    }

    fn parse_prefix(&mut self) -> Expr {
        if self.at(TokenKind::Not) || self.at(TokenKind::Minus) {
            let token = self.bump();
            let op = if token.kind == TokenKind::Not {
                PrefixOp::Not
            } else {
                PrefixOp::Negate
            };
            let expr = self.parse_expr(13);
            return Expr {
                span: self.source.span(token.range.cover(lhs_range(&expr))),
                kind: ExprKind::Prefix {
                    op,
                    expr: Box::new(expr),
                },
            };
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        loop {
            if !self.at(TokenKind::LParen) {
                break;
            }
            let start = lhs_range(&expr);
            self.bump();
            let mut args = Vec::new();
            while !self.at(TokenKind::RParen) && !self.is_at_end() {
                args.push(self.parse_expr(0));
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            let end = self.eat(TokenKind::RParen).map_or_else(
                || lhs_range(args.last().unwrap_or(&expr)),
                |token| token.range,
            );
            expr = Expr {
                span: self.source.span(start.cover(end)),
                kind: ExprKind::Call {
                    callee: Box::new(expr),
                    args,
                },
            };
        }
        expr
    }

    fn parse_primary(&mut self) -> Expr {
        let Some(token) = self.tokens.get(self.cursor).cloned() else {
            return self.missing_expr();
        };

        match token.kind {
            TokenKind::Underscore | TokenKind::Hole => {
                self.bump();
                let name = token.text.strip_prefix('_').and_then(|suffix| {
                    if suffix.is_empty() {
                        None
                    } else {
                        Some(suffix.to_string())
                    }
                });
                let satisfy = self.parse_satisfy_clause();
                Expr {
                    kind: ExprKind::Hole { name, satisfy },
                    span: self.source.span(token.range),
                }
            }
            TokenKind::String => {
                self.bump();
                Expr {
                    kind: ExprKind::StringLiteral(token.text),
                    span: self.source.span(token.range),
                }
            }
            TokenKind::Int => {
                self.bump();
                Expr {
                    kind: ExprKind::IntLiteral(token.text),
                    span: self.source.span(token.range),
                }
            }
            TokenKind::Float => {
                self.bump();
                Expr {
                    kind: ExprKind::FloatLiteral(token.text),
                    span: self.source.span(token.range),
                }
            }
            TokenKind::Ident => self.parse_name_path(),
            TokenKind::LParen => {
                let start = self.bump();
                if let Some(end) = self.eat(TokenKind::RParen) {
                    Expr {
                        kind: ExprKind::Unit,
                        span: self.source.span(start.range.cover(end.range)),
                    }
                } else {
                    let expr = self.parse_expr(0);
                    let end = self
                        .eat(TokenKind::RParen)
                        .map_or_else(|| lhs_range(&expr), |token| token.range);
                    Expr {
                        span: self.source.span(start.range.cover(end)),
                        ..expr
                    }
                }
            }
            TokenKind::LBrace => self.parse_record(),
            TokenKind::LBracket => self.parse_list(),
            _ => {
                self.bump();
                Expr {
                    kind: ExprKind::Missing,
                    span: self.source.span(token.range),
                }
            }
        }
    }

    fn parse_satisfy_clause(&mut self) -> Option<SatisfyClause> {
        if !self.at_ident_text("satisfy") {
            return None;
        }
        let start = self.bump().range;
        let candidate = if self.at(TokenKind::Ident) {
            Some(self.bump().text)
        } else {
            None
        };
        let mut end = self
            .eat(TokenKind::FatArrow)
            .map_or(start, |token| token.range);
        let mut parts = Vec::new();
        let mut depth = 0usize;
        while let Some(token) = self.tokens.get(self.cursor) {
            if depth == 0
                && matches!(
                    token.kind,
                    TokenKind::Comma
                        | TokenKind::RParen
                        | TokenKind::RBracket
                        | TokenKind::RBrace
                        | TokenKind::Eof
                )
            {
                break;
            }
            match token.kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
            let token = self.bump();
            end = token.range;
            parts.push(token.text);
        }
        Some(SatisfyClause {
            candidate,
            predicate: normalize_collected_text(parts),
            span: self.source.span(start.cover(end)),
        })
    }

    fn parse_name_path(&mut self) -> Expr {
        let start = self.current().range;
        let mut end = start;
        let mut segments = vec![self.bump().text];
        while self.eat(TokenKind::DoubleColon).is_some() {
            if self.at(TokenKind::Ident) {
                let segment = self.bump();
                end = segment.range;
                segments.push(segment.text);
            } else {
                break;
            }
        }
        Expr {
            kind: ExprKind::NamePath(segments),
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_list(&mut self) -> Expr {
        let start = self.bump().range;
        let mut items = Vec::new();
        while !self.at(TokenKind::RBracket) && !self.is_at_end() {
            items.push(self.parse_expr(0));
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self.eat(TokenKind::RBracket).map_or_else(
            || items.last().map_or(start, lhs_range),
            |token| token.range,
        );
        Expr {
            kind: ExprKind::List(items),
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_record(&mut self) -> Expr {
        let start = self.bump().range;
        let mut fields = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.is_at_end() {
            let field_start = self.current().range;
            let name = if self.at(TokenKind::Ident) {
                self.bump().text
            } else {
                self.bump();
                String::new()
            };
            self.eat(TokenKind::Colon);
            let expr = self.parse_expr(0);
            let field_span = self.source.span(field_start.cover(lhs_range(&expr)));
            fields.push(RecordField {
                name,
                expr,
                span: field_span,
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self.eat(TokenKind::RBrace).map_or_else(
            || {
                fields.last().map_or(start, |field| {
                    TextRange::new(field.span.start.offset, field.span.end.offset)
                })
            },
            |token| token.range,
        );
        Expr {
            kind: ExprKind::Record(fields),
            span: self.source.span(start.cover(end)),
        }
    }

    fn current_binary_op(&self) -> Option<(BinaryOp, u8, u8)> {
        match self.tokens.get(self.cursor)?.kind {
            TokenKind::PipeGreater => Some((BinaryOp::PipeGreater, 1, 2)),
            TokenKind::Or => Some((BinaryOp::Or, 3, 4)),
            TokenKind::And => Some((BinaryOp::And, 5, 6)),
            TokenKind::EqualEqual => Some((BinaryOp::Equal, 7, 8)),
            TokenKind::BangEqual => Some((BinaryOp::NotEqual, 7, 8)),
            TokenKind::Less => Some((BinaryOp::Less, 9, 10)),
            TokenKind::LessEqual => Some((BinaryOp::LessEqual, 9, 10)),
            TokenKind::Greater => Some((BinaryOp::Greater, 9, 10)),
            TokenKind::GreaterEqual => Some((BinaryOp::GreaterEqual, 9, 10)),
            TokenKind::Plus => Some((BinaryOp::Add, 11, 12)),
            TokenKind::Minus => Some((BinaryOp::Subtract, 11, 12)),
            TokenKind::Star => Some((BinaryOp::Multiply, 13, 14)),
            TokenKind::Slash => Some((BinaryOp::Divide, 13, 14)),
            _ => None,
        }
    }

    fn missing_expr(&self) -> Expr {
        Expr {
            kind: ExprKind::Missing,
            span: self.source.span(TextRange::at(self.source.len())),
        }
    }

    fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump())
        } else {
            None
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| token.kind == kind)
    }

    fn at_ident_text(&self, text: &str) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| token.kind == TokenKind::Ident && token.text == text)
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn bump(&mut self) -> Token {
        let token = self.tokens[self.cursor].clone();
        self.cursor += 1;
        token
    }
}

fn lhs_range(expr: &Expr) -> TextRange {
    TextRange::new(expr.span.start.offset, expr.span.end.offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_public_function() {
        let source = SourceFile::new(
            "main.veln",
            "pub fn main() -> Result(Unit, AppError) effects [stdio]\n  Ok(())\nend\n",
        );

        let output = parse(&source);

        assert!(output.diagnostics.is_empty());
        assert_eq!(output.tree.items.len(), 1);
        let SyntaxItem::Function(function) = &output.tree.items[0];
        assert_eq!(function.name.as_deref(), Some("main"));
        assert_eq!(
            function.effects.as_ref().unwrap(),
            &vec!["stdio".to_string()]
        );
        assert!(function.end_present);
    }

    #[test]
    fn parses_omitted_signature_annotations_as_recoverable_ast_facts() {
        let source = SourceFile::new("main.veln", "fn helper(value)\n  value\nend\n");

        let output = parse(&source);

        assert!(output.diagnostics.is_empty());
        let SyntaxItem::Function(function) = &output.tree.items[0];
        assert_eq!(function.params[0].ty, None);
        assert_eq!(function.return_type, None);
        assert_eq!(function.effects, None);
    }

    #[test]
    fn reports_missing_end() {
        let source = SourceFile::new("main.veln", "fn broken() -> Unit\n  _\n");
        let output = parse(&source);

        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "parse.expected_end")
        );
    }

    #[test]
    fn lossless_tree_retains_trivia() {
        let source = SourceFile::new(
            "main.veln",
            "// module comment\nfn id(value: Int) -> Int\n  value // tail comment\nend\n",
        );

        let output = parse(&source);
        let tokens = output.tree.lossless_tokens().collect::<Vec<_>>();

        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::Comment && token.text == "// module comment")
        );
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::Whitespace)
        );
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::Comment && token.text == "// tail comment")
        );
        assert_eq!(output.tree.items.len(), 1);
    }

    #[test]
    fn lossless_tree_groups_declarations_for_formatting() {
        let text = concat!(
            "mod app\n",
            "use stdio\n",
            "pub fn main() -> Unit effects [stdio]\n",
            "  require ready\n",
            "  let message = \"hello\"\n",
            "  stdio::println(message)\n",
            "end\n",
        );
        let source = SourceFile::new("main.veln", text);

        let output = parse(&source);
        let kinds = output
            .tree
            .descendant_nodes()
            .map(|node| node.kind)
            .collect::<Vec<_>>();
        let rendered = output
            .tree
            .lossless_tokens()
            .map(|token| token.text.as_str())
            .collect::<String>();

        assert_eq!(rendered, text);
        assert!(kinds.contains(&SyntaxNodeKind::ModuleDecl));
        assert!(kinds.contains(&SyntaxNodeKind::UseDecl));
        assert!(kinds.contains(&SyntaxNodeKind::FunctionDecl));
        assert!(kinds.contains(&SyntaxNodeKind::FunctionSignature));
        assert!(kinds.contains(&SyntaxNodeKind::ContractClause));
        assert!(kinds.contains(&SyntaxNodeKind::Body));
        assert!(kinds.contains(&SyntaxNodeKind::LetStatement));
        assert!(kinds.contains(&SyntaxNodeKind::ExprLine));
    }

    #[test]
    fn parses_structured_calls_and_holes() {
        let source = SourceFile::new(
            "main.veln",
            "fn main() -> Unit\n  stdio::println(_message)\n  _\nend\n",
        );

        let output = parse(&source);
        let SyntaxItem::Function(function) = &output.tree.items[0];
        let BodyLine::Expr { expr, .. } = &function.body[0] else {
            panic!("expected expression line");
        };

        let ExprKind::Call { callee, args } = &expr.kind else {
            panic!("expected call expression");
        };
        assert!(matches!(
            &callee.kind,
            ExprKind::NamePath(segments) if segments == &vec!["stdio".to_string(), "println".to_string()]
        ));
        assert!(matches!(
            &args[0].kind,
            ExprKind::Hole {
                name: Some(name), ..
            } if name == "message"
        ));

        let BodyLine::Expr { expr, .. } = &function.body[1] else {
            panic!("expected expression line");
        };
        assert!(matches!(&expr.kind, ExprKind::Hole { name: None, .. }));
    }
}
