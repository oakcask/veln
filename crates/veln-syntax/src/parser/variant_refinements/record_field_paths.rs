use super::*;

pub(super) fn assign_record_field_paths(
    tokens: &[Token],
    refinements: &mut [VariantRefinementType],
) {
    if tokens.first().map(|token| token.kind) != Some(TokenKind::LBrace) {
        return;
    }
    let token_indexes = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.range.start, index))
        .collect::<std::collections::HashMap<_, _>>();
    let targets = refinements
        .iter()
        .enumerate()
        .filter_map(|(index, refinement)| {
            token_indexes
                .get(&refinement.span.start.offset)
                .copied()
                .map(|token_index| (token_index, index))
        })
        .collect::<std::collections::HashMap<_, _>>();
    let mut paths = vec![None; refinements.len()];
    RecordPathCollector::new(tokens, &targets, &mut paths).collect();
    for (refinement, path) in refinements.iter_mut().zip(paths) {
        refinement.record_field_path = path;
    }
}

#[derive(Default)]
struct RecordFieldState {
    function_field: bool,
    angle_depth: usize,
    paren_depth: usize,
    square_depth: usize,
    skipped_brace_depth: usize,
}

enum RecordTraversalState {
    Record,
    Field(RecordFieldState),
}

struct RecordPathCollector<'a> {
    tokens: &'a [Token],
    targets: &'a std::collections::HashMap<usize, usize>,
    paths: &'a mut [Option<RecordFieldPath>],
    cursor: usize,
    path: Vec<String>,
    previous_path_len: usize,
    minimum_depth_since_target: usize,
    states: Vec<RecordTraversalState>,
}

impl<'a> RecordPathCollector<'a> {
    fn new(
        tokens: &'a [Token],
        targets: &'a std::collections::HashMap<usize, usize>,
        paths: &'a mut [Option<RecordFieldPath>],
    ) -> Self {
        Self {
            tokens,
            targets,
            paths,
            cursor: 1,
            path: Vec::new(),
            previous_path_len: 0,
            minimum_depth_since_target: usize::MAX,
            states: vec![RecordTraversalState::Record],
        }
    }

    fn collect(mut self) {
        while let Some(state) = self.states.pop() {
            let complete = match state {
                RecordTraversalState::Record => self.visit_record_start(),
                RecordTraversalState::Field(field) => self.visit_record_field(field),
            };
            if !complete {
                return;
            }
        }
    }

    fn visit_record_start(&mut self) -> bool {
        skip_type_trivia(self.tokens, &mut self.cursor);
        if self.tokens.get(self.cursor).map(|token| token.kind) == Some(TokenKind::RBrace) {
            self.cursor += 1;
            return true;
        }
        let Some(token) = self.tokens.get(self.cursor) else {
            return false;
        };
        self.path.push(token.text.clone());
        self.cursor += 1;
        skip_type_trivia(self.tokens, &mut self.cursor);
        if self.tokens.get(self.cursor).map(|token| token.kind) != Some(TokenKind::Colon) {
            return false;
        }
        self.cursor += 1;
        skip_type_trivia(self.tokens, &mut self.cursor);
        self.states
            .push(RecordTraversalState::Field(RecordFieldState {
                function_field: self.tokens.get(self.cursor).map(|token| token.kind)
                    == Some(TokenKind::Fn),
                ..RecordFieldState::default()
            }));
        true
    }

    fn visit_record_field(&mut self, mut field: RecordFieldState) -> bool {
        let Some(token) = self.tokens.get(self.cursor) else {
            return false;
        };
        let kind = token.kind;
        self.retain_refinement_path(&field);
        self.advance_field(kind, &mut field);
        true
    }

    fn retain_refinement_path(&mut self, field: &RecordFieldState) {
        if field.function_field
            || field.angle_depth != 0
            || field.paren_depth != 0
            || field.square_depth != 0
        {
            return;
        }
        let Some(index) = self.targets.get(&self.cursor) else {
            return;
        };
        let shared_prefix_len = self
            .previous_path_len
            .min(self.path.len())
            .min(self.minimum_depth_since_target);
        self.paths[*index] = Some(RecordFieldPath {
            shared_prefix_len,
            suffix: self.path[shared_prefix_len..].to_vec(),
        });
        self.previous_path_len = self.path.len();
        self.minimum_depth_since_target = usize::MAX;
    }

    fn advance_field(&mut self, kind: TokenKind, field: &mut RecordFieldState) {
        match kind {
            TokenKind::LBrace
                if !field.function_field && field.angle_depth == 0 && field.paren_depth == 0 =>
            {
                self.cursor += 1;
                self.states
                    .push(RecordTraversalState::Field(std::mem::take(field)));
                self.states.push(RecordTraversalState::Record);
                return;
            }
            TokenKind::LBrace if field.function_field => field.skipped_brace_depth += 1,
            TokenKind::RBrace if field.function_field && field.skipped_brace_depth > 0 => {
                field.skipped_brace_depth -= 1;
            }
            TokenKind::Less => field.angle_depth += 1,
            TokenKind::Greater | TokenKind::ShiftRight | TokenKind::ShiftRightLogical => {
                field.angle_depth = field.angle_depth.saturating_sub(closing_angle_count(kind));
            }
            TokenKind::LParen => field.paren_depth += 1,
            TokenKind::RParen if field.paren_depth > 0 => field.paren_depth -= 1,
            TokenKind::LBracket => field.square_depth += 1,
            TokenKind::RBracket if field.square_depth > 0 => field.square_depth -= 1,
            TokenKind::Comma | TokenKind::RBrace
                if field.angle_depth == 0
                    && field.paren_depth == 0
                    && field.square_depth == 0
                    && field.skipped_brace_depth == 0 =>
            {
                self.finish_field(kind);
                return;
            }
            _ => {}
        }
        self.cursor += 1;
        self.states
            .push(RecordTraversalState::Field(std::mem::take(field)));
    }

    fn finish_field(&mut self, kind: TokenKind) {
        self.cursor += 1;
        self.path.pop();
        self.minimum_depth_since_target = self.minimum_depth_since_target.min(self.path.len());
        if kind == TokenKind::Comma {
            self.states.push(RecordTraversalState::Record);
        }
    }
}

fn skip_type_trivia(tokens: &[Token], cursor: &mut usize) {
    while tokens.get(*cursor).is_some_and(|token| {
        matches!(
            token.kind,
            TokenKind::Whitespace | TokenKind::Comment | TokenKind::Newline
        )
    }) {
        *cursor += 1;
    }
}
