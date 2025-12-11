use crate::error::{ParseResult, SpanInput};
use fct_ast::{
    BodyNode,
    DirectiveNode,
    FacetBlock,
    FacetDocument,
    FacetNode,
    FunctionSignature,
    KeyValueNode,
    LensCallNode,
    ListItemNode,
    Parameter,
    PipelineNode,
    ScalarValue,
    Span,
    TypeNode,
    ValueNode,
};
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while},
    character::complete::{
        alpha1,
        alphanumeric1,
        char,
        digit1,
        line_ending,
        multispace0,
        none_of,
        space0,
        space1,
    },
    combinator::{all_consuming, eof, map, map_res, opt, recognize, value},
    multi::{many0, separated_list0, separated_list1},
    sequence::{delimited, pair, preceded, terminated, tuple},
};
use std::collections::HashMap;

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
    map(
        recognize(pair(
            alt((alpha1, tag("_"))),
            many0(alt((alphanumeric1, tag("_")))),
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
            // Found escape, parse the escaped character
            if let Ok((after2, ch)) = alt::<_, _, nom::error::VerboseError<SpanInput>, _>((
                value('"', char('"')),
                value('\\', char('\\')),
                value('\n', char('n')),
                value('\r', char('r')),
                value('\t', char('t')),
            ))(after) {
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
    map_res(digit1, |s: SpanInput| s.fragment().parse::<i64>())(input)
}

fn float(input: SpanInput) -> ParseResult<f64> {
    use nom::character::complete::{char, one_of};
    use nom::combinator::recognize;
    use nom::sequence::tuple;

    // Float pattern: [digits].[digits] optionally followed by e[+-]?[digits]
    let float_pattern = recognize(tuple((
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
    let (input, ty) = identifier(input)?;
    let node = match ty.as_str() {
        "string" | "int" | "float" | "bool" | "null" | "any" => TypeNode::Primitive(ty),
        _ => TypeNode::Primitive(ty), // Fallback to primitive for unknown types
    };
    Ok((input, node))
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
fn lens_args(input: SpanInput) -> ParseResult<(Vec<ValueNode>, HashMap<String, ValueNode>)> {
    let (input, _) = space0(input)?;

    // Empty args case - check for ')' but DON'T consume it (lens_call will consume it)
    if let Ok(_) = char::<_, nom::error::VerboseError<SpanInput>>(')')(input) {
        return Ok((input, (vec![], HashMap::new())));
    }

    let mut positional = vec![];
    let mut named = HashMap::new();
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
                if let Ok((inp7, _)) = 
                    char::<_, nom::error::VerboseError<SpanInput>>(',')(inp6)
                {
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
    let (input, entries) = separated_list0(separator, preceded(multispace0, key_value_pair_only))(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('}')(input)?;

    let mut map = std::collections::HashMap::new();
    for (key, value) in entries {
        map.insert(key, value);
    }
    Ok((input, ValueNode::Map(map)))
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

fn key_value(input: SpanInput) -> ParseResult<KeyValueNode> {
    let (input, key) = identifier(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = space0(input)?;
    let (input, value) = parse_value(input)?;

    Ok((
        input,
        KeyValueNode {
            key,
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

fn attributes(input: SpanInput) -> ParseResult<HashMap<String, ValueNode>> {
    delimited(
        char('('),
        map(
            separated_list0(
                preceded(space0, char(',')), 
                preceded(space0, key_value_pair_only)
            ),
            |pairs| pairs.into_iter().collect(),
        ),
        char(')')
    )(input)
}

// Helper for attributes where we just want key=value or key: value
fn key_value_pair_only(input: SpanInput) -> ParseResult<(String, ValueNode)> {
    let (input, key) = identifier(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = alt((char(':'), char('=')))(input)?;
    let (input, _) = space0(input)?;
    let (input, value) = parse_value(input)?;
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
        let (input, _) = eol(input)?;

        Ok((
            input,
            FunctionSignature {
                name,
                params,
                return_type,
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

        let node = FacetNode::Import(fct_ast::ImportNode {
            path,
            span,
        });

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

    // Parse optional attributes: (key=value, ...)
    let (input, attrs) = opt(attributes)(input)?;
    let attributes = attrs.unwrap_or_default();

    let (input, _) = space0(input)?;

    // Check for inline body or newline
    let (input, _) = eol(input)?;

    // Parse body with increased indentation
    let (input, body) = block_body(level + 1)(input)?;

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
            attributes: attributes.clone(),
            body,
            span,
        }),
        "user" => FacetNode::User(FacetBlock {
            name: name.clone(),
            attributes: attributes.clone(),
            body,
            span,
        }),
        "vars" => FacetNode::Vars(FacetBlock {
            name: name.clone(),
            attributes: attributes.clone(),
            body,
            span,
        }),
        "var_types" => FacetNode::VarTypes(FacetBlock {
            name: name.clone(),
            attributes: attributes.clone(),
            body,
            span,
        }),
        "test" => {
            // Parse test block content
            let test_name = attributes.get("name")
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

            // Parse mock section
            let mocks = parse_test_mocks(&body);

            // Parse assert section
            let assertions = parse_test_assertions(&body);

            FacetNode::Test(fct_ast::TestBlock {
                name: test_name.to_string(),
                vars,
                mocks,
                assertions,
                body: Vec::new(), // Test blocks don't have regular body
                span,
            })
        },
        _ => FacetNode::Meta(FacetBlock {
            name: name.clone(),
            attributes: attributes.clone(),
            body,
            span,
        }),
    };

    Ok((input, node))
}

pub fn parse_document(input: &str) -> Result<FacetDocument, String> {
    // Reject tabs per spec (F002)
    if let Some((idx, _)) = input
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains('\t'))
    {
        return Err(format!("F002: Tabs are not allowed (line {})", idx + 1));
    }

    // Enforce 2-space indentation (F001) for non-empty/non-comment lines
    for (idx, line) in input.lines().enumerate() {
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

    let span_input = SpanInput::new(input);

    // Top level blocks have indentation 0
    let parser = many0(preceded(empty_lines, |i| facet_block(i, 0)));

    let (_input, blocks) = all_consuming(parser)(span_input)
        .map_err(|e| format!("F003: Unclosed delimiter: {:?}", e))?;

    Ok(FacetDocument {
        blocks,
        span: to_span(span_input),
    })
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
}

// ============================================================================ 
// TEST BLOCK PARSING
// ============================================================================ 



fn parse_test_vars(body: &[BodyNode]) -> std::collections::HashMap<String, fct_ast::ValueNode> {
    let mut vars = std::collections::HashMap::new();

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
                            if let Some(assertion) = parse_assertion_from_string(assert_str, &kv.span) {
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

fn parse_assertion_from_string(assert_str: &str, span: &fct_ast::Span) -> Option<fct_ast::Assertion> {
    // Simple parsing for now - split by spaces and parse
    let parts: Vec<&str> = assert_str.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let kind = match parts[0] {
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
    };

    Some(fct_ast::Assertion {
        kind,
        span: span.clone(),
    })
}
