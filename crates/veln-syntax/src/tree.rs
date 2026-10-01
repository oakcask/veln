use veln_source::{SourceSpan, TextRange};

use crate::{
    BodyLine, EffectDecl, Expr, ExprChildren, ExprKind, FunctionDecl, HandlerDecl, ModuleDecl,
    PublicAliasDecl, SchemaDecl, SyntaxItem, Token, TokenKind, TypeDecl, UseDecl,
};

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
    EffectDecl,
    HandlerDecl,
    HandlerOperationClause,
    TypeDecl,
    SchemaDecl,
    PublicAliasDecl,
    FunctionSignature,
    ContractClause,
    Body,
    LetStatement,
    ExprLine,
    DeferStatement,
    BeginExpr,
    CleanupBody,
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

    for node in collect_top_level_nodes(module, uses, items) {
        push_tokens_before(&tokens, &mut cursor, node.range().start, &mut children);
        let node_tokens = take_tokens_in_range(&tokens, &mut cursor, node.range());
        children.push(SyntaxElement::Node(node.into_syntax_node(node_tokens)));
    }

    push_remaining_tokens(&tokens, &mut cursor, &mut children);
    SyntaxNode::root(children, source_len)
}

fn collect_top_level_nodes<'a>(
    module: Option<&ModuleDecl>,
    uses: &[UseDecl],
    items: &'a [SyntaxItem],
) -> Vec<TopLevelNode<'a>> {
    let mut nodes = Vec::new();
    if let Some(module) = module {
        nodes.push(TopLevelNode::Module(span_range(&module.span)));
    }
    nodes.extend(
        uses.iter()
            .map(|use_decl| TopLevelNode::Use(span_range(&use_decl.span))),
    );
    nodes.extend(items.iter().map(TopLevelNode::from));
    nodes.sort_by_key(|node| node.range().start);
    nodes
}

enum TopLevelNode<'a> {
    Module(TextRange),
    Use(TextRange),
    Function(&'a FunctionDecl),
    Effect(&'a EffectDecl),
    Handler(&'a HandlerDecl),
    Type(&'a TypeDecl),
    Schema(&'a SchemaDecl),
    PublicAlias(&'a PublicAliasDecl),
}

impl<'a> From<&'a SyntaxItem> for TopLevelNode<'a> {
    fn from(item: &'a SyntaxItem) -> Self {
        match item {
            SyntaxItem::Function(function) => Self::Function(function),
            SyntaxItem::Effect(effect) => Self::Effect(effect),
            SyntaxItem::Handler(handler) => Self::Handler(handler),
            SyntaxItem::Type(type_decl) => Self::Type(type_decl),
            SyntaxItem::Schema(schema) => Self::Schema(schema),
            SyntaxItem::PublicAlias(alias) => Self::PublicAlias(alias),
        }
    }
}

impl TopLevelNode<'_> {
    fn range(&self) -> TextRange {
        match self {
            Self::Module(range) | Self::Use(range) => *range,
            Self::Function(function) => span_range(&function.span),
            Self::Effect(effect) => span_range(&effect.span),
            Self::Handler(handler) => span_range(&handler.span),
            Self::Type(type_decl) => span_range(&type_decl.span),
            Self::Schema(schema) => span_range(&schema.span),
            Self::PublicAlias(alias) => span_range(&alias.span),
        }
    }

    fn into_syntax_node(self, tokens: &[Token]) -> SyntaxNode {
        let range = self.range();
        match self {
            Self::Module(_) => token_node(SyntaxNodeKind::ModuleDecl, range, tokens),
            Self::Use(_) => token_node(SyntaxNodeKind::UseDecl, range, tokens),
            Self::Function(function) => build_lossless_function(function, tokens),
            Self::Effect(_) => token_node(SyntaxNodeKind::EffectDecl, range, tokens),
            Self::Handler(handler) => build_lossless_handler(handler, tokens),
            Self::Type(_) => token_node(SyntaxNodeKind::TypeDecl, range, tokens),
            Self::Schema(_) => token_node(SyntaxNodeKind::SchemaDecl, range, tokens),
            Self::PublicAlias(_) => token_node(SyntaxNodeKind::PublicAliasDecl, range, tokens),
        }
    }
}

fn build_lossless_handler(handler: &HandlerDecl, tokens: &[Token]) -> SyntaxNode {
    let range = span_range(&handler.span);
    let mut children = Vec::new();
    let mut cursor = 0usize;
    for clause in &handler.operation_clauses {
        let clause_range = span_range(&clause.span);
        push_tokens_before(tokens, &mut cursor, clause_range.start, &mut children);
        let clause_tokens = take_tokens_in_range(tokens, &mut cursor, clause_range);
        children.push(SyntaxElement::Node(build_expr_container_node(
            SyntaxNodeKind::HandlerOperationClause,
            clause_range,
            clause_tokens,
            &clause.body,
        )));
    }
    push_remaining_tokens(tokens, &mut cursor, &mut children);
    SyntaxNode::new(SyntaxNodeKind::HandlerDecl, range, children)
}

fn build_lossless_function(function: &FunctionDecl, tokens: &[Token]) -> SyntaxNode {
    let range = span_range(&function.span);
    let mut children = Vec::new();
    let mut cursor = 0;

    if let Some(signature_end) = tokens
        .iter()
        .find(|token| token.kind == TokenKind::Newline)
        .map(|token| token.range.end)
    {
        let signature_tokens = take_tokens_in_range(
            tokens,
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
        push_tokens_before(tokens, &mut cursor, contract_range.start, &mut children);
        let contract_tokens = take_tokens_in_range(tokens, &mut cursor, contract_range);
        children.push(SyntaxElement::Node(token_node(
            SyntaxNodeKind::ContractClause,
            contract_range,
            contract_tokens,
        )));
    }

    if !function.body.is_empty() {
        let mut body_children = Vec::new();
        for line in &function.body {
            let line_range = body_line_range(line);
            push_body_tokens_before(tokens, &mut cursor, line_range.start, &mut body_children);
            let line_tokens = take_tokens_in_range(tokens, &mut cursor, line_range);
            body_children.push(SyntaxElement::Node(build_body_line_node(line, line_tokens)));
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

    push_remaining_tokens(tokens, &mut cursor, &mut children);
    SyntaxNode::new(SyntaxNodeKind::FunctionDecl, range, children)
}

fn build_body_line_node(line: &BodyLine, tokens: &[Token]) -> SyntaxNode {
    match line {
        BodyLine::Let { expr, span, .. } => {
            build_expr_container_node(SyntaxNodeKind::LetStatement, span_range(span), tokens, expr)
        }
        BodyLine::Expr { expr, span } => {
            build_expr_container_node(SyntaxNodeKind::ExprLine, span_range(span), tokens, expr)
        }
        BodyLine::Defer {
            body,
            block_span,
            span,
            ..
        } => build_cleanup_region_node(
            SyntaxNodeKind::DeferStatement,
            span_range(span),
            span_range(block_span),
            tokens,
            body,
        ),
    }
}

fn build_expr_container_node(
    kind: SyntaxNodeKind,
    range: TextRange,
    tokens: &[Token],
    expr: &Expr,
) -> SyntaxNode {
    let mut begin_exprs = Vec::new();
    collect_outer_begin_exprs(expr, &mut begin_exprs);
    begin_exprs.sort_by_key(|expr| expr.span.start.offset);
    let mut children = Vec::new();
    let mut cursor = 0usize;
    for begin in begin_exprs {
        let begin_range = span_range(&begin.span);
        push_tokens_before(tokens, &mut cursor, begin_range.start, &mut children);
        let begin_tokens = take_tokens_in_range(tokens, &mut cursor, begin_range);
        children.push(SyntaxElement::Node(build_begin_expr_node(
            begin,
            begin_tokens,
        )));
    }
    push_remaining_tokens(tokens, &mut cursor, &mut children);
    SyntaxNode::new(kind, range, children)
}

fn build_begin_expr_node(expr: &Expr, tokens: &[Token]) -> SyntaxNode {
    let ExprKind::Begin { body, block_span } = &expr.kind else {
        unreachable!("begin-expression node requires begin expression")
    };
    build_cleanup_region_node(
        SyntaxNodeKind::BeginExpr,
        span_range(&expr.span),
        span_range(block_span),
        tokens,
        body,
    )
}

fn build_cleanup_region_node(
    kind: SyntaxNodeKind,
    range: TextRange,
    block_range: TextRange,
    tokens: &[Token],
    body: &[BodyLine],
) -> SyntaxNode {
    let mut children = Vec::new();
    let mut cursor = 0usize;
    push_tokens_before(tokens, &mut cursor, block_range.start, &mut children);
    let block_tokens = take_tokens_in_range(tokens, &mut cursor, block_range);
    children.push(SyntaxElement::Node(build_cleanup_body_node(
        block_range,
        block_tokens,
        body,
    )));
    push_remaining_tokens(tokens, &mut cursor, &mut children);
    SyntaxNode::new(kind, range, children)
}

fn build_cleanup_body_node(range: TextRange, tokens: &[Token], body: &[BodyLine]) -> SyntaxNode {
    let mut children = Vec::new();
    let mut cursor = 0usize;
    for line in body {
        let line_range = body_line_range(line);
        push_tokens_before(tokens, &mut cursor, line_range.start, &mut children);
        let line_tokens = take_tokens_in_range(tokens, &mut cursor, line_range);
        children.push(SyntaxElement::Node(build_body_line_node(line, line_tokens)));
    }
    push_remaining_tokens(tokens, &mut cursor, &mut children);
    SyntaxNode::new(SyntaxNodeKind::CleanupBody, range, children)
}

fn body_line_range(line: &BodyLine) -> TextRange {
    match line {
        BodyLine::Let { span, .. } | BodyLine::Expr { span, .. } | BodyLine::Defer { span, .. } => {
            span_range(span)
        }
    }
}

fn collect_outer_begin_exprs<'a>(expr: &'a Expr, begins: &mut Vec<&'a Expr>) {
    let mut pending = vec![expr];
    while let Some(expr) = pending.pop() {
        match expr.children() {
            ExprChildren::BeginBody(_) => begins.push(expr),
            ExprChildren::One { child, .. } => pending.push(child),
            ExprChildren::Pair { first, second, .. } => {
                pending.push(first);
                pending.push(second);
            }
            ExprChildren::Slice(children) => pending.extend(children),
            ExprChildren::HeadAndSlice(head, children) => {
                pending.push(head);
                pending.extend(children);
            }
            ExprChildren::Record(fields) => {
                pending.extend(fields.iter().map(|field| &field.expr));
            }
            ExprChildren::Dict(entries) => {
                for entry in entries {
                    pending.push(&entry.key);
                    pending.push(&entry.value);
                }
            }
            ExprChildren::Match(scrutinee, arms) => {
                pending.push(scrutinee);
                pending.extend(arms.iter().map(|arm| &arm.expr));
            }
            ExprChildren::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                pending.push(condition);
                pending.push(then_branch);
                for branch in else_if_branches {
                    pending.push(&branch.condition);
                    pending.push(&branch.expr);
                }
                pending.push(else_branch);
            }
            ExprChildren::None => {}
        }
    }
}

fn token_node(kind: SyntaxNodeKind, range: TextRange, tokens: &[Token]) -> SyntaxNode {
    SyntaxNode::new(
        kind,
        range,
        tokens.iter().cloned().map(SyntaxElement::Token).collect(),
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

fn take_tokens_in_range<'a>(
    tokens: &'a [Token],
    cursor: &mut usize,
    range: TextRange,
) -> &'a [Token] {
    let start = *cursor;
    while tokens
        .get(*cursor)
        .is_some_and(|token| token.range.start >= range.start && token.range.end <= range.end)
    {
        *cursor += 1;
    }
    &tokens[start..*cursor]
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
