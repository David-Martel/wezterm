#[test]
fn test_safe_lua_literal_accepts_basic_types() {
    use crate::config::is_safe_lua_literal;
    // Keywords
    assert!(is_safe_lua_literal("true"));
    assert!(is_safe_lua_literal("false"));
    assert!(is_safe_lua_literal("nil"));
    // Numbers: integer, float, scientific, hex
    assert!(is_safe_lua_literal("0"));
    assert!(is_safe_lua_literal("42"));
    assert!(is_safe_lua_literal("-1"));
    assert!(is_safe_lua_literal("123.45"));
    assert!(is_safe_lua_literal("-1.2e+3"));
    assert!(is_safe_lua_literal("0xff"));
    assert!(is_safe_lua_literal("0xDEADBEEF"));
    // Strings: single and double quoted
    assert!(is_safe_lua_literal("\"a string\""));
    assert!(is_safe_lua_literal("'single quoted'"));
    assert!(is_safe_lua_literal("\"\""));
    assert!(is_safe_lua_literal("''"));
    // Strings with escape sequences
    assert!(is_safe_lua_literal(r#""escaped \"quote\"""#));
    assert!(is_safe_lua_literal(r"'escaped \'quote\''"));
    assert!(is_safe_lua_literal(r#""newline\nand\ttab""#));
    assert!(is_safe_lua_literal(r#""null\0byte""#));
    // Whitespace tolerance
    assert!(is_safe_lua_literal("  true  "));
    assert!(is_safe_lua_literal("\t42\n"));
}

#[test]
fn test_safe_lua_literal_accepts_tables() {
    use crate::config::is_safe_lua_literal;
    assert!(is_safe_lua_literal("{}"));
    assert!(is_safe_lua_literal("{ a = 1, [2] = 'b', 3 }"));
    assert!(is_safe_lua_literal(
        "{ nested = { table = true }, num = -1.2e+3, hex = 0xff }"
    ));
    // Semicolon separators (valid Lua)
    assert!(is_safe_lua_literal("{ a = 1; b = 2 }"));
    // Trailing separator
    assert!(is_safe_lua_literal("{ a = 1, }"));
    // Nested tables
    assert!(is_safe_lua_literal("{ a = { b = { c = true } } }"));
    // Bracketed keys
    assert!(is_safe_lua_literal("{ [1] = 'a', [2] = 'b' }"));
    assert!(is_safe_lua_literal("{ ['key'] = true }"));
}

#[test]
fn test_safe_lua_literal_rejects_code_injection() {
    use crate::config::is_safe_lua_literal;
    // Direct function calls
    assert!(!is_safe_lua_literal("os.execute('calc')"));
    assert!(!is_safe_lua_literal("print('hello')"));
    assert!(!is_safe_lua_literal("require('os')"));
    assert!(!is_safe_lua_literal("dofile('/etc/passwd')"));
    assert!(!is_safe_lua_literal("loadstring('code')()"));
    // Function definitions
    assert!(!is_safe_lua_literal("function() end"));
    assert!(!is_safe_lua_literal("function() os.execute('calc') end"));
    // Injection via table values
    assert!(!is_safe_lua_literal("{ a = os.execute('calc') }"));
    assert!(!is_safe_lua_literal("{ [os.execute('calc')] = 1 }"));
    // String concatenation (expression, not literal)
    assert!(!is_safe_lua_literal("'a' .. 'b'"));
    assert!(!is_safe_lua_literal("\"hello\" .. os.execute('calc')"));
    // Arithmetic expressions
    assert!(!is_safe_lua_literal("1 + 2"));
    // Variable references (non-keyword identifiers)
    assert!(!is_safe_lua_literal("config"));
    assert!(!is_safe_lua_literal("path"));
    assert!(!is_safe_lua_literal("x"));
}

#[test]
fn test_safe_lua_literal_rejects_global_access() {
    use crate::config::is_safe_lua_literal;
    // Lua global objects
    assert!(!is_safe_lua_literal("_G"));
    assert!(!is_safe_lua_literal("_VERSION"));
    assert!(!is_safe_lua_literal("io"));
    assert!(!is_safe_lua_literal("os"));
    assert!(!is_safe_lua_literal("string"));
    assert!(!is_safe_lua_literal("table"));
    // In table context
    assert!(!is_safe_lua_literal("{ a = _G }"));
    assert!(!is_safe_lua_literal("{ a = io }"));
}

#[test]
fn test_safe_lua_literal_rejects_multiline_and_comments() {
    use crate::config::is_safe_lua_literal;
    // Lua multiline strings (not supported by parser — correctly rejected)
    assert!(!is_safe_lua_literal("[[multiline]]"));
    assert!(!is_safe_lua_literal("[=[level 1]=]"));
    // Lua comments
    assert!(!is_safe_lua_literal("-- comment"));
    assert!(!is_safe_lua_literal("42 -- trailing comment"));
    assert!(!is_safe_lua_literal("--[[ block comment ]]"));
    // Statement injection via newlines
    assert!(!is_safe_lua_literal("true\nos.execute('calc')"));
    assert!(!is_safe_lua_literal("42\nreturn nil"));
}

#[test]
fn test_safe_lua_literal_rejects_unterminated() {
    use crate::config::is_safe_lua_literal;
    assert!(!is_safe_lua_literal("\"unterminated"));
    assert!(!is_safe_lua_literal("'unterminated"));
    assert!(!is_safe_lua_literal("{ a = 1"));
    assert!(!is_safe_lua_literal(""));
}
