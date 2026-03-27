with open("config/src/config.rs", "r") as f:
    content = f.read()

# Insert the functions outside `impl Config`
patch_fns = """
use std::iter::Peekable;
use std::str::Chars;

pub fn is_safe_lua_literal(s: &str) -> bool {
    let mut chars = s.trim().chars().peekable();
    if let Ok(_) = parse_expr(&mut chars) {
        skip_whitespace(&mut chars);
        chars.next().is_none()
    } else {
        false
    }
}

fn skip_whitespace(chars: &mut Peekable<Chars>) {
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
}

fn parse_expr(chars: &mut Peekable<Chars>) -> Result<(), ()> {
    skip_whitespace(chars);
    match chars.peek() {
        Some(&'"') | Some(&'\\'') => parse_string(chars),
        Some(&'{') => parse_table(chars),
        Some(&c) if c.is_ascii_digit() || *c == '-' || *c == '+' || *c == '.' => parse_number(chars),
        Some(&c) if c.is_ascii_alphabetic() || *c == '_' => parse_ident_or_keyword(chars),
        _ => Err(()),
    }
}

fn parse_string(chars: &mut Peekable<Chars>) -> Result<(), ()> {
    let quote = chars.next().unwrap();
    let mut escaped = false;
    for c in chars.by_ref() {
        if escaped {
            escaped = false;
        } else if c == '\\\\' {
            escaped = true;
        } else if c == quote {
            return Ok(());
        }
    }
    Err(())
}

fn parse_table(chars: &mut Peekable<Chars>) -> Result<(), ()> {
    chars.next(); // skip '{'
    loop {
        skip_whitespace(chars);
        if let Some(&'}') = chars.peek() {
            chars.next();
            return Ok(());
        }

        // Try to parse `[expr] = expr`
        if let Some(&'[') = chars.peek() {
            chars.next(); // '['
            parse_expr(chars)?;
            skip_whitespace(chars);
            if chars.next() != Some(']') { return Err(()); }
            skip_whitespace(chars);
            if chars.next() != Some('=') { return Err(()); }
            skip_whitespace(chars);
            parse_expr(chars)?;
        } else {
            // Try to parse `ident = expr` or just `expr`
            let mut clone = chars.clone();
            let mut is_ident = false;
            if let Some(&c) = clone.peek() {
                if c.is_ascii_alphabetic() || c == '_' {
                    clone.next();
                    while let Some(&c) = clone.peek() {
                        if c.is_ascii_alphanumeric() || *c == '_' {
                            clone.next();
                        } else {
                            break;
                        }
                    }
                    skip_whitespace(&mut clone);
                    if clone.peek() == Some(&'=') {
                        is_ident = true;
                        *chars = clone;
                        chars.next(); // '='
                        skip_whitespace(chars);
                        parse_expr(chars)?;
                    }
                }
            }
            if !is_ident {
                parse_expr(chars)?;
            }
        }

        skip_whitespace(chars);
        match chars.peek() {
            Some(&',') | Some(&';') => { chars.next(); },
            Some(&'}') => { chars.next(); return Ok(()); },
            _ => return Err(()),
        }
    }
}

fn parse_number(chars: &mut Peekable<Chars>) -> Result<(), ()> {
    let mut num_str = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+' || *c == 'e' || *c == 'E' || *c == 'x' || *c == 'X' || c.is_ascii_hexdigit() {
            num_str.push(*c);
            chars.next();
        } else {
            break;
        }
    }
    if num_str.is_empty() {
        Err(())
    } else {
        Ok(())
    }
}

fn parse_ident_or_keyword(chars: &mut Peekable<Chars>) -> Result<(), ()> {
    let mut ident = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_alphanumeric() || *c == '_' {
            ident.push(*c);
            chars.next();
        } else {
            break;
        }
    }
    if ident == "true" || ident == "false" || ident == "nil" {
        Ok(())
    } else {
        Err(())
    }
}
"""

# Find `fn default_one() -> usize {` which is outside of impl Config to place our fns
content = content.replace("fn default_one() -> usize {", patch_fns + "\nfn default_one() -> usize {")

patch2 = """            if value == "nil" {
                // Literal nil as the value is the same as not specifying the value.
                // We special case this here as we want to explicitly check for
                // the value evaluating as nil, as can happen in the case where the
                // user specifies something like: `--config term=xterm`.
                // The RHS references a global that doesn't exist and evaluates as
                // nil. We want to raise this as an error.
                continue;
            }
            if !is_safe_lua_literal(value) {
                anyhow::bail!(
                    "config override for `{}` has an invalid or unsafe value: `{}`. Only simple literals (strings, numbers, booleans, and basic tables) are allowed.",
                    key, value
                );
            }"""

content = content.replace("""            if value == "nil" {
                // Literal nil as the value is the same as not specifying the value.
                // We special case this here as we want to explicitly check for
                // the value evaluating as nil, as can happen in the case where the
                // user specifies something like: `--config term=xterm`.
                // The RHS references a global that doesn't exist and evaluates as
                // nil. We want to raise this as an error.
                continue;
            }""", patch2)

with open("config/src/config.rs", "w") as f:
    f.write(content)
