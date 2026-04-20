use crate::transpiler::compiler::ast::{Analysis, Delimiter, Group, SyntaxNode};
use crate::transpiler::compiler::lexer::{Token, TokenKind};
use crate::transpiler::error::TranspileError;

pub fn generate(analysis: &Analysis) -> Result<String, TranspileError> {
    lower_sequence(&analysis.module.nodes, ParentContext::root())
}

#[derive(Clone, Copy)]
struct ParentContext<'a> {
    delimiter: Option<Delimiter>,
    before_open: Option<&'a str>,
}

impl<'a> ParentContext<'a> {
    fn root() -> Self {
        Self {
            delimiter: None,
            before_open: None,
        }
    }

    fn child(delimiter: Delimiter, before_open: Option<&'a str>) -> Self {
        Self {
            delimiter: Some(delimiter),
            before_open,
        }
    }
}

fn lower_sequence(nodes: &[SyntaxNode], parent: ParentContext<'_>) -> Result<String, TranspileError> {
    let mut result = String::new();
    let mut index = 0usize;

    while index < nodes.len() {
        if let Some((end, rewritten)) = lower_function(nodes, index)? {
            push_fragment(&mut result, &rewritten);
            index = end;
            continue;
        }

        if let Some((end, rewritten)) = lower_string_binding(nodes, index, parent)? {
            push_fragment(&mut result, &rewritten);
            index = end;
            continue;
        }

        if let Some((end, rewritten)) = lower_static_string(nodes, index, parent)? {
            push_fragment(&mut result, &rewritten);
            index = end;
            continue;
        }

        if let Some((end, rewritten)) = lower_hash_map_assignment(nodes, index, parent)? {
            push_fragment(&mut result, &rewritten);
            index = end;
            continue;
        }

        if let Some((end, rewritten)) = lower_iterator_chain(nodes, index, parent)? {
            push_fragment(&mut result, &rewritten);
            index = end;
            continue;
        }

        match &nodes[index] {
            SyntaxNode::Token(token) => {
                let text = lower_token(nodes, index, token, parent)?;
                push_fragment(&mut result, &text);
            }
            SyntaxNode::Group(group) => {
                let text = lower_group(nodes, index, group, parent)?;
                push_fragment(&mut result, &text);
            }
        }
        index += 1;
    }

    Ok(result)
}

fn lower_function(nodes: &[SyntaxNode], start: usize) -> Result<Option<(usize, String)>, TranspileError> {
    if token_text_at(nodes, start) != Some("fn") {
        return Ok(None);
    }

    let Some(body_index) = find_next_brace_group(nodes, start) else {
        return Ok(None);
    };
    let SyntaxNode::Group(body) = &nodes[body_index] else {
        return Ok(None);
    };

    let signature = lower_sequence(&nodes[start..body_index], ParentContext::root())?;
    let returns_result = signature.contains("-> Result<");
    let body_inner = lower_sequence(
        &body.nodes,
        ParentContext::child(Delimiter::Brace, previous_token_text(nodes, body_index)),
    )?;
    let body_inner = if returns_result {
        wrap_result_tail_expression(&body_inner)
    } else {
        body_inner
    };
    Ok(Some((
        body_index + 1,
        format!("{} {{ {} }}", signature.trim_end(), body_inner.trim()),
    )))
}

fn lower_string_binding(
    nodes: &[SyntaxNode],
    start: usize,
    parent: ParentContext<'_>,
) -> Result<Option<(usize, String)>, TranspileError> {
    if token_text_at(nodes, start) != Some("let") {
        return Ok(None);
    }
    let Some(end) = find_statement_end(nodes, start) else {
        return Ok(None);
    };
    let statement = &nodes[start..end];
    let Some(colon) = find_top_level_token(statement, ":") else {
        return Ok(None);
    };
    let Some(eq) = find_top_level_token(statement, "=") else {
        return Ok(None);
    };
    if colon >= eq || !is_string_type(&statement[colon + 1..eq]) {
        return Ok(None);
    }

    let lhs = lower_sequence(&statement[..eq + 1], parent)?;
    let rhs = lower_sequence(&statement[eq + 1..statement.len().saturating_sub(1)], parent)?;
    let rhs = if should_wrap_string_binding(&rhs) {
        format!("String::from({})", rhs.trim())
    } else {
        rhs.trim().to_string()
    };
    Ok(Some((end, format!("{} {} ;", lhs.trim_end(), rhs))))
}

fn lower_static_string(
    nodes: &[SyntaxNode],
    start: usize,
    parent: ParentContext<'_>,
) -> Result<Option<(usize, String)>, TranspileError> {
    if token_text_at(nodes, start) != Some("static") {
        return Ok(None);
    }
    let Some(end) = find_statement_end(nodes, start) else {
        return Ok(None);
    };
    let statement = &nodes[start..end];
    let Some(colon) = find_top_level_token(statement, ":") else {
        return Ok(None);
    };
    let Some(eq) = find_top_level_token(statement, "=") else {
        return Ok(None);
    };
    if colon >= eq || !is_string_type(&statement[colon + 1..eq]) {
        return Ok(None);
    }
    if statement.len() != eq + 3 {
        return Ok(None);
    }
    let Some(Token { kind: TokenKind::String, text, .. }) = as_token(&statement[eq + 1]) else {
        return Ok(None);
    };

    let lhs = lower_sequence(&statement[..colon + 1], parent)?;
    Ok(Some((end, format!("{} &str = {} ;", lhs.trim_end(), text))))
}

fn lower_hash_map_assignment(
    nodes: &[SyntaxNode],
    start: usize,
    parent: ParentContext<'_>,
) -> Result<Option<(usize, String)>, TranspileError> {
    let Some(Token { kind: TokenKind::Ident, text: ident, .. }) = as_token(&nodes[start]) else {
        return Ok(None);
    };
    let Some(SyntaxNode::Group(key_group)) = nodes.get(start + 1) else {
        return Ok(None);
    };
    if key_group.delimiter != Delimiter::Bracket || token_text_at(nodes, start + 2) != Some("=") {
        return Ok(None);
    }
    let end = find_expression_end(nodes, start + 3);
    let key = lower_sequence(
        &key_group.nodes,
        ParentContext::child(Delimiter::Bracket, Some(ident.as_str())),
    )?;
    let value = lower_sequence(&nodes[start + 3..end], parent)?;
    Ok(Some((
        end,
        format!("{}.insert({}, {})", ident, key.trim(), value.trim()),
    )))
}

fn lower_iterator_chain(
    nodes: &[SyntaxNode],
    start: usize,
    parent: ParentContext<'_>,
) -> Result<Option<(usize, String)>, TranspileError> {
    if !is_expression_start(nodes, start) {
        return Ok(None);
    }

    let Some((receiver_end, first_call)) = find_first_helper_call(nodes, start)? else {
        return Ok(None);
    };
    let mut receiver = lower_sequence(&nodes[start..receiver_end], parent)?;
    let iter_kind = iterator_kind(&receiver);
    if matches!(iter_kind, IteratorKind::NeedsIter) {
        receiver.push_str(".iter()");
    }

    let mut rewritten = receiver;
    let mut call = Some(first_call);
    let mut end = receiver_end;
    let mut last_kind = HelperKind::Find;
    while let Some(current) = call {
        let argument = lower_helper_argument(&current.argument, current.kind, iter_kind)?;
        rewritten.push_str(current.kind.rust_name());
        rewritten.push('(');
        rewritten.push_str(argument.trim());
        rewritten.push(')');
        end = current.end;
        last_kind = current.kind;
        call = parse_helper_call(nodes, current.end)?;
    }
    if matches!(last_kind, HelperKind::Map | HelperKind::Filter) {
        rewritten.push_str(".collect()");
    }

    Ok(Some((end, rewritten)))
}

fn lower_token(
    nodes: &[SyntaxNode],
    index: usize,
    token: &Token,
    parent: ParentContext<'_>,
) -> Result<String, TranspileError> {
    match token.kind {
        TokenKind::Colon if should_emit_return_arrow(nodes, index) => Ok("->".to_string()),
        TokenKind::Ident if token.text == "string" => Ok("String".to_string()),
        TokenKind::String => lower_string_literal(nodes, index, token, parent),
        TokenKind::Dot if matches_view_call(nodes, index) => Ok(".as_str".to_string()),
        TokenKind::Ident if token.text == "view" && matches_view_call(nodes, index.saturating_sub(1)) => {
            Ok(String::new())
        }
        _ => Ok(token.text.clone()),
    }
}

fn lower_group(
    nodes: &[SyntaxNode],
    index: usize,
    group: &Group,
    _parent: ParentContext<'_>,
) -> Result<String, TranspileError> {
    let before_open = previous_token_text(nodes, index);
    let inner = lower_sequence(&group.nodes, ParentContext::child(group.delimiter, before_open))?;
    match group.delimiter {
        Delimiter::Paren => Ok(format!("({})", inner)),
        Delimiter::Bracket if should_rewrite_vec(nodes, index, group) => Ok(format!("vec![{}]", inner)),
        Delimiter::Bracket => Ok(format!("[{}]", inner)),
        Delimiter::Brace if should_rewrite_map(nodes, index, group) => lower_map_literal(group, before_open),
        Delimiter::Brace => Ok(format!("{{ {} }}", inner.trim())),
    }
}

fn lower_map_literal(group: &Group, before_open: Option<&str>) -> Result<String, TranspileError> {
    if group.nodes.is_empty() {
        return Ok("std::collections::HashMap::new()".to_string());
    }
    let _ = before_open;
    let entries = split_top_level(&group.nodes, ",")?
        .into_iter()
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let pair = split_top_level(entry, ":")?;
            let key = pair.first().copied().unwrap_or(&[]);
            let value = pair.get(1).copied().unwrap_or(&[]);
            Ok(format!(
                "({}, {})",
                lower_sequence(key, ParentContext::root())?.trim(),
                lower_sequence(value, ParentContext::root())?.trim()
            ))
        })
        .collect::<Result<Vec<_>, TranspileError>>()?;
    Ok(format!(
        "std::collections::HashMap::from([{}])",
        entries.join(", ")
    ))
}

fn lower_string_literal(
    nodes: &[SyntaxNode],
    index: usize,
    token: &Token,
    parent: ParentContext<'_>,
) -> Result<String, TranspileError> {
    if should_keep_raw_string(nodes, index, parent) {
        return Ok(token.text.clone());
    }

    let content = &token.text[1..token.text.len() - 1];
    let interpolations = extract_interpolations(content, token.span.start + 1)?;
    if interpolations.is_empty() {
        return Ok(format!("String::from({})", token.text));
    }

    let mut format_string = String::new();
    let mut arguments = Vec::with_capacity(interpolations.len());
    let mut cursor = 0usize;
    for interpolation in interpolations {
        format_string.push_str(&escape_format_segment(&content[cursor..interpolation.start]));
        format_string.push_str("{}");
        arguments.push(compile_embedded_expression(
            &content[interpolation.start + 1..interpolation.end - 1],
            token.span.start + 1 + interpolation.start + 1,
        )?);
        cursor = interpolation.end;
    }
    format_string.push_str(&escape_format_segment(&content[cursor..]));
    Ok(format!("format!(\"{}\", {})", format_string, arguments.join(", ")))
}

fn compile_embedded_expression(source: &str, offset: usize) -> Result<String, TranspileError> {
    let tokens = crate::transpiler::compiler::lexer::lex(source)?;
    let adjusted = tokens
        .into_iter()
        .map(|mut token| {
            token.span = (token.span.start + offset)..(token.span.end + offset);
            token
        })
        .collect::<Vec<_>>();
    let module = crate::transpiler::compiler::parser::parse(&adjusted)?;
    let analysis = crate::transpiler::compiler::semantics::analyze(module)?;
    generate(&analysis)
}

fn should_keep_raw_string(nodes: &[SyntaxNode], index: usize, parent: ParentContext<'_>) -> bool {
    let prev = previous_token_text(nodes, index);
    if matches!(prev, Some("=") | Some(":")) {
        return false;
    }
    if matches!(prev, Some("!") | Some("&")) {
        return true;
    }

    match (prev, parent.delimiter, parent.before_open) {
        (None, Some(Delimiter::Paren), Some(before)) => {
            before == "!" || is_identifier_like(before) || matches!(before, ")" | "]")
        }
        (Some("("), Some(Delimiter::Paren), Some(before)) => {
            before == "!" || is_identifier_like(before) || matches!(before, ")" | "]")
        }
        (Some(","), Some(Delimiter::Paren), Some(before)) => {
            before == "!" || is_identifier_like(before) || matches!(before, ")" | "]")
        }
        (Some(","), Some(Delimiter::Bracket), Some(before)) => {
            is_identifier_like(before) || matches!(before, ")" | "]" | "\"")
        }
        (Some("["), Some(Delimiter::Bracket), Some(before)) => {
            is_identifier_like(before) || matches!(before, ")" | "]")
        }
        _ => false,
    }
}

fn extract_interpolations(source: &str, base_offset: usize) -> Result<Vec<Interpolation>, TranspileError> {
    let bytes = source.as_bytes();
    let mut items = Vec::new();
    let mut index = 0usize;

    while index < bytes.len() {
        match bytes[index] as char {
            '\\' => index += 2,
            '{' => {
                let start = index;
                let mut depth = 1usize;
                index += 1;
                while index < bytes.len() {
                    match bytes[index] as char {
                        '\\' => index += 2,
                        '{' => {
                            depth += 1;
                            index += 1;
                        }
                        '}' => {
                            depth -= 1;
                            index += 1;
                            if depth == 0 {
                                items.push(Interpolation { start, end: index });
                                break;
                            }
                        }
                        _ => index += 1,
                    }
                }
                if depth != 0 {
                    return Err(TranspileError::diagnostic(
                        "unclosed string interpolation",
                        (base_offset + start)..(base_offset + source.len()),
                    ));
                }
            }
            _ => index += 1,
        }
    }

    Ok(items)
}

fn escape_format_segment(segment: &str) -> String {
    segment.replace('{', "{{").replace('}', "}}")
}

fn should_emit_return_arrow(nodes: &[SyntaxNode], index: usize) -> bool {
    matches!(previous_token_text(nodes, index), Some(")") | Some("|"))
}

fn matches_view_call(nodes: &[SyntaxNode], index: usize) -> bool {
    token_text_at(nodes, index) == Some(".")
        && token_text_at(nodes, index + 1) == Some("view")
        && matches!(nodes.get(index + 2), Some(SyntaxNode::Group(group)) if group.delimiter == Delimiter::Paren && group.nodes.is_empty())
}

fn should_rewrite_vec(nodes: &[SyntaxNode], index: usize, group: &Group) -> bool {
    previous_token_text(nodes, index).is_none_or(is_expression_delimiter)
        && !contains_top_level_token(&group.nodes, ";")
}

fn should_rewrite_map(nodes: &[SyntaxNode], index: usize, group: &Group) -> bool {
    previous_token_text(nodes, index).is_none_or(is_expression_delimiter)
        && (group.nodes.is_empty() || contains_top_level_token(&group.nodes, ":"))
}

fn contains_top_level_token(nodes: &[SyntaxNode], token: &str) -> bool {
    nodes.iter().any(|node| matches!(node, SyntaxNode::Token(current) if current.text == token))
}

fn is_expression_delimiter(token: &str) -> bool {
    matches!(token, "=" | "(" | "," | "{" | "[" | ";")
}

fn find_next_brace_group(nodes: &[SyntaxNode], start: usize) -> Option<usize> {
    (start..nodes.len()).find(|index| matches!(nodes.get(*index), Some(SyntaxNode::Group(group)) if group.delimiter == Delimiter::Brace))
}

fn find_statement_end(nodes: &[SyntaxNode], start: usize) -> Option<usize> {
    (start..nodes.len())
        .find(|index| token_text_at(nodes, *index) == Some(";"))
        .map(|index| index + 1)
}

fn find_expression_end(nodes: &[SyntaxNode], start: usize) -> usize {
    (start..nodes.len())
        .find(|index| matches!(token_text_at(nodes, *index), Some(";" | ",")))
        .unwrap_or(nodes.len())
}

fn find_top_level_token(nodes: &[SyntaxNode], token: &str) -> Option<usize> {
    nodes.iter()
        .position(|node| matches!(node, SyntaxNode::Token(current) if current.text == token))
}

fn split_top_level<'a>(nodes: &'a [SyntaxNode], delimiter: &str) -> Result<Vec<&'a [SyntaxNode]>, TranspileError> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    for (index, node) in nodes.iter().enumerate() {
        if matches!(node, SyntaxNode::Token(current) if current.text == delimiter) {
            parts.push(&nodes[start..index]);
            start = index + 1;
        }
    }
    parts.push(&nodes[start..]);
    Ok(parts)
}

fn is_string_type(nodes: &[SyntaxNode]) -> bool {
    nodes.len() == 1
        && matches!(nodes.first(), Some(SyntaxNode::Token(token)) if token.text == "String" || token.text == "string")
}

fn should_wrap_string_binding(rhs: &str) -> bool {
    let trimmed = rhs.trim();
    !(trimmed.starts_with("String::from(")
        || trimmed.starts_with("format!(")
        || trimmed.ends_with(".to_string()")
        || trimmed == "String::new()")
}

fn wrap_result_tail_expression(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut segments = trimmed.rsplitn(2, ';');
    let tail = segments.next().unwrap_or_default().trim();
    let prefix = segments.next().unwrap_or_default().trim();
    if tail.is_empty()
        || trimmed.ends_with(';')
        || tail.starts_with("return")
        || tail.starts_with("Ok(")
        || tail.starts_with("Err(")
    {
        return trimmed.to_string();
    }
    if prefix.is_empty() {
        format!("Ok({})", tail)
    } else {
        format!("{}; Ok({})", prefix, tail)
    }
}

fn find_first_helper_call(nodes: &[SyntaxNode], start: usize) -> Result<Option<(usize, HelperCall)>, TranspileError> {
    for index in start..nodes.len() {
        match token_text_at(nodes, index) {
            Some(";" | ",") => return Ok(None),
            Some(".") => {
                if let Some(call) = parse_helper_call(nodes, index)? {
                    return Ok(Some((index, call)));
                }
            }
            _ => {}
        }
    }
    Ok(None)
}

fn parse_helper_call(nodes: &[SyntaxNode], dot_index: usize) -> Result<Option<HelperCall>, TranspileError> {
    if token_text_at(nodes, dot_index) != Some(".") {
        return Ok(None);
    }
    let Some(name) = token_text_at(nodes, dot_index + 1) else {
        return Ok(None);
    };
    let Some(kind) = HelperKind::from_name(name) else {
        return Ok(None);
    };
    let Some(SyntaxNode::Group(argument)) = nodes.get(dot_index + 2) else {
        return Ok(None);
    };
    if argument.delimiter != Delimiter::Paren {
        return Ok(None);
    }
    Ok(Some(HelperCall {
        kind,
        argument: argument.nodes.clone(),
        end: dot_index + 3,
    }))
}

fn lower_helper_argument(
    argument: &[SyntaxNode],
    kind: HelperKind,
    iter_kind: IteratorKind,
) -> Result<String, TranspileError> {
    if let Some(arrow) = find_top_level_token(argument, "=>") {
        let param = normalize_closure_param(&lower_sequence(&argument[..arrow], ParentContext::root())?);
        let body = lower_sequence(&argument[arrow + 1..], ParentContext::root())?;
        let body = rewrite_closure_body(&body, &param, kind.deref_count(iter_kind));
        return Ok(format!("|{}| {}", param, body.trim()));
    }
    lower_sequence(argument, ParentContext::root())
}

fn normalize_closure_param(param: &str) -> String {
    let trimmed = param.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') && trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].trim().to_string()
    } else {
        trimmed.to_string()
    }
}

fn rewrite_closure_body(body: &str, param: &str, deref_count: usize) -> String {
    if deref_count == 0 || !is_identifier_like(param) {
        return body.to_string();
    }
    body.replace(param, &format!("({}{})", "*".repeat(deref_count), param))
}

fn iterator_kind(receiver: &str) -> IteratorKind {
    let trimmed = receiver.trim_end();
    if trimmed.ends_with(".into_iter()") {
        IteratorKind::IntoIter
    } else if trimmed.ends_with(".iter()") || trimmed.ends_with(".iter_mut()") {
        IteratorKind::Iter
    } else {
        IteratorKind::NeedsIter
    }
}

fn is_expression_start(nodes: &[SyntaxNode], index: usize) -> bool {
    match nodes.get(index) {
        Some(SyntaxNode::Token(token)) => {
            matches!(token.kind, TokenKind::Ident)
                && previous_token_text(nodes, index).is_none_or(is_expression_boundary)
        }
        Some(SyntaxNode::Group(group)) => {
            group.delimiter == Delimiter::Paren
                && previous_token_text(nodes, index).is_none_or(is_expression_boundary)
        }
        None => false,
    }
}

fn is_expression_boundary(token: &str) -> bool {
    matches!(
        token,
        "="
            | "("
            | "["
            | "{"
            | ","
            | ";"
            | ":"
            | "+"
            | "-"
            | "*"
            | "/"
            | "%"
            | "!"
            | "&"
            | "|"
            | "^"
            | "<"
            | ">"
            | "?"
    )
}

fn previous_token_text(nodes: &[SyntaxNode], index: usize) -> Option<&str> {
    nodes[..index].iter().rev().find_map(|node| match node {
        SyntaxNode::Token(token) => Some(token.text.as_str()),
        SyntaxNode::Group(group) => Some(match group.delimiter {
            Delimiter::Paren => ")",
            Delimiter::Bracket => "]",
            Delimiter::Brace => "}",
        }),
    })
}

fn token_text_at(nodes: &[SyntaxNode], index: usize) -> Option<&str> {
    match nodes.get(index) {
        Some(SyntaxNode::Token(token)) => Some(token.text.as_str()),
        Some(SyntaxNode::Group(group)) => Some(match group.delimiter {
            Delimiter::Paren => "(",
            Delimiter::Bracket => "[",
            Delimiter::Brace => "{",
        }),
        None => None,
    }
}

fn as_token(node: &SyntaxNode) -> Option<&Token> {
    match node {
        SyntaxNode::Token(token) => Some(token),
        SyntaxNode::Group(_) => None,
    }
}

fn is_identifier_like(token: &str) -> bool {
    token.chars().next().is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        && token
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn push_fragment(buffer: &mut String, fragment: &str) {
    if fragment.is_empty() {
        return;
    }
    if buffer.is_empty() {
        buffer.push_str(fragment);
        return;
    }
    let prev = buffer.chars().next_back();
    let next = fragment.chars().next();
    if prev.zip(next).is_some_and(|(prev, next)| needs_space(prev, next)) {
        buffer.push(' ');
    }
    buffer.push_str(fragment);
}

fn needs_space(prev: char, next: char) -> bool {
    ((prev == '_' || prev.is_ascii_alphanumeric()) && (next == '_' || next.is_ascii_alphanumeric()))
        || (matches!(prev, ')' | ']' | '}') && (next == '_' || next.is_ascii_alphanumeric()))
        || ((prev == '_' || prev.is_ascii_alphanumeric()) && next == '{')
}

#[derive(Clone)]
struct Interpolation {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy)]
enum HelperKind {
    Map,
    Filter,
    Find,
    FindIndex,
}

impl HelperKind {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "map" => Some(Self::Map),
            "filter" => Some(Self::Filter),
            "find" => Some(Self::Find),
            "findIndex" => Some(Self::FindIndex),
            _ => None,
        }
    }

    fn rust_name(self) -> &'static str {
        match self {
            Self::Map => ".map",
            Self::Filter => ".filter",
            Self::Find => ".find",
            Self::FindIndex => ".position",
        }
    }

    fn deref_count(self, iter_kind: IteratorKind) -> usize {
        let item_refs = match iter_kind {
            IteratorKind::IntoIter => 0,
            IteratorKind::Iter | IteratorKind::NeedsIter => 1,
        };
        item_refs
            + match self {
                Self::Filter | Self::Find => 1,
                Self::Map | Self::FindIndex => 0,
            }
    }
}

struct HelperCall {
    kind: HelperKind,
    argument: Vec<SyntaxNode>,
    end: usize,
}

#[derive(Clone, Copy)]
enum IteratorKind {
    NeedsIter,
    Iter,
    IntoIter,
}
