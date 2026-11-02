//! A small syntax highlighter for code blocks on slides. It splits code into
//! keywords, strings, numbers, comments, function names, types and
//! punctuation, using the fence's language name (```js, ```python …).
//! Unknown or missing languages get a generic C/script-like treatment.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tok {
    Plain,
    Keyword,
    Literal,
    String,
    Number,
    Comment,
    Function,
    Type,
    Punct,
}

#[derive(Clone, Copy, PartialEq)]
enum Family {
    CLike,
    Python,
    Shell,
    Ruby,
    Sql,
    Markup,
    Css,
    Data,
    Generic,
}

struct Lang {
    family: Family,
    keywords: &'static [&'static str],
}

const JS: &[&str] = &[
    "var",
    "let",
    "const",
    "function",
    "return",
    "if",
    "else",
    "for",
    "while",
    "do",
    "break",
    "continue",
    "switch",
    "case",
    "default",
    "new",
    "delete",
    "typeof",
    "instanceof",
    "in",
    "of",
    "class",
    "extends",
    "super",
    "this",
    "import",
    "export",
    "from",
    "as",
    "async",
    "await",
    "yield",
    "try",
    "catch",
    "finally",
    "throw",
    "void",
    "static",
    "get",
    "set",
    "interface",
    "type",
    "enum",
    "implements",
    "public",
    "private",
    "protected",
    "readonly",
    "declare",
    "namespace",
    "abstract",
    "keyof",
];
const RUST: &[&str] = &[
    "fn", "let", "mut", "const", "static", "if", "else", "match", "for", "while", "loop", "in",
    "return", "break", "continue", "struct", "enum", "impl", "trait", "type", "pub", "use", "mod",
    "crate", "self", "Self", "super", "where", "as", "ref", "move", "async", "await", "dyn",
    "unsafe", "extern",
];
const PYTHON: &[&str] = &[
    "def", "return", "if", "elif", "else", "for", "while", "in", "not", "and", "or", "is",
    "import", "from", "as", "class", "try", "except", "finally", "raise", "with", "lambda",
    "yield", "pass", "break", "continue", "global", "nonlocal", "async", "await", "del", "assert",
    "self", "print",
];
const GO: &[&str] = &[
    "func",
    "var",
    "const",
    "type",
    "struct",
    "interface",
    "map",
    "chan",
    "if",
    "else",
    "for",
    "range",
    "switch",
    "case",
    "default",
    "return",
    "break",
    "continue",
    "go",
    "defer",
    "select",
    "package",
    "import",
    "fallthrough",
    "goto",
];
const C_FAMILY: &[&str] = &[
    "int",
    "long",
    "short",
    "char",
    "float",
    "double",
    "void",
    "bool",
    "unsigned",
    "signed",
    "const",
    "static",
    "struct",
    "union",
    "enum",
    "typedef",
    "if",
    "else",
    "for",
    "while",
    "do",
    "switch",
    "case",
    "default",
    "return",
    "break",
    "continue",
    "goto",
    "sizeof",
    "class",
    "public",
    "private",
    "protected",
    "virtual",
    "override",
    "template",
    "typename",
    "namespace",
    "using",
    "new",
    "delete",
    "this",
    "try",
    "catch",
    "throw",
    "auto",
    "final",
    "extends",
    "implements",
    "import",
    "package",
    "interface",
    "abstract",
    "var",
    "string",
    "foreach",
    "in",
    "is",
    "async",
    "await",
    "internal",
    "readonly",
    "include",
    "define",
    "func",
    "let",
    "guard",
    "struct",
    "protocol",
    "extension",
    "fun",
    "val",
    "when",
    "object",
    "data",
    "init",
    "self",
];
const SHELL: &[&str] = &[
    "if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac",
    "in", "function", "return", "export", "local", "echo", "cd", "set", "unset", "source", "exit",
    "sudo",
];
const RUBY: &[&str] = &[
    "def",
    "end",
    "if",
    "elsif",
    "else",
    "unless",
    "while",
    "until",
    "for",
    "in",
    "do",
    "return",
    "class",
    "module",
    "self",
    "require",
    "yield",
    "begin",
    "rescue",
    "ensure",
    "then",
    "case",
    "when",
    "puts",
    "attr_accessor",
    "attr_reader",
    "lambda",
    "proc",
];
const PHP: &[&str] = &[
    "function",
    "return",
    "if",
    "else",
    "elseif",
    "foreach",
    "for",
    "while",
    "as",
    "echo",
    "class",
    "public",
    "private",
    "protected",
    "static",
    "new",
    "use",
    "namespace",
    "array",
    "extends",
    "implements",
    "try",
    "catch",
    "throw",
    "fn",
    "match",
];
const SQL: &[&str] = &[
    "select", "from", "where", "and", "or", "not", "insert", "into", "values", "update", "set",
    "delete", "create", "table", "drop", "alter", "join", "left", "right", "inner", "outer", "on",
    "group", "by", "order", "having", "limit", "as", "distinct", "union", "all", "in", "is",
    "like", "between", "case", "when", "then", "else", "end", "primary", "key", "index", "with",
    "asc", "desc", "count", "sum", "avg", "min", "max",
];
const GENERIC: &[&str] = &[
    "if",
    "else",
    "for",
    "while",
    "return",
    "function",
    "def",
    "fn",
    "func",
    "var",
    "let",
    "const",
    "class",
    "import",
    "from",
    "export",
    "new",
    "this",
    "self",
    "in",
    "of",
    "do",
    "switch",
    "case",
    "break",
    "continue",
    "try",
    "catch",
    "finally",
    "throw",
    "async",
    "await",
    "public",
    "private",
    "static",
    "struct",
    "enum",
    "impl",
    "type",
    "interface",
    "package",
    "use",
    "end",
    "then",
    "elif",
    "lambda",
    "yield",
];
const LITERALS: &[&str] = &[
    "true",
    "false",
    "null",
    "undefined",
    "nil",
    "None",
    "True",
    "False",
    "NaN",
    "Infinity",
    "NULL",
    "nullptr",
];

fn lang(name: &str) -> Lang {
    let n = name.trim().to_ascii_lowercase();
    let n = n
        .split(|c: char| c.is_whitespace() || c == '{')
        .next()
        .unwrap_or("");
    let (family, keywords): (Family, &'static [&'static str]) = match n {
        "js" | "javascript" | "jsx" | "ts" | "typescript" | "tsx" | "mjs" | "node" => {
            (Family::CLike, JS)
        }
        "rust" | "rs" => (Family::CLike, RUST),
        "go" | "golang" => (Family::CLike, GO),
        "c" | "h" | "cpp" | "c++" | "cc" | "hpp" | "java" | "cs" | "csharp" | "c#" | "swift"
        | "kotlin" | "kt" | "dart" | "scala" | "objc" | "objective-c" => (Family::CLike, C_FAMILY),
        "php" => (Family::CLike, PHP),
        "py" | "python" | "python3" => (Family::Python, PYTHON),
        "sh" | "bash" | "zsh" | "shell" | "console" | "fish" | "powershell" | "ps1" => {
            (Family::Shell, SHELL)
        }
        "rb" | "ruby" => (Family::Ruby, RUBY),
        "sql" | "mysql" | "postgres" | "postgresql" | "sqlite" => (Family::Sql, SQL),
        "html" | "xml" | "svg" | "vue" => (Family::Markup, &[]),
        "css" | "scss" | "sass" | "less" => (Family::Css, &[]),
        "json" | "yaml" | "yml" | "toml" | "ini" => (Family::Data, &[]),
        _ => (Family::Generic, GENERIC),
    };
    Lang { family, keywords }
}

/// Splits `code` into highlighted runs. Concatenating the runs gives `code`.
pub fn highlight(code: &str, language: &str) -> Vec<(String, Tok)> {
    let l = lang(language);
    match l.family {
        Family::Markup => markup(code),
        _ => tokens(code, &l),
    }
}

fn push(out: &mut Vec<(String, Tok)>, s: &str, t: Tok) {
    if s.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut() {
        if last.1 == t {
            last.0.push_str(s);
            return;
        }
    }
    out.push((s.to_string(), t));
}

fn tokens(code: &str, l: &Lang) -> Vec<(String, Tok)> {
    let c: Vec<char> = code.chars().collect();
    let n = c.len();
    let mut out = Vec::new();
    let mut i = 0;
    let s = |a: usize, b: usize| c[a..b].iter().collect::<String>();
    let starts = |i: usize, pat: &str| {
        let p: Vec<char> = pat.chars().collect();
        i + p.len() <= n && c[i..i + p.len()] == p[..]
    };
    let hash_comments = matches!(
        l.family,
        Family::Python | Family::Shell | Family::Ruby | Family::Data | Family::Generic
    );
    let slash_comments = matches!(
        l.family,
        Family::CLike | Family::Css | Family::Generic | Family::Data
    );
    while i < n {
        let ch = c[i];
        // comments
        if (slash_comments && starts(i, "//"))
            || (hash_comments && ch == '#' && !(l.family == Family::Css))
            || (l.family == Family::Sql && starts(i, "--"))
        {
            let mut j = i;
            while j < n && c[j] != '\n' {
                j += 1;
            }
            push(&mut out, &s(i, j), Tok::Comment);
            i = j;
            continue;
        }
        if (slash_comments || l.family == Family::Sql) && starts(i, "/*") {
            let mut j = i + 2;
            while j < n && !starts(j, "*/") {
                j += 1;
            }
            j = (j + 2).min(n);
            push(&mut out, &s(i, j), Tok::Comment);
            i = j;
            continue;
        }
        // strings
        if ch == '"' || ch == '\'' || ch == '`' {
            let triple = l.family == Family::Python && (starts(i, "\"\"\"") || starts(i, "'''"));
            let mut j = i + if triple { 3 } else { 1 };
            loop {
                if j >= n {
                    break;
                }
                if c[j] == '\\' {
                    j += 2;
                    continue;
                }
                if triple {
                    if c[j] == ch && starts(j, &ch.to_string().repeat(3)) {
                        j += 3;
                        break;
                    }
                } else if c[j] == ch {
                    j += 1;
                    break;
                } else if c[j] == '\n' && ch != '`' {
                    break;
                }
                j += 1;
            }
            let j = j.min(n);
            push(&mut out, &s(i, j), Tok::String);
            i = j;
            continue;
        }
        // numbers
        if ch.is_ascii_digit() || (ch == '.' && i + 1 < n && c[i + 1].is_ascii_digit()) {
            let prev_word = i > 0 && (c[i - 1].is_alphanumeric() || c[i - 1] == '_');
            if !prev_word {
                let mut j = i;
                while j < n && (c[j].is_ascii_alphanumeric() || c[j] == '.' || c[j] == '_') {
                    j += 1;
                }
                push(&mut out, &s(i, j), Tok::Number);
                i = j;
                continue;
            }
        }
        // words
        if ch.is_alphabetic() || ch == '_' || ch == '$' || ch == '@' {
            let mut j = i + 1;
            while j < n && (c[j].is_alphanumeric() || c[j] == '_' || c[j] == '$') {
                j += 1;
            }
            let w = s(i, j);
            let lower = w.to_ascii_lowercase();
            let is_kw = if l.family == Family::Sql {
                l.keywords.contains(&lower.as_str())
            } else {
                l.keywords.contains(&w.as_str())
            };
            let mut k = j;
            while k < n && c[k] == ' ' {
                k += 1;
            }
            let t = if LITERALS.contains(&w.as_str()) {
                Tok::Literal
            } else if is_kw {
                Tok::Keyword
            } else if l.family == Family::Css && k < n && c[k] == ':' {
                Tok::Keyword
            } else if l.family == Family::Data && k < n && c[k] == ':' {
                Tok::Function
            } else if k < n && c[k] == '(' {
                Tok::Function
            } else if w.starts_with(|ch: char| ch.is_ascii_uppercase())
                && w.chars().any(|ch| ch.is_ascii_lowercase())
            {
                Tok::Type
            } else if w.starts_with('$')
                && w.len() > 1
                && matches!(l.family, Family::Shell | Family::CLike)
            {
                // shell and PHP variables
                Tok::Type
            } else {
                Tok::Plain
            };
            push(&mut out, &w, t);
            i = j;
            continue;
        }
        let t = if ch.is_whitespace() || ch.is_alphanumeric() {
            Tok::Plain
        } else {
            Tok::Punct
        };
        push(&mut out, &ch.to_string(), t);
        i += 1;
    }
    out
}

/// HTML / XML: tags, attribute names, attribute values and comments.
fn markup(code: &str) -> Vec<(String, Tok)> {
    let c: Vec<char> = code.chars().collect();
    let n = c.len();
    let mut out = Vec::new();
    let mut i = 0;
    let s = |a: usize, b: usize| c[a..b].iter().collect::<String>();
    while i < n {
        if c[i] == '<' && i + 3 < n && c[i + 1] == '!' && c[i + 2] == '-' && c[i + 3] == '-' {
            let mut j = i + 4;
            while j + 2 < n && !(c[j] == '-' && c[j + 1] == '-' && c[j + 2] == '>') {
                j += 1;
            }
            let j = (j + 3).min(n);
            push(&mut out, &s(i, j), Tok::Comment);
            i = j;
        } else if c[i] == '<' {
            let mut j = i + 1;
            while j < n && (c[j] == '/' || c[j] == '!' || c[j] == '?') {
                j += 1;
            }
            push(&mut out, &s(i, j), Tok::Punct);
            let k = j;
            while j < n && (c[j].is_alphanumeric() || c[j] == '-' || c[j] == ':') {
                j += 1;
            }
            push(&mut out, &s(k, j), Tok::Keyword);
            i = j;
            // attributes until '>'
            while i < n && c[i] != '>' {
                if c[i] == '"' || c[i] == '\'' {
                    let q = c[i];
                    let mut j = i + 1;
                    while j < n && c[j] != q {
                        j += 1;
                    }
                    let j = (j + 1).min(n);
                    push(&mut out, &s(i, j), Tok::String);
                    i = j;
                } else if c[i].is_alphabetic() {
                    let mut j = i;
                    while j < n && (c[j].is_alphanumeric() || c[j] == '-' || c[j] == ':') {
                        j += 1;
                    }
                    push(&mut out, &s(i, j), Tok::Function);
                    i = j;
                } else {
                    let t = if c[i].is_whitespace() {
                        Tok::Plain
                    } else {
                        Tok::Punct
                    };
                    push(&mut out, &c[i].to_string(), t);
                    i += 1;
                }
            }
            if i < n {
                push(&mut out, ">", Tok::Punct);
                i += 1;
            }
        } else {
            let mut j = i;
            while j < n && c[j] != '<' {
                j += 1;
            }
            push(&mut out, &s(i, j), Tok::Plain);
            i = j;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(code: &str, lang: &str) -> Vec<(String, Tok)> {
        highlight(code, lang)
            .into_iter()
            .map(|(t, k)| (t.trim().to_string(), k))
            .filter(|(t, _)| !t.is_empty())
            .collect()
    }

    #[test]
    fn runs_rebuild_the_code() {
        let code = "var i = 10;\nvar k = i + 20; // sum\nconsole.log(\"hi\", `x`);";
        let joined: String = highlight(code, "js").into_iter().map(|(t, _)| t).collect();
        assert_eq!(joined, code);
    }

    #[test]
    fn javascript_tokens() {
        let k = kinds("var i = 10;", "js");
        assert_eq!(k[0], ("var".into(), Tok::Keyword));
        assert_eq!(k[1], ("i".into(), Tok::Plain));
        assert_eq!(k[3], ("10".into(), Tok::Number));
        let k = kinds("log(\"a\") // c", "javascript");
        assert_eq!(k[0], ("log".into(), Tok::Function));
        assert!(k.iter().any(|(t, k)| t == "\"a\"" && *k == Tok::String));
        assert_eq!(k.last().unwrap(), &("// c".into(), Tok::Comment));
    }

    #[test]
    fn python_and_sql() {
        let k = kinds("def f(x): # note\n    return None", "python");
        assert_eq!(k[0], ("def".into(), Tok::Keyword));
        assert_eq!(k[1], ("f".into(), Tok::Function));
        assert!(k.iter().any(|(t, k)| t == "# note" && *k == Tok::Comment));
        assert!(k.iter().any(|(t, k)| t == "None" && *k == Tok::Literal));
        let k = kinds("SELECT name FROM users -- all", "sql");
        assert_eq!(k[0], ("SELECT".into(), Tok::Keyword));
        assert_eq!(k.last().unwrap().1, Tok::Comment);
    }

    #[test]
    fn html_tags() {
        let k = kinds("<a href=\"/x\">Hi</a>", "html");
        assert!(k.iter().any(|(t, k)| t == "a" && *k == Tok::Keyword));
        assert!(k.iter().any(|(t, k)| t == "href" && *k == Tok::Function));
        assert!(k.iter().any(|(t, k)| t == "\"/x\"" && *k == Tok::String));
    }
}
