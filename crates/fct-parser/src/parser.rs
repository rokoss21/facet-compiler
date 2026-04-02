use crate::error::{ParseResult, SpanInput};
use fct_ast::{
    BodyNode, DirectiveNode, FacetBlock, FacetDocument, FacetNode, FunctionSignature, KeyValueNode,
    LensCallNode, ListItemNode, MapKeyKind, OrderedMap, Parameter, PipelineNode, ScalarValue, Span,
    TypeNode, ValueNode,
};
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while, take_while_m_n},
    character::complete::{char, digit1, line_ending, multispace0, none_of, space0, space1},
    combinator::{all_consuming, eof, map, map_res, opt, recognize, value},
    multi::{many0, separated_list0, separated_list1},
    sequence::{delimited, pair, preceded, terminated, tuple},
};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

// --- Helper Functions ---

fn to_span(input: SpanInput) -> Span {
    Span {
        start: input.location_offset(),
        end: input.location_offset() + input.fragment().len(),
        line: input.location_line() as usize,
        column: input.get_utf8_column(),
    }
}

fn comment(input: SpanInput) -> ParseResult<SpanInput> {
    recognize(pair(char('#'), is_not("\n\r")))(input)
}

fn eol(input: SpanInput) -> ParseResult<SpanInput> {
    alt((line_ending, eof_as_str))(input)
}

fn eof_as_str(input: SpanInput) -> ParseResult<SpanInput> {
    recognize(eof)(input)
}

// Matches exactly N spaces
fn indentation(level: usize) -> impl Fn(SpanInput) -> ParseResult<SpanInput> {
    move |input: SpanInput| {
        let (input, spaces) = take_while(|c| c == ' ')(input)?;
        if spaces.fragment().len() == level * 2 {
            Ok((input, spaces))
        } else {
            Err(nom::Err::Error(nom::error::VerboseError {
                errors: vec![(
                    input,
                    nom::error::VerboseErrorKind::Nom(nom::error::ErrorKind::Tag),
                )],
            }))
        }
    }
}

// Consumes empty lines and comments
fn empty_lines(input: SpanInput) -> ParseResult<SpanInput> {
    recognize(many0(alt((
        value((), line_ending),
        value((), pair(space0, line_ending)),
        value((), pair(space0, pair(comment, line_ending))),
    ))))(input)
}

// --- Primitive Parsers ---

fn identifier(input: SpanInput) -> ParseResult<String> {
    let is_ident_start = |c: char| c.is_ascii_alphabetic() || c == '_';
    let is_ident_char = |c: char| c.is_ascii_alphanumeric() || c == '_';

    map(
        recognize(pair(
            take_while_m_n(1, 1, is_ident_start),
            take_while(is_ident_char),
        )),
        |s: SpanInput| s.fragment().to_string(),
    )(input)
}

fn string_literal(input: SpanInput) -> ParseResult<String> {
    let (input, _) = char('"')(input)?;
    let mut collected = String::new();
    let mut rest = input;
    loop {
        // Lookahead for closing quote
        if let Ok((after, _)) = char::<_, nom::error::VerboseError<SpanInput>>('"')(rest) {
            // found closing quote
            return Ok((after, collected));
        }

        // Check for escape sequences
        if let Ok((after, _)) = char::<_, nom::error::VerboseError<SpanInput>>('\\')(rest) {
            if let Ok((after_u, _)) = char::<_, nom::error::VerboseError<SpanInput>>('u')(after) {
                let (after_hex, hex_digits) =
                    take_while_m_n(4, 4, |c: char| c.is_ascii_hexdigit())(after_u)?;
                let code_point = u32::from_str_radix(hex_digits.fragment(), 16).map_err(|_| {
                    nom::Err::Error(nom::error::VerboseError {
                        errors: vec![(
                            rest,
                            nom::error::VerboseErrorKind::Nom(nom::error::ErrorKind::Escaped),
                        )],
                    })
                })?;
                let ch = char::from_u32(code_point).ok_or_else(|| {
                    nom::Err::Error(nom::error::VerboseError {
                        errors: vec![(
                            rest,
                            nom::error::VerboseErrorKind::Nom(nom::error::ErrorKind::Escaped),
                        )],
                    })
                })?;
                collected.push(ch);
                rest = after_hex;
                continue;
            }

            // Found escape, parse the escaped character
            if let Ok((after2, ch)) = alt::<_, _, nom::error::VerboseError<SpanInput>, _>((
                value('"', char('"')),
                value('\\', char('\\')),
                value('\n', char('n')),
                value('\r', char('r')),
                value('\t', char('t')),
            ))(after)
            {
                collected.push(ch);
                rest = after2;
                continue;
            }
        }

        // Take one non-quote, non-backslash char
        let (after, ch) = none_of::<_, _, nom::error::VerboseError<SpanInput>>("\"\\")(rest)?;
        collected.push(ch);
        rest = after;
    }
}

fn boolean(input: SpanInput) -> ParseResult<bool> {
    alt((value(true, tag("true")), value(false, tag("false"))))(input)
}

fn null(input: SpanInput) -> ParseResult<()> {
    value((), tag("null"))(input)
}

fn integer(input: SpanInput) -> ParseResult<i64> {
    map_res(recognize(pair(opt(char('-')), digit1)), |s: SpanInput| {
        s.fragment().parse::<i64>()
    })(input)
}

fn float(input: SpanInput) -> ParseResult<f64> {
    use nom::character::complete::{char, one_of};
    use nom::combinator::recognize;
    use nom::sequence::tuple;

    // Float pattern: -?[digits].[digits] optionally followed by e[+-]?[digits]
    let float_pattern = recognize(tuple((
        opt(char('-')),
        digit1,
        char('.'),
        digit1,
        opt(tuple((one_of("eE"), opt(one_of("+-")), digit1))),
    )));

    map_res(float_pattern, |s: SpanInput| s.fragment().parse::<f64>())(input)
}

fn scalar_value(input: SpanInput) -> ParseResult<ScalarValue> {
    alt((
        map(boolean, ScalarValue::Bool),
        map(null, |_| ScalarValue::Null),
        map(float, ScalarValue::Float), // Float must be before Int
        map(integer, ScalarValue::Int),
    ))(input)
}

fn type_node(input: SpanInput) -> ParseResult<TypeNode> {
    parse_union_type(input)
}

fn parse_union_type(input: SpanInput) -> ParseResult<TypeNode> {
    let (mut input, first) = parse_primary_type(input)?;
    let mut members = vec![first];

    loop {
        let (after_ws, _) = multispace0(input)?;
        if let Ok((after_bar, _)) = char::<_, nom::error::VerboseError<SpanInput>>('|')(after_ws) {
            let (after_bar, _) = multispace0(after_bar)?;
            let (after_ty, next) = parse_primary_type(after_bar)?;
            members.push(next);
            input = after_ty;
        } else {
            input = after_ws;
            break;
        }
    }

    if members.len() == 1 {
        Ok((input, members.remove(0)))
    } else {
        Ok((input, TypeNode::Union(members)))
    }
}

fn parse_primary_type(input: SpanInput) -> ParseResult<TypeNode> {
    alt((
        parse_struct_type,
        parse_list_type,
        parse_map_type,
        parse_embedding_type,
        parse_image_type,
        parse_audio_type,
        map(identifier, TypeNode::Primitive),
    ))(input)
}

fn parse_list_type(input: SpanInput) -> ParseResult<TypeNode> {
    let (input, _) = tag("list")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('<')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, item_ty) = type_node(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('>')(input)?;
    Ok((input, TypeNode::List(Box::new(item_ty))))
}

fn parse_map_type(input: SpanInput) -> ParseResult<TypeNode> {
    let (input, _) = tag("map")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('<')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = alt((tag("string"), tag("String")))(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(',')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, value_ty) = type_node(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('>')(input)?;
    Ok((input, TypeNode::Map(Box::new(value_ty))))
}

fn parse_struct_type(input: SpanInput) -> ParseResult<TypeNode> {
    let (input, _) = tag("struct")(input)?;
    let (mut input, _) = multispace0(input)?;
    let (i, _) = char('{')(input)?;
    input = i;

    let mut fields = OrderedMap::new();
    loop {
        let (after_ws, _) = multispace0(input)?;
        if let Ok((after_close, _)) = char::<_, nom::error::VerboseError<SpanInput>>('}')(after_ws)
        {
            return Ok((after_close, TypeNode::Struct(fields)));
        }

        let (after_name, field_name) = identifier(after_ws)?;
        let (after_name, _) = multispace0(after_name)?;
        let (after_colon, _) = char(':')(after_name)?;
        let (after_colon, _) = multispace0(after_colon)?;
        let (after_ty, field_ty) = type_node(after_colon)?;
        fields.insert(field_name, field_ty);

        let (after_sep_ws, _) = multispace0(after_ty)?;
        if let Ok((after_comma, _)) =
            char::<_, nom::error::VerboseError<SpanInput>>(',')(after_sep_ws)
        {
            input = after_comma;
        } else {
            input = after_sep_ws;
        }
    }
}

fn parse_embedding_type(input: SpanInput) -> ParseResult<TypeNode> {
    let (input, _) = tag("embedding")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('<')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("size")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, size_digits) = digit1(input)?;
    let size = size_digits.fragment().parse::<usize>().map_err(|_| {
        nom::Err::Error(nom::error::VerboseError {
            errors: vec![(
                input,
                nom::error::VerboseErrorKind::Nom(nom::error::ErrorKind::Digit),
            )],
        })
    })?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('>')(input)?;
    Ok((input, TypeNode::Embedding { size }))
}

fn parse_image_type(input: SpanInput) -> ParseResult<TypeNode> {
    let (input, _) = tag("image")(input)?;
    parse_media_constraints(input, true)
}

fn parse_audio_type(input: SpanInput) -> ParseResult<TypeNode> {
    let (input, _) = tag("audio")(input)?;
    parse_media_constraints(input, false)
}

fn parse_media_constraints(input: SpanInput, image: bool) -> ParseResult<TypeNode> {
    let (input, _) = multispace0(input)?;
    if let Ok((mut cursor, _)) = char::<_, nom::error::VerboseError<SpanInput>>('(')(input) {
        let mut format: Option<String> = None;
        let mut max_dim: Option<u32> = None;
        let mut max_duration: Option<f64> = None;

        loop {
            let (after_ws, _) = multispace0(cursor)?;
            if let Ok((after_close, _)) =
                char::<_, nom::error::VerboseError<SpanInput>>(')')(after_ws)
            {
                return if image {
                    Ok((after_close, TypeNode::Image { max_dim, format }))
                } else {
                    Ok((
                        after_close,
                        TypeNode::Audio {
                            max_duration,
                            format,
                        },
                    ))
                };
            }

            let (after_key, key) = identifier(after_ws)?;
            let (after_key, _) = multispace0(after_key)?;
            let (after_eq, _) = char('=')(after_key)?;
            let (after_eq, _) = multispace0(after_eq)?;

            if key == "format" {
                let (after_value, fmt) = alt((string_literal, identifier))(after_eq)?;
                format = Some(fmt);
                cursor = after_value;
            } else if image && key == "max_dim" {
                let (after_value, digits) = digit1(after_eq)?;
                if let Ok(parsed) = digits.fragment().parse::<u32>() {
                    max_dim = Some(parsed);
                }
                cursor = after_value;
            } else if !image && key == "max_duration" {
                let (after_value, duration) =
                    alt((map(float, |f| f), map(integer, |i| i as f64)))(after_eq)?;
                max_duration = Some(duration);
                cursor = after_value;
            } else {
                let (after_value, _) = alt((
                    map(string_literal, |_| ()),
                    map(float, |_| ()),
                    map(integer, |_| ()),
                    map(identifier, |_| ()),
                ))(after_eq)?;
                cursor = after_value;
            }

            let (after_sep_ws, _) = multispace0(cursor)?;
            if let Ok((after_comma, _)) =
                char::<_, nom::error::VerboseError<SpanInput>>(',')(after_sep_ws)
            {
                cursor = after_comma;
            } else {
                cursor = after_sep_ws;
            }
        }
    }

    if image {
        Ok((
            input,
            TypeNode::Image {
                max_dim: None,
                format: None,
            },
        ))
    } else {
        Ok((
            input,
            TypeNode::Audio {
                max_duration: None,
                format: None,
            },
        ))
    }
}

// --- Value Parsers ---

fn variable_ref(input: SpanInput) -> ParseResult<String> {
    let (input, _) = char('$')(input)?;
    let (input, parts) = separated_list1(char('.'), identifier)(input)?;
    let name = parts.join(".");
    Ok((input, name))
}

// Directives like @input(...)
fn directive(input: SpanInput) -> ParseResult<DirectiveNode> {
    let start = input;
    let (input, _) = char('@')(input)?;
    let (input, name) = identifier(input)?;
    let (input, _) = space0(input)?;
    let (input, args) = opt(attributes)(input)?;

    Ok((
        input,
        DirectiveNode {
            name,
            args: args.unwrap_or_default(),
            span: to_span(start),
        },
    ))
}

// Parse lens arguments: positional and named
fn lens_args(input: SpanInput) -> ParseResult<(Vec<ValueNode>, OrderedMap<String, ValueNode>)> {
    let (input, _) = space0(input)?;

    // Empty args case - check for ')' but DON'T consume it (lens_call will consume it)
    if char::<_, nom::error::VerboseError<SpanInput>>(')')(input).is_ok() {
        return Ok((input, (vec![], OrderedMap::new())));
    }

    let mut positional = vec![];
    let mut named = OrderedMap::new();
    let mut input = input;

    loop {
        let (inp, _) = space0(input)?;

        // Try named argument (key=value)
        if let Ok((inp2, key)) = identifier(inp) {
            let (inp3, _) = space0(inp2)?;
            if let Ok((inp4, _)) = char::<_, nom::error::VerboseError<SpanInput>>('=')(inp3) {
                let (inp5, _) = space0(inp4)?;
                let (inp6, value) = parse_value_simple(inp5)?;
                named.insert(key, value);
                input = inp6;

                // Check for comma or end
                let (inp6, _) = space0(input)?;
                if let Ok((inp7, _)) = char::<_, nom::error::VerboseError<SpanInput>>(',')(inp6) {
                    input = inp7;
                    continue;
                } else {
                    input = inp6;
                    break;
                }
            }
        }

        // Try positional argument
        let (inp2, value) = parse_value_simple(inp)?;
        positional.push(value);
        input = inp2;

        // Check for comma or end
        let (inp3, _) = space0(input)?;
        if let Ok((inp4, _)) = char::<_, nom::error::VerboseError<SpanInput>>(',')(inp3) {
            input = inp4;
            continue;
        } else {
            input = inp3;
            break;
        }
    }

    Ok((input, (positional, named)))
}

// Simple value parser (no pipeline support to avoid recursion issues)
fn parse_value_simple(input: SpanInput) -> ParseResult<ValueNode> {
    alt((
        map(scalar_value, ValueNode::Scalar),
        map(string_literal, ValueNode::String),
        map(variable_ref, ValueNode::Variable),
        map(directive, ValueNode::Directive),
    ))(input)
}

fn lens_call(input: SpanInput) -> ParseResult<LensCallNode> {
    let start = input;
    let (input, name) = identifier(input)?;
    let (input, _) = char('(')(input)?;
    let (input, (args, kwargs)) = lens_args(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    Ok((
        input,
        LensCallNode {
            name,
            args,
            kwargs,
            span: to_span(start),
        },
    ))
}

fn list_literal(input: SpanInput) -> ParseResult<ValueNode> {
    let (input, _) = char('[')(input)?;
    let (input, _) = multispace0(input)?;
    // Allow separation by comma or newline, with arbitrary whitespace
    let separator = alt((
        value((), delimited(multispace0, char(','), multispace0)),
        value((), line_ending),
    ));
    let (input, items) = separated_list0(separator, preceded(multispace0, parse_value))(input)?;
    let (input, _) = multispace0(input)?;
    if input.fragment().starts_with(',') {
        return Err(nom::Err::Error(nom::error::VerboseError {
            errors: vec![(
                input,
                nom::error::VerboseErrorKind::Nom(nom::error::ErrorKind::SeparatedList),
            )],
        }));
    }
    let (input, _) = char(']')(input)?;
    Ok((input, ValueNode::List(items)))
}

fn map_literal(input: SpanInput) -> ParseResult<ValueNode> {
    let (input, _) = char('{')(input)?;
    let (input, _) = multispace0(input)?;
    // Support comma or newline separated entries (newline more common in FACET)
    let separator = alt((
        value((), delimited(multispace0, char(','), multispace0)),
        value((), line_ending),
    ));
    let (input, entries) =
        separated_list0(separator, preceded(multispace0, map_key_value_pair))(input)?;
    let (input, _) = multispace0(input)?;
    if input.fragment().starts_with(',') {
        return Err(nom::Err::Error(nom::error::VerboseError {
            errors: vec![(
                input,
                nom::error::VerboseErrorKind::Nom(nom::error::ErrorKind::SeparatedList),
            )],
        }));
    }
    let (input, _) = char('}')(input)?;

    let mut map = OrderedMap::new();
    for (key, value) in entries {
        map.insert(key, value);
    }
    Ok((input, ValueNode::Map(map)))
}

fn attribute_string_literal(input: SpanInput) -> ParseResult<String> {
    let (input, s) = string_literal(input)?;
    if s.contains("{{") || s.contains("}}") {
        return Err(nom::Err::Error(nom::error::VerboseError {
            errors: vec![(
                input,
                nom::error::VerboseErrorKind::Nom(nom::error::ErrorKind::Tag),
            )],
        }));
    }
    Ok((input, s))
}

fn parse_attribute_atom(input: SpanInput) -> ParseResult<ValueNode> {
    alt((
        map(scalar_value, ValueNode::Scalar),
        map(attribute_string_literal, ValueNode::String),
        map(variable_ref, ValueNode::Variable),
    ))(input)
}

// Helper to match only horizontal whitespace (spaces and tabs, NOT newlines)
fn horizontal_space(input: SpanInput) -> ParseResult<SpanInput> {
    take_while(|c| c == ' ' || c == '\t')(input)
}

fn parse_value(input: SpanInput) -> ParseResult<ValueNode> {
    // Parse a base value first
    let (input, base) = alt((
        map_literal,
        list_literal,
        map(scalar_value, ValueNode::Scalar),
        map(string_literal, ValueNode::String),
        map(variable_ref, ValueNode::Variable),
        map(directive, ValueNode::Directive),
    ))(input)?;

    // Optionally parse a pipeline tail
    // Use horizontal_space to consume only spaces/tabs, NOT newlines
    let (input, lenses) = many0(preceded(
        tuple((horizontal_space, tag("|>"), horizontal_space)),
        lens_call,
    ))(input)?;

    if lenses.is_empty() {
        Ok((input, base))
    } else {
        Ok((
            input,
            ValueNode::Pipeline(PipelineNode {
                initial: Box::new(base),
                lenses,
                span: to_span(input), // Span is approximate here
            }),
        ))
    }
}

// --- Body Parsing ---

fn map_key(input: SpanInput) -> ParseResult<(String, MapKeyKind)> {
    alt((
        map(identifier, |s| (s, MapKeyKind::Identifier)),
        map(string_literal, |s| (s, MapKeyKind::String)),
    ))(input)
}

fn key_value(input: SpanInput) -> ParseResult<KeyValueNode> {
    let (input, (key, key_kind)) = map_key(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = space0(input)?;
    let (input, value) = parse_value(input)?;

    Ok((
        input,
        KeyValueNode {
            key,
            key_kind,
            value,
            span: to_span(input), // Approx
        },
    ))
}

fn list_item(input: SpanInput) -> ParseResult<ListItemNode> {
    let (input, _) = char('-')(input)?;
    let (input, _) = space1(input)?;
    let (input, value) = parse_value(input)?;

    Ok((
        input,
        ListItemNode {
            value,
            span: to_span(input), // Approx
        },
    ))
}

fn body_line(input: SpanInput) -> ParseResult<BodyNode> {
    alt((
        map(key_value, BodyNode::KeyValue),
        map(list_item, BodyNode::ListItem),
    ))(input)
}

fn attributes(input: SpanInput) -> ParseResult<OrderedMap<String, ValueNode>> {
    delimited(
        char('('),
        map(
            separated_list0(
                preceded(space0, char(',')),
                preceded(space0, attribute_key_value_pair),
            ),
            |pairs| pairs.into_iter().collect(),
        ),
        char(')'),
    )(input)
}

// Helper for map entries where key may be identifier or string
fn map_key_value_pair(input: SpanInput) -> ParseResult<(String, ValueNode)> {
    let (input, (key, _key_kind)) = map_key(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = alt((char(':'), char('=')))(input)?;
    let (input, _) = space0(input)?;
    let (input, value) = parse_value(input)?;
    Ok((input, (key, value)))
}

// Helper for attributes where key MUST be identifier
fn attribute_key_value_pair(input: SpanInput) -> ParseResult<(String, ValueNode)> {
    let (input, key) = identifier(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = alt((char(':'), char('=')))(input)?;
    let (input, _) = space0(input)?;
    let (input, value) = parse_attribute_atom(input)?;
    Ok((input, (key, value)))
}

fn block_body(level: usize) -> impl Fn(SpanInput) -> ParseResult<Vec<BodyNode>> {
    move |input: SpanInput| {
        many0(preceded(
            pair(empty_lines, indentation(level)),
            terminated(body_line, eol),
        ))(input)
    }
}

fn body_nodes_to_value(body: &[BodyNode]) -> ValueNode {
    let has_kv = body.iter().any(|n| matches!(n, BodyNode::KeyValue(_)));
    let has_list = body.iter().any(|n| matches!(n, BodyNode::ListItem(_)));

    match (has_kv, has_list) {
        (true, false) => {
            let mut map = OrderedMap::new();
            for node in body {
                if let BodyNode::KeyValue(kv) = node {
                    map.insert(kv.key.clone(), kv.value.clone());
                }
            }
            ValueNode::Map(map)
        }
        (false, true) => {
            let mut items = Vec::new();
            for node in body {
                if let BodyNode::ListItem(item) = node {
                    items.push(item.value.clone());
                }
            }
            ValueNode::List(items)
        }
        (true, true) => {
            let mut map = OrderedMap::new();
            let mut items = Vec::new();
            for node in body {
                match node {
                    BodyNode::KeyValue(kv) => {
                        map.insert(kv.key.clone(), kv.value.clone());
                    }
                    BodyNode::ListItem(item) => items.push(item.value.clone()),
                }
            }
            map.insert("__items".to_string(), ValueNode::List(items));
            ValueNode::Map(map)
        }
        (false, false) => ValueNode::Map(OrderedMap::new()),
    }
}

fn test_block_body(level: usize) -> impl Fn(SpanInput) -> ParseResult<Vec<BodyNode>> {
    move |mut input: SpanInput| {
        let mut sections = Vec::new();

        loop {
            let (after_empty, _) = empty_lines(input)?;
            input = after_empty;

            let section_start = input;
            let (after_indent, _) = match indentation(level)(input) {
                Ok(ok) => ok,
                Err(_) => break,
            };
            let (after_key, key) = identifier(after_indent)?;
            let (after_key, _) = space0(after_key)?;
            let (after_colon, _) = char(':')(after_key)?;
            let (after_colon, _) = space0(after_colon)?;

            let (next_input, value) = if let Ok((after_eol, _)) = eol(after_colon) {
                let (after_nested, nested_body) = if key == "mock" {
                    mock_section_body(level + 1)(after_eol)?
                } else {
                    block_body(level + 1)(after_eol)?
                };
                (after_nested, body_nodes_to_value(&nested_body))
            } else {
                let (after_value, value) = parse_value(after_colon)?;
                let (after_value, _) = space0(after_value)?;
                let (after_value, _) = eol(after_value)?;
                (after_value, value)
            };

            sections.push(BodyNode::KeyValue(KeyValueNode {
                key,
                key_kind: MapKeyKind::Identifier,
                value,
                span: to_span(section_start),
            }));
            input = next_input;
        }

        Ok((input, sections))
    }
}

fn mock_key(input: SpanInput) -> ParseResult<String> {
    if let Ok((rest, quoted)) = string_literal(input) {
        return Ok((rest, quoted));
    }

    let (rest, raw) = take_while(|c| c != ':' && c != '\n' && c != '\r')(input)?;
    let key = raw.fragment().trim().to_string();
    if key.is_empty() {
        return Err(nom::Err::Error(nom::error::VerboseError {
            errors: vec![(
                input,
                nom::error::VerboseErrorKind::Nom(nom::error::ErrorKind::Tag),
            )],
        }));
    }
    Ok((rest, key))
}

fn mock_section_body(level: usize) -> impl Fn(SpanInput) -> ParseResult<Vec<BodyNode>> {
    move |mut input: SpanInput| {
        let mut entries = Vec::new();

        loop {
            let (after_empty, _) = empty_lines(input)?;
            input = after_empty;

            let row_start = input;
            let (after_indent, _) = match indentation(level)(input) {
                Ok(ok) => ok,
                Err(_) => break,
            };
            let (after_key, key) = mock_key(after_indent)?;
            let (after_key, _) = space0(after_key)?;
            let (after_colon, _) = char(':')(after_key)?;
            let (after_colon, _) = space0(after_colon)?;
            let (after_value, value) = parse_value(after_colon)?;
            let (after_value, _) = space0(after_value)?;
            let (next_input, _) = eol(after_value)?;

            entries.push(BodyNode::KeyValue(KeyValueNode {
                key,
                key_kind: MapKeyKind::Identifier,
                value,
                span: to_span(row_start),
            }));
            input = next_input;
        }

        Ok((input, entries))
    }
}

// Parse a single function parameter with span tracking
fn function_param(input: SpanInput) -> ParseResult<Parameter> {
    let param_start = input;
    let (input, _) = space0(input)?;
    let (input, pname) = identifier(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = space0(input)?;
    let (input, ptype) = type_node(input)?;
    let (input, _) = space0(input)?;

    Ok((
        input,
        Parameter {
            name: pname,
            type_node: ptype,
            span: to_span(param_start),
        },
    ))
}

// Parse function signature inside @interface
fn interface_fn(level: usize) -> impl Fn(SpanInput) -> ParseResult<FunctionSignature> {
    move |input: SpanInput| {
        let fn_start = input;
        let (input, _) = indentation(level)(input)?;
        let (input, _) = tag("fn")(input)?;
        let (input, _) = space1(input)?;
        let (input, name) = identifier(input)?;
        let (input, _) = space0(input)?;
        let (input, _) = char('(')(input)?;

        // Parse parameters with proper span tracking
        let (input, params) = separated_list0(preceded(space0, char(',')), function_param)(input)?;

        let (input, _) = char(')')(input)?;
        let (input, _) = space0(input)?;
        let (input, _) = tag("->")(input)?;
        let (input, _) = space0(input)?;
        let (input, return_type) = type_node(input)?;
        let (input, _) = space0(input)?;
        let (input, fn_attrs) = opt(attributes)(input)?;
        let effect = fn_attrs
            .as_ref()
            .and_then(|attrs| attrs.get("effect"))
            .and_then(|v| match v {
                ValueNode::String(s) => Some(s.clone()),
                _ => None,
            });
        let (input, _) = eol(input)?;

        Ok((
            input,
            FunctionSignature {
                name,
                params,
                return_type,
                effect,
                span: to_span(fn_start),
            },
        ))
    }
}

fn interface_body(level: usize) -> impl Fn(SpanInput) -> ParseResult<Vec<FunctionSignature>> {
    move |input: SpanInput| many0(preceded(empty_lines, interface_fn(level)))(input)
}

// --- Block Parsing ---

fn facet_block(input: SpanInput, level: usize) -> ParseResult<FacetNode> {
    let (input, start_pos) = nom_locate::position(input)?;
    let (input, _) = char('@')(input)?;
    let (input, name) = identifier(input)?;
    let (input, _) = space0(input)?;

    // Special-case @import "path"
    if name == "import" {
        let (input, _) = space0(input)?;
        let (input, path_value) = parse_value(input)?;
        let (input, _) = space0(input)?;
        let (input, _) = eol(input)?;

        // Extract path string (validation happens in TypeChecker)
        let path = match path_value {
            ValueNode::String(s) => s,
            _ => {
                // For non-string imports, use empty string and let validator catch it
                String::new()
            }
        };

        let (input, end_pos) = nom_locate::position(input)?;
        let span = Span {
            start: start_pos.location_offset(),
            end: end_pos.location_offset(),
            line: start_pos.location_line() as usize,
            column: start_pos.get_utf8_column(),
        };

        let node = FacetNode::Import(fct_ast::ImportNode { path, span });

        return Ok((input, node));
    }

    // Special-case @interface Name (functions parsing TBD)
    if name == "interface" {
        let (input, iface_name) = identifier(input)?;
        let (input, _) = space0(input)?;
        let (input, _) = eol(input)?;
        let (input, functions) = interface_body(level + 1)(input)?;

        let span = Span {
            start: start_pos.location_offset(),
            end: start_pos.location_offset() + iface_name.len(),
            line: start_pos.location_line() as usize,
            column: start_pos.get_utf8_column(),
        };

        let node = FacetNode::Interface(fct_ast::InterfaceNode {
            name: iface_name,
            functions,
            span,
        });

        return Ok((input, node));
    }

    // Parse facet attributes.
    // @test supports both legacy `@test(name="...")` and spec form `@test "..."`.
    let (input, parsed_attributes) = if name == "test" {
        if let Ok((next, attrs)) = attributes(input) {
            (next, attrs)
        } else if let Ok((next, test_name)) = string_literal(input) {
            let mut attrs = OrderedMap::new();
            attrs.insert("name".to_string(), ValueNode::String(test_name));
            (next, attrs)
        } else {
            (input, OrderedMap::new())
        }
    } else {
        let (next, attrs) = opt(attributes)(input)?;
        (next, attrs.unwrap_or_default())
    };

    let (input, _) = space0(input)?;

    // Check for inline body or newline
    let (input, _) = eol(input)?;

    // Parse body with increased indentation
    let (input, body) = if name == "test" {
        test_block_body(level + 1)(input)?
    } else {
        block_body(level + 1)(input)?
    };

    let (input, end_pos) = nom_locate::position(input)?;

    let span = Span {
        start: start_pos.location_offset(),
        end: end_pos.location_offset(),
        line: start_pos.location_line() as usize,
        column: start_pos.get_utf8_column(),
    };

    let node = match name.as_str() {
        "system" => FacetNode::System(FacetBlock {
            name: name.clone(),
            attributes: parsed_attributes.clone(),
            body,
            span,
        }),
        "user" => FacetNode::User(FacetBlock {
            name: name.clone(),
            attributes: parsed_attributes.clone(),
            body,
            span,
        }),
        "assistant" => FacetNode::Assistant(FacetBlock {
            name: name.clone(),
            attributes: parsed_attributes.clone(),
            body,
            span,
        }),
        "vars" => FacetNode::Vars(FacetBlock {
            name: name.clone(),
            attributes: parsed_attributes.clone(),
            body,
            span,
        }),
        "var_types" => FacetNode::VarTypes(FacetBlock {
            name: name.clone(),
            attributes: parsed_attributes.clone(),
            body,
            span,
        }),
        "context" => FacetNode::Context(FacetBlock {
            name: name.clone(),
            attributes: parsed_attributes.clone(),
            body,
            span,
        }),
        "policy" => FacetNode::Policy(FacetBlock {
            name: name.clone(),
            attributes: parsed_attributes.clone(),
            body,
            span,
        }),
        "test" => {
            // Parse test block content
            let test_name = parsed_attributes
                .get("name")
                .and_then(|v| {
                    if let ValueNode::String(s) = v {
                        Some(s.as_str())
                    } else {
                        None
                    }
                })
                .unwrap_or("unnamed test");

            // Parse vars section
            let vars = parse_test_vars(&body);

            // Parse input section
            let input = parse_test_input(&body);

            // Parse mock section
            let mocks = parse_test_mocks(&body);

            // Parse assert section
            let assertions = parse_test_assertions(&body);

            FacetNode::Test(fct_ast::TestBlock {
                name: test_name.to_string(),
                vars,
                input,
                mocks,
                assertions,
                body: Vec::new(), // Test blocks don't have regular body
                span,
            })
        }
        _ => FacetNode::Meta(FacetBlock {
            name: name.clone(),
            attributes: parsed_attributes.clone(),
            body,
            span,
        }),
    };

    Ok((input, node))
}

pub fn normalize_source(input: &str) -> String {
    let nfc: String = input.nfc().collect();
    nfc.replace("\r\n", "\n").replace('\r', "\n")
}

pub fn compute_document_hash(input: &str) -> String {
    let normalized = normalize_source(input);
    let hash = Sha256::digest(normalized.as_bytes());
    format!("{:x}", hash)
}

fn has_forbidden_attribute_interpolation(input: &str) -> Option<usize> {
    for (idx, line) in input.lines().enumerate() {
        let mut offset = 0usize;
        while offset < line.len() {
            let Some(at_rel) = line[offset..].find('@') else {
                break;
            };
            let at = offset + at_rel;
            let Some(open_rel) = line[at..].find('(') else {
                break;
            };
            let open = at + open_rel;
            let Some(close_rel) = line[open + 1..].find(')') else {
                break;
            };
            let close = open + 1 + close_rel;
            let attrs = &line[open + 1..close];
            if attrs.contains("{{") || attrs.contains("}}") {
                return Some(idx + 1);
            }
            offset = close + 1;
        }
    }
    None
}

pub fn parse_document(input: &str) -> Result<FacetDocument, String> {
    let normalized = normalize_source(input);

    if let Some(line) = has_forbidden_attribute_interpolation(&normalized) {
        return Err(format!(
            "F402: Attribute interpolation is forbidden (line {})",
            line
        ));
    }

    // Reject tabs per spec (F002)
    if let Some((idx, _)) = normalized
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains('\t'))
    {
        return Err(format!("F002: Tabs are not allowed (line {})", idx + 1));
    }

    // Enforce 2-space indentation (F001) for non-empty/non-comment lines
    for (idx, line) in normalized.lines().enumerate() {
        let trimmed = line.trim_start_matches(' ');
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let leading = line.len() - trimmed.len();
        // Allow top-level (0 spaces) or multiples of 2
        if leading % 2 != 0 {
            return Err(format!(
                "F001: Invalid indentation at line {} (must be multiples of 2 spaces)",
                idx + 1
            ));
        }
    }

    let span_input = SpanInput::new(&normalized);

    // Top level blocks have indentation 0
    let parser = many0(preceded(empty_lines, |i| facet_block(i, 0)));

    let (_input, blocks) = all_consuming(parser)(span_input)
        .map_err(|e| format!("F003: Unclosed delimiter: {:?}", e))?;

    Ok(FacetDocument {
        blocks,
        span: to_span(span_input),
    })
}

pub fn parse_document_bytes(input: &[u8]) -> Result<FacetDocument, String> {
    let source =
        std::str::from_utf8(input).map_err(|_| "F003: Input MUST be valid UTF-8".to_string())?;
    parse_document(source)
}

// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_forbidden_returns_f002() {
        let res = parse_document("@system\n\tkey: \"v\"\n");
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("F002"));
    }

    #[test]
    fn parses_input_directive() {
        let src = "@vars\n  user_query: @input(type=\"string\")\n";
        let doc = parse_document(src).expect("should parse input directive");
        match &doc.blocks[0] {
            FacetNode::Vars(block) => match &block.body[0] {
                BodyNode::KeyValue(kv) => match &kv.value {
                    ValueNode::Directive(d) => {
                        assert_eq!(d.name, "input");
                        assert!(
                            matches!(d.args.get("type"), Some(ValueNode::String(t)) if t == "string")
                        );
                    }
                    other => panic!("expected directive, got {:?}", other),
                },
                _ => panic!("expected key-value in vars block"),
            },
            other => panic!("expected vars block, got {:?}", other),
        }
    }

    #[test]
    fn parses_interface_header() {
        let src = "@interface WeatherAPI\n  fn get(city: string) -> string\n";
        let doc = parse_document(src).expect("interface should parse");
        match &doc.blocks[0] {
            FacetNode::Interface(iface) => {
                assert_eq!(iface.name, "WeatherAPI");
                assert_eq!(iface.functions.len(), 1);
                let func = &iface.functions[0];
                assert_eq!(func.name, "get");
                assert_eq!(func.params.len(), 1);
                assert!(
                    matches!(func.params[0].type_node, TypeNode::Primitive(ref s) if s == "string")
                );
            }
            other => panic!("expected interface node, got {:?}", other),
        }
    }

    #[test]
    fn parses_interface_composite_types() {
        let src = "@interface WeatherAPI\n  fn get_current(city: string, tags: list<string>, meta: map<string, int>) -> struct { temp: float, condition: string } (effect=\"read\")\n";
        let doc = parse_document(src).expect("interface composite types should parse");

        let iface = match &doc.blocks[0] {
            FacetNode::Interface(iface) => iface,
            other => panic!("expected interface node, got {:?}", other),
        };
        let func = &iface.functions[0];
        assert_eq!(func.name, "get_current");
        assert_eq!(func.params.len(), 3);
        assert!(matches!(func.params[0].type_node, TypeNode::Primitive(ref s) if s == "string"));
        assert!(matches!(
            func.params[1].type_node,
            TypeNode::List(ref t) if matches!(**t, TypeNode::Primitive(ref s) if s == "string")
        ));
        assert!(matches!(
            func.params[2].type_node,
            TypeNode::Map(ref t) if matches!(**t, TypeNode::Primitive(ref s) if s == "int")
        ));
        assert!(matches!(func.effect.as_deref(), Some("read")));

        match &func.return_type {
            TypeNode::Struct(fields) => {
                assert!(matches!(
                    fields.get("temp"),
                    Some(TypeNode::Primitive(s)) if s == "float"
                ));
                assert!(matches!(
                    fields.get("condition"),
                    Some(TypeNode::Primitive(s)) if s == "string"
                ));
            }
            other => panic!("expected struct return type, got {:?}", other),
        }
    }

    #[test]
    fn parses_interface_union_and_embedding_types() {
        let src = "@interface EmbedAPI\n  fn embed(text: string) -> embedding<size=1536> (effect=\"read\")\n  fn maybe(text: string) -> string | null (effect=\"read\")\n";
        let doc = parse_document(src).expect("union and embedding types should parse");

        let iface = match &doc.blocks[0] {
            FacetNode::Interface(iface) => iface,
            other => panic!("expected interface node, got {:?}", other),
        };
        assert_eq!(iface.functions.len(), 2);

        assert!(matches!(
            iface.functions[0].return_type,
            TypeNode::Embedding { size: 1536 }
        ));
        assert!(matches!(
            iface.functions[1].return_type,
            TypeNode::Union(ref members)
            if members.len() == 2
                && matches!(members[0], TypeNode::Primitive(ref s) if s == "string")
                && matches!(members[1], TypeNode::Primitive(ref s) if s == "null")
        ));
    }

    #[test]
    fn parses_interface_multiline_struct_return_type() {
        let src = "@interface WeatherAPI\n  fn get_current(city: string) -> struct {\n    temp: float\n    condition: string\n  } (effect=\"read\")\n";
        let doc = parse_document(src).expect("multiline struct return type should parse");

        let iface = match &doc.blocks[0] {
            FacetNode::Interface(iface) => iface,
            other => panic!("expected interface node, got {:?}", other),
        };
        let func = &iface.functions[0];
        assert!(matches!(func.effect.as_deref(), Some("read")));
        assert!(matches!(func.return_type, TypeNode::Struct(_)));
    }

    #[test]
    fn parses_float_literals() {
        // Test basic float
        let src = "@vars\n  pi: 3.14\n";
        let doc = parse_document(src).expect("should parse basic float");
        match &doc.blocks[0] {
            FacetNode::Vars(block) => match &block.body[0] {
                BodyNode::KeyValue(kv) => {
                    assert_eq!(kv.key, "pi");
                    match &kv.value {
                        ValueNode::Scalar(ScalarValue::Float(f)) => {
                            assert!((*f - 3.14).abs() < 0.001);
                        }
                        other => panic!("expected float scalar, got {:?}", other),
                    }
                }
                _ => panic!("expected key-value"),
            },
            other => panic!("expected vars block, got {:?}", other),
        }

        // Test scientific notation
        let src2 = "@vars\n  big: 1.23e10\n  small: 2.5e-3\n";
        let doc2 = parse_document(src2).expect("should parse scientific notation");
        match &doc2.blocks[0] {
            FacetNode::Vars(block) => match &block.body[0] {
                BodyNode::KeyValue(kv) => {
                    assert_eq!(kv.key, "big");
                    match &kv.value {
                        ValueNode::Scalar(ScalarValue::Float(f)) => {
                            assert!((*f - 1.23e10).abs() < 1e6);
                        }
                        other => panic!("expected float, got {:?}", other),
                    }
                }
                _ => panic!("expected key-value"),
            },
            other => panic!("expected vars block, got {:?}", other),
        }

        // Test that integers still work
        let src3 = "@vars\n  count: 42\n";
        let doc3 = parse_document(src3).expect("should parse integer");
        match &doc3.blocks[0] {
            FacetNode::Vars(block) => match &block.body[0] {
                BodyNode::KeyValue(kv) => match &kv.value {
                    ValueNode::Scalar(ScalarValue::Int(i)) => {
                        assert_eq!(*i, 42);
                    }
                    other => panic!("expected int scalar, got {:?}", other),
                },
                _ => panic!("expected key-value"),
            },
            other => panic!("expected vars block, got {:?}", other),
        }
    }

    // Error tests

    #[test]
    fn test_wrong_indentation_f001() {
        // Wrong indentation - 3 spaces instead of 2
        let src = "@system\n   role: \"assistant\"\n";
        let res = parse_document(src);
        assert!(res.is_err(), "Should fail on wrong indentation");
        // Parser should reject incorrect indentation
    }

    #[test]
    fn test_mixed_indentation_f001() {
        // First line correct, second line wrong
        let src = "@vars\n  key1: \"value1\"\n   key2: \"value2\"\n";
        let res = parse_document(src);
        assert!(res.is_err(), "Should fail on mixed indentation");
    }

    #[test]
    fn test_unclosed_string_f003() {
        // Unclosed string literal
        let src = "@system\n  role: \"assistant\n";
        let res = parse_document(src);
        assert!(res.is_err(), "Should fail on unclosed string");
    }

    #[test]
    fn test_unclosed_parenthesis_f003() {
        // Unclosed parenthesis in lens call
        let src = "@vars\n  text: \"hello\" |> trim(\
";
        let res = parse_document(src);
        assert!(res.is_err(), "Should fail on unclosed parenthesis");
    }

    #[test]
    fn test_invalid_block_name() {
        // Invalid block name
        let src = "@invalid_block\n  key: \"value\"\n";
        let _res = parse_document(src);
        // Should either parse or fail gracefully
        // This tests parser robustness
    }

    #[test]
    fn test_empty_document() {
        // Empty document
        let src = "";
        let res = parse_document(src);
        // Empty document should parse with no blocks
        assert!(res.is_ok());
        let doc = res.unwrap();
        assert_eq!(doc.blocks.len(), 0);
    }

    #[test]
    fn test_comments_ignored() {
        // Comments should be ignored
        let src = "# This is a comment\n@system\n  # Another comment\n  role: \"assistant\"\n";
        let res = parse_document(src);
        assert!(res.is_ok(), "Should parse with comments");
        let doc = res.unwrap();
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn test_pipeline_parsing() {
        // Test that parser correctly handles simple values and variable references
        // Note: Full pipeline support in key-value context is not yet implemented
        let src = "@vars\n  name: \"Alice\"\n  upper: $name";
        let res = parse_document(src);
        assert!(res.is_ok(), "Should parse variables");
        let doc = res.unwrap();
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0] {
            FacetNode::Vars(block) => {
                assert_eq!(block.body.len(), 2);
                // First key-value: string literal
                match &block.body[0] {
                    BodyNode::KeyValue(kv) => {
                        assert_eq!(kv.key, "name");
                        assert!(matches!(kv.value, ValueNode::String(_)));
                    }
                    _ => panic!("expected key-value"),
                }
                // Second key-value: variable reference
                match &block.body[1] {
                    BodyNode::KeyValue(kv) => {
                        assert_eq!(kv.key, "upper");
                        assert!(matches!(kv.value, ValueNode::Variable(_)));
                    }
                    _ => panic!("expected key-value"),
                }
            }
            other => panic!("expected vars block, got {:?}", other),
        }
    }

    #[test]
    fn test_lens_with_arguments() {
        // Lens with positional and keyword arguments
        let src = "@vars\n  parts: \"a,b,c\" |> split(\",\")\n";
        let res = parse_document(src);
        assert!(res.is_ok(), "Should parse lens with arguments");
        let doc = res.unwrap();
        match &doc.blocks[0] {
            FacetNode::Vars(block) => match &block.body[0] {
                BodyNode::KeyValue(kv) => match &kv.value {
                    ValueNode::Pipeline(p) => {
                        assert_eq!(p.lenses.len(), 1);
                        assert_eq!(p.lenses[0].name, "split");
                        assert_eq!(p.lenses[0].args.len(), 1);
                    }
                    other => panic!("expected pipeline, got {:?}", other),
                },
                _ => panic!("expected key-value"),
            },
            other => panic!("expected vars block, got {:?}", other),
        }
    }

    #[test]
    fn test_variable_reference() {
        // Variable reference with $
        let src = "@vars\n  name: \"Alice\"\n  greeting: $name\n";
        let res = parse_document(src);
        assert!(res.is_ok(), "Should parse variable reference");
        let doc = res.unwrap();
        match &doc.blocks[0] {
            FacetNode::Vars(block) => {
                assert_eq!(block.body.len(), 2);
                match &block.body[1] {
                    BodyNode::KeyValue(kv) => match &kv.value {
                        ValueNode::Variable(var_ref) => {
                            assert_eq!(var_ref, "name");
                        }
                        other => panic!("expected variable, got {:?}", other),
                    },
                    _ => panic!("expected key-value"),
                }
            }
            other => panic!("expected vars block, got {:?}", other),
        }
    }

    #[test]
    fn test_multiple_blocks() {
        // Multiple blocks in document
        let src = "@system\n  role: \"assistant\"\n\n@vars\n  name: \"test\"\n\n@user\n  query: \"hello\"\n";
        let res = parse_document(src);
        assert!(res.is_ok(), "Should parse multiple blocks");
        let doc = res.unwrap();
        assert_eq!(doc.blocks.len(), 3);
        assert!(matches!(doc.blocks[0], FacetNode::System(_)));
        assert!(matches!(doc.blocks[1], FacetNode::Vars(_)));
        assert!(matches!(doc.blocks[2], FacetNode::User(_)));
    }

    #[test]
    fn test_normalizes_crlf_and_cr() {
        let src = "@vars\r\n  a: \"x\"\r  b: \"y\"\r\n";
        let res = parse_document(src);
        assert!(res.is_ok(), "parser should normalize CRLF/CR to LF");
    }

    #[test]
    fn test_normalizes_to_nfc() {
        // "e" + COMBINING ACUTE ACCENT should normalize to NFC "é"
        let src = "@vars\n  word: \"e\u{0301}\"\n";
        let doc = parse_document(src).expect("should parse and normalize to NFC");
        match &doc.blocks[0] {
            FacetNode::Vars(block) => match &block.body[0] {
                BodyNode::KeyValue(kv) => match &kv.value {
                    ValueNode::String(s) => assert_eq!(s, "é"),
                    other => panic!("expected string value, got {:?}", other),
                },
                other => panic!("expected key-value node, got {:?}", other),
            },
            other => panic!("expected vars block, got {:?}", other),
        }
    }

    #[test]
    fn test_document_hash_equivalent_for_lf_and_crlf() {
        let lf = "@vars\n  a: \"x\"\n";
        let crlf = "@vars\r\n  a: \"x\"\r\n";
        assert_eq!(compute_document_hash(lf), compute_document_hash(crlf));
    }

    #[test]
    fn test_document_hash_equivalent_for_nfc_variants() {
        let nfd = "@vars\n  word: \"e\u{0301}\"\n";
        let nfc = "@vars\n  word: \"é\"\n";
        assert_eq!(compute_document_hash(nfd), compute_document_hash(nfc));
    }

    #[test]
    fn test_span_coordinates_equivalent_for_lf_and_crlf() {
        let lf = "@vars\n  a: \"x\"\n";
        let crlf = "@vars\r\n  a: \"x\"\r\n";

        let lf_doc = parse_document(lf).expect("LF should parse");
        let crlf_doc = parse_document(crlf).expect("CRLF should parse");

        let lf_kv = match &lf_doc.blocks[0] {
            FacetNode::Vars(block) => match &block.body[0] {
                BodyNode::KeyValue(kv) => kv,
                _ => panic!("expected key-value"),
            },
            _ => panic!("expected vars block"),
        };
        let crlf_kv = match &crlf_doc.blocks[0] {
            FacetNode::Vars(block) => match &block.body[0] {
                BodyNode::KeyValue(kv) => kv,
                _ => panic!("expected key-value"),
            },
            _ => panic!("expected vars block"),
        };

        assert_eq!(lf_kv.span.line, crlf_kv.span.line);
        assert_eq!(lf_kv.span.column, crlf_kv.span.column);
        assert_eq!(lf_kv.span.start, crlf_kv.span.start);
        assert_eq!(lf_kv.span.end, crlf_kv.span.end);
    }

    #[test]
    fn test_span_coordinates_equivalent_for_nfc_variants() {
        let nfd = "@vars\n  word: \"e\u{0301}\"\n";
        let nfc = "@vars\n  word: \"é\"\n";

        let nfd_doc = parse_document(nfd).expect("NFD should parse");
        let nfc_doc = parse_document(nfc).expect("NFC should parse");

        let nfd_kv = match &nfd_doc.blocks[0] {
            FacetNode::Vars(block) => match &block.body[0] {
                BodyNode::KeyValue(kv) => kv,
                _ => panic!("expected key-value"),
            },
            _ => panic!("expected vars block"),
        };
        let nfc_kv = match &nfc_doc.blocks[0] {
            FacetNode::Vars(block) => match &block.body[0] {
                BodyNode::KeyValue(kv) => kv,
                _ => panic!("expected key-value"),
            },
            _ => panic!("expected vars block"),
        };

        assert_eq!(nfd_kv.span.line, nfc_kv.span.line);
        assert_eq!(nfd_kv.span.column, nfc_kv.span.column);
        assert_eq!(nfd_kv.span.start, nfc_kv.span.start);
        assert_eq!(nfd_kv.span.end, nfc_kv.span.end);
    }

    #[test]
    fn test_ast_snapshot_is_deterministic_for_same_source() {
        let source = "@vars\n  a: \"x\"\n  b: [1, 2]\n@user\n  content: $a\n";

        let doc1 = parse_document(source).expect("first parse should succeed");
        let doc2 = parse_document(source).expect("second parse should succeed");

        let s1 = serde_json::to_string(&doc1).expect("serialize doc1");
        let s2 = serde_json::to_string(&doc2).expect("serialize doc2");
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_ast_snapshot_is_deterministic_for_nfc_equivalent_inputs() {
        let nfd = "@vars\n  word: \"e\u{0301}\"\n@user\n  content: $word\n";
        let nfc = "@vars\n  word: \"é\"\n@user\n  content: $word\n";

        let doc1 = parse_document(nfd).expect("nfd parse should succeed");
        let doc2 = parse_document(nfc).expect("nfc parse should succeed");

        let s1 = serde_json::to_string(&doc1).expect("serialize nfd");
        let s2 = serde_json::to_string(&doc2).expect("serialize nfc");
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_rejects_non_ascii_identifier() {
        let src = "@vars\n  имя: \"Alice\"\n";
        let res = parse_document(src);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("F003"));
    }

    #[test]
    fn test_parses_unicode_escape() {
        let src = "@vars\n  x: \"\\u0041\"\n";
        let doc = parse_document(src).expect("unicode escape should parse");
        match &doc.blocks[0] {
            FacetNode::Vars(block) => match &block.body[0] {
                BodyNode::KeyValue(kv) => match &kv.value {
                    ValueNode::String(s) => assert_eq!(s, "A"),
                    other => panic!("expected string, got {:?}", other),
                },
                _ => panic!("expected key-value"),
            },
            _ => panic!("expected vars block"),
        }
    }

    #[test]
    fn test_inline_list_and_map_literals_parse() {
        let src = "@vars\n  xs: [1, \"x\", true, null]\n  cfg: {retries: 3, mode: \"safe\", nested: {ok: true}}\n";
        let doc = parse_document(src).expect("inline list/map should parse");

        match &doc.blocks[0] {
            FacetNode::Vars(block) => {
                assert_eq!(block.body.len(), 2);
                match &block.body[0] {
                    BodyNode::KeyValue(kv) => match &kv.value {
                        ValueNode::List(items) => assert_eq!(items.len(), 4),
                        other => panic!("expected inline list, got {:?}", other),
                    },
                    _ => panic!("expected key-value for xs"),
                }

                match &block.body[1] {
                    BodyNode::KeyValue(kv) => match &kv.value {
                        ValueNode::Map(map) => {
                            assert!(map.contains_key("retries"));
                            assert!(map.contains_key("mode"));
                            assert!(map.contains_key("nested"));
                        }
                        other => panic!("expected inline map, got {:?}", other),
                    },
                    _ => panic!("expected key-value for cfg"),
                }
            }
            _ => panic!("expected vars block"),
        }
    }

    #[test]
    fn test_trailing_comma_inline_list_rejected() {
        let src = "@vars\n  xs: [1, 2,]\n";
        let res = parse_document(src);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("F003"));
    }

    #[test]
    fn test_trailing_comma_inline_map_rejected() {
        let src = "@vars\n  cfg: {a: 1,}\n";
        let res = parse_document(src);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("F003"));
    }

    #[test]
    fn test_quoted_key_in_block_parses() {
        let src = "@meta\n  \"x.acme.build_id\": \"abc\"\n";
        let doc = parse_document(src).expect("quoted keys should parse");
        match &doc.blocks[0] {
            FacetNode::Meta(block) => match &block.body[0] {
                BodyNode::KeyValue(kv) => {
                    assert_eq!(kv.key, "x.acme.build_id");
                    assert_eq!(kv.key_kind, MapKeyKind::String);
                }
                _ => panic!("expected key-value"),
            },
            _ => panic!("expected meta block"),
        }
    }

    #[test]
    fn test_parse_document_bytes_utf8_validation() {
        let bad = &[0xff, 0xfe, 0xfd];
        let res = parse_document_bytes(bad);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("UTF-8"));
    }

    #[test]
    fn test_parses_assistant_context_and_policy() {
        let src = "@context\n  budget: 1000\n\n@assistant\n  content: \"ok\"\n\n@policy\n  defaults: { }\n";
        let doc = parse_document(src).expect("facets should parse");
        assert_eq!(doc.blocks.len(), 3);
        assert!(matches!(doc.blocks[0], FacetNode::Context(_)));
        assert!(matches!(doc.blocks[1], FacetNode::Assistant(_)));
        assert!(matches!(doc.blocks[2], FacetNode::Policy(_)));
    }

    #[test]
    fn test_attribute_allows_atom_values() {
        let src = "@system(model=\"gpt-x\", temp=0.2, when=$enabled)\n  content: \"ok\"\n";
        let doc = parse_document(src).expect("atom attributes should parse");
        match &doc.blocks[0] {
            FacetNode::System(block) => {
                assert_eq!(block.attributes.len(), 3);
            }
            _ => panic!("expected system block"),
        }
    }

    #[test]
    fn test_attribute_rejects_pipeline_value() {
        let src = "@system(model=\"gpt-x\" |> trim())\n  content: \"ok\"\n";
        let res = parse_document(src);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("F003"));
    }

    #[test]
    fn test_attribute_rejects_input_directive() {
        let src = "@system(when=@input(type=\"bool\"))\n  content: \"ok\"\n";
        let res = parse_document(src);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("F003"));
    }

    #[test]
    fn test_attribute_interpolation_raises_f402() {
        let src = "@system(prompt=\"Hello {{name}}\")\n  content: \"ok\"\n";
        let res = parse_document(src);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("F402"));
    }

    #[test]
    fn test_parses_test_input_section() {
        let src = "@test(name=\"basic\")\n  input:\n    query: \"hello\"\n    n: 3\n  assert:\n    - \"output contains hello\"\n";
        let doc = parse_document(src).expect("@test input should parse");
        match &doc.blocks[0] {
            FacetNode::Test(test) => {
                assert_eq!(test.name, "basic");
                assert_eq!(test.input.len(), 2);
                assert!(
                    matches!(test.input.get("query"), Some(ValueNode::String(s)) if s == "hello")
                );
                assert!(matches!(
                    test.input.get("n"),
                    Some(ValueNode::Scalar(ScalarValue::Int(3)))
                ));
                assert_eq!(test.assertions.len(), 1);
            }
            other => panic!("expected test block, got {:?}", other),
        }
    }

    #[test]
    fn test_parses_test_block_with_spec_string_name_syntax() {
        let src =
            "@test \"basic\"\n  assert:\n    - \"canonical.messages[0].role == \\\"system\\\"\"\n";
        let doc = parse_document(src).expect("@test \"name\" should parse");
        match &doc.blocks[0] {
            FacetNode::Test(test) => {
                assert_eq!(test.name, "basic");
                assert_eq!(test.assertions.len(), 1);
            }
            other => panic!("expected test block, got {:?}", other),
        }
    }

    #[test]
    fn test_parses_test_block_vars_mock_assert_multiline() {
        let src = "@test(name=\"full\")\n  vars:\n    username: \"TestUser\"\n  mock:\n    WeatherAPI.get_current: { temp: 10, condition: \"Rain\" }\n  assert:\n    - \"output contains Rain\"\n";
        let doc = parse_document(src).expect("@test multiline sections should parse");
        match &doc.blocks[0] {
            FacetNode::Test(test) => {
                assert_eq!(test.name, "full");
                assert_eq!(test.vars.len(), 1);
                assert!(matches!(
                    test.vars.get("username"),
                    Some(ValueNode::String(s)) if s == "TestUser"
                ));
                assert_eq!(test.mocks.len(), 1);
                assert_eq!(test.mocks[0].target, "WeatherAPI.get_current");
                assert_eq!(test.assertions.len(), 1);
            }
            other => panic!("expected test block, got {:?}", other),
        }
    }

    #[test]
    fn test_parses_test_assertion_expression_equals() {
        let src = "@test(name=\"expr\")\n  assert:\n    - \"canonical.messages[0].role == \\\"system\\\"\"\n";
        let doc = parse_document(src).expect("@test assertion expression should parse");
        match &doc.blocks[0] {
            FacetNode::Test(test) => {
                assert_eq!(test.assertions.len(), 1);
                match &test.assertions[0].kind {
                    fct_ast::AssertionKind::Equals { target, expected } => {
                        assert_eq!(target, "canonical.messages[0].role");
                        assert!(
                            matches!(expected, ValueNode::String(s) if s == "system"),
                            "expected string literal parsed from equality assertion"
                        );
                    }
                    other => panic!("expected equals assertion, got {:?}", other),
                }
            }
            other => panic!("expected test block, got {:?}", other),
        }
    }
}

// ============================================================================
// TEST BLOCK PARSING
// ============================================================================

fn parse_test_vars(body: &[BodyNode]) -> OrderedMap<String, fct_ast::ValueNode> {
    let mut vars = OrderedMap::new();

    for node in body {
        if let BodyNode::KeyValue(kv) = node {
            if kv.key == "vars" {
                if let fct_ast::ValueNode::Map(var_map) = &kv.value {
                    for (key, value) in var_map {
                        vars.insert(key.clone(), value.clone());
                    }
                }
            }
        }
    }

    vars
}

fn parse_test_input(body: &[BodyNode]) -> OrderedMap<String, fct_ast::ValueNode> {
    let mut input = OrderedMap::new();

    for node in body {
        if let BodyNode::KeyValue(kv) = node {
            if kv.key == "input" {
                if let fct_ast::ValueNode::Map(input_map) = &kv.value {
                    for (key, value) in input_map {
                        input.insert(key.clone(), value.clone());
                    }
                }
            }
        }
    }

    input
}

fn parse_test_mocks(body: &[BodyNode]) -> Vec<fct_ast::MockDefinition> {
    let mut mocks = Vec::new();

    for node in body {
        if let BodyNode::KeyValue(kv) = node {
            if kv.key == "mock" {
                if let fct_ast::ValueNode::Map(mock_map) = &kv.value {
                    for (target, return_value) in mock_map {
                        mocks.push(fct_ast::MockDefinition {
                            target: target.clone(),
                            return_value: return_value.clone(),
                            span: kv.span.clone(),
                        });
                    }
                }
            }
        }
    }

    mocks
}

fn parse_test_assertions(body: &[BodyNode]) -> Vec<fct_ast::Assertion> {
    let mut assertions = Vec::new();

    for node in body {
        if let BodyNode::KeyValue(kv) = node {
            if kv.key == "assert" {
                if let fct_ast::ValueNode::List(assert_list) = &kv.value {
                    for assert_value in assert_list {
                        if let fct_ast::ValueNode::String(assert_str) = assert_value {
                            if let Some(assertion) =
                                parse_assertion_from_string(assert_str, &kv.span)
                            {
                                assertions.push(assertion);
                            }
                        }
                    }
                }
            }
        }
    }

    assertions
}

fn parse_assertion_from_string(
    assert_str: &str,
    span: &fct_ast::Span,
) -> Option<fct_ast::Assertion> {
    let expr = assert_str.trim();
    if expr.is_empty() {
        return None;
    }

    let kind = if let Some((lhs, rhs)) = expr.split_once(" not contains ") {
        fct_ast::AssertionKind::NotContains {
            target: lhs.trim().to_string(),
            text: strip_wrapping_quotes(rhs.trim()).to_string(),
        }
    } else if let Some((lhs, rhs)) = expr.split_once(" contains ") {
        fct_ast::AssertionKind::Contains {
            target: lhs.trim().to_string(),
            text: strip_wrapping_quotes(rhs.trim()).to_string(),
        }
    } else if let Some((lhs, rhs)) = expr.split_once(" == ") {
        fct_ast::AssertionKind::Equals {
            target: lhs.trim().to_string(),
            expected: parse_assert_value(rhs.trim()),
        }
    } else if let Some((lhs, rhs)) = expr.split_once(" != ") {
        fct_ast::AssertionKind::NotEquals {
            target: lhs.trim().to_string(),
            expected: parse_assert_value(rhs.trim()),
        }
    } else if let Some((lhs, rhs)) = expr.split_once(" < ") {
        let value = rhs.trim().parse::<f64>().ok()?;
        fct_ast::AssertionKind::LessThan {
            field: lhs.trim().to_string(),
            value,
        }
    } else if let Some((lhs, rhs)) = expr.split_once(" > ") {
        let value = rhs.trim().parse::<f64>().ok()?;
        fct_ast::AssertionKind::GreaterThan {
            field: lhs.trim().to_string(),
            value,
        }
    } else if let Some(target) = expr.strip_suffix(" is true") {
        fct_ast::AssertionKind::True {
            target: target.trim().to_string(),
        }
    } else if let Some(target) = expr.strip_suffix(" is false") {
        fct_ast::AssertionKind::False {
            target: target.trim().to_string(),
        }
    } else if let Some(target) = expr.strip_suffix(" is not null") {
        fct_ast::AssertionKind::NotNull {
            target: target.trim().to_string(),
        }
    } else if let Some(target) = expr.strip_suffix(" is null") {
        fct_ast::AssertionKind::Null {
            target: target.trim().to_string(),
        }
    } else if let Some(expected) = expr.strip_prefix("sentiment ") {
        fct_ast::AssertionKind::Sentiment {
            target: "output".to_string(),
            expected: strip_wrapping_quotes(expected.trim()).to_string(),
        }
    } else {
        // Backward-compat fallback for legacy shorthand parser behavior.
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }
        match parts[0] {
            "output" => {
                if parts.len() >= 3 {
                    match parts[1] {
                        "contains" => fct_ast::AssertionKind::Contains {
                            target: "output".to_string(),
                            text: parts[2..].join(" ").trim_matches('"').to_string(),
                        },
                        "not" => {
                            if parts.len() >= 4 && parts[2] == "contains" {
                                fct_ast::AssertionKind::NotContains {
                                    target: "output".to_string(),
                                    text: parts[3..].join(" ").trim_matches('"').to_string(),
                                }
                            } else {
                                return None;
                            }
                        }
                        _ => return None,
                    }
                } else {
                    return None;
                }
            }
            "cost" => {
                if parts.len() >= 2 && parts[1] == "<" {
                    if let Ok(value) = parts[2].parse::<f64>() {
                        fct_ast::AssertionKind::LessThan {
                            field: "cost".to_string(),
                            value,
                        }
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            "tokens" => {
                if parts.len() >= 2 && parts[1] == "<" {
                    if let Ok(value) = parts[2].parse::<f64>() {
                        fct_ast::AssertionKind::LessThan {
                            field: "tokens".to_string(),
                            value,
                        }
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            "sentiment" => {
                if parts.len() >= 2 {
                    fct_ast::AssertionKind::Sentiment {
                        target: "output".to_string(),
                        expected: parts[1].trim_matches('"').to_string(),
                    }
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    };

    Some(fct_ast::Assertion {
        kind,
        span: span.clone(),
    })
}

fn strip_wrapping_quotes(s: &str) -> &str {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn parse_assert_value(raw: &str) -> ValueNode {
    let trimmed = raw.trim();
    if trimmed == "true" {
        ValueNode::Scalar(ScalarValue::Bool(true))
    } else if trimmed == "false" {
        ValueNode::Scalar(ScalarValue::Bool(false))
    } else if trimmed == "null" {
        ValueNode::Scalar(ScalarValue::Null)
    } else if let Ok(i) = trimmed.parse::<i64>() {
        ValueNode::Scalar(ScalarValue::Int(i))
    } else if let Ok(f) = trimmed.parse::<f64>() {
        ValueNode::Scalar(ScalarValue::Float(f))
    } else {
        ValueNode::String(strip_wrapping_quotes(trimmed).to_string())
    }
}
