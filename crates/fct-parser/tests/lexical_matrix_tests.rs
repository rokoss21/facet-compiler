use fct_parser::parse_document;

#[test]
fn lexical_positive_matrix() {
    let cases = [
        ("ascii identifier", "@vars\n  alpha_1: \"ok\"\n"),
        (
            "valid escapes",
            "@vars\n  s: \"line\\nquote:\\\" slash:\\\\ tab:\\t cr:\\r uni:\\u0041\"\n",
        ),
        (
            "scalars",
            "@vars\n  i: -42\n  f: 3.14\n  e: 1.2e+3\n  t: true\n  n: null\n",
        ),
        ("quoted map key", "@meta\n  \"x.acme.build_id\": \"abc\"\n"),
    ];

    for (name, src) in cases {
        let result = parse_document(src);
        assert!(result.is_ok(), "case '{}' failed: {:?}", name, result.err());
    }
}

#[test]
fn lexical_negative_matrix() {
    let cases = [
        ("non-ascii identifier", "@vars\n  имя: \"x\"\n", "F003"),
        ("invalid escape", "@vars\n  s: \"bad \\q\"\n", "F003"),
        ("unclosed string", "@vars\n  s: \"oops\n", "F003"),
        ("tab forbidden", "@vars\n\tk: \"x\"\n", "F002"),
    ];

    for (name, src, code) in cases {
        let err = parse_document(src).expect_err(name);
        assert!(
            err.contains(code),
            "case '{}' expected {}, got: {}",
            name,
            code,
            err
        );
    }
}
