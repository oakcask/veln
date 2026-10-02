use super::*;

impl<'a> ExprParser<'a> {
    pub(super) fn parse_name_path(&mut self) -> Expr {
        let start = self.current().range;
        let mut end = start;
        let first = self.bump();
        let mut segments = vec![first.text];
        let mut segment_spans = vec![self.source.span(first.range)];
        while self.eat(TokenKind::DoubleColon).is_some() {
            if self.at_contextual_identifier() || self.at(TokenKind::Decode) {
                let segment = self.bump();
                end = segment.range;
                segment_spans.push(self.source.span(segment.range));
                segments.push(segment.text);
            } else {
                break;
            }
        }
        if let Some(value) = bare_expression_bool_literal(&segments) {
            return Expr {
                kind: ExprKind::BoolLiteral(value),
                span: self.source.span(start.cover(end)),
            };
        }
        Expr {
            kind: ExprKind::NamePath {
                segments,
                segment_spans,
            },
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_name_path_segments(
        &mut self,
        context: &'static str,
        expected_name: &'static str,
    ) -> Vec<String> {
        let mut segments = Vec::new();
        if self.at_contextual_identifier() {
            segments.push(self.bump().text);
        } else {
            self.error_current(
                "parse.name_path",
                format!("{context} is missing {expected_name}"),
                vec![expected_name],
                RecoveryStrategy::InsertToken,
                None,
            );
        }
        while self.eat(TokenKind::DoubleColon).is_some() {
            if self.at_contextual_identifier() {
                segments.push(self.bump().text);
            } else {
                self.error_current(
                    "parse.name_path",
                    format!("{context} has an incomplete path"),
                    vec!["path segment"],
                    RecoveryStrategy::InsertToken,
                    None,
                );
                break;
            }
        }
        segments
    }
}
