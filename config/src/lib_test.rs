use mlua::Lua;

#[test]
fn test_injection() {
    use crate::config::is_safe_lua_literal;
    assert!(is_safe_lua_literal("true"));
    assert!(is_safe_lua_literal("false"));
    assert!(is_safe_lua_literal("nil"));
    assert!(is_safe_lua_literal("123.45"));
    assert!(is_safe_lua_literal("\"a string\""));
    assert!(is_safe_lua_literal("{ a = 1, [2] = 'b', 3 }"));
    assert!(!is_safe_lua_literal("os.execute('calc')"));
    assert!(!is_safe_lua_literal("{ a = os.execute('calc') }"));
    assert!(!is_safe_lua_literal("function() end"));
    assert!(is_safe_lua_literal("{ nested = { table = true }, num = -1.2e+3, hex = 0xff }"));
    assert!(!is_safe_lua_literal("{ [os.execute('calc')] = 1 }"));
}
