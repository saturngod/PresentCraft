//! A small LaTeX-to-text converter for `$...$` and `$$...$$` math.
//!
//! It handles superscripts and subscripts, Greek letters and common symbols,
//! `\frac`, `\sqrt`, `\text` and blackboard letters. The result is a list of
//! runs that callers lay out with smaller, raised or lowered text.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Script {
    Normal,
    Sup,
    Sub,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Run {
    pub text: String,
    pub script: Script,
    /// Letters (variables) are italic; digits, symbols and names are upright.
    pub italic: bool,
}

/// Converts LaTeX source into styled runs.
pub fn runs(src: &str) -> Vec<Run> {
    let chars: Vec<char> = src.chars().collect();
    let mut p = Parser { chars, i: 0 };
    let mut out = Vec::new();
    p.seq(&mut out, Script::Normal, None);
    merge(out)
}

/// Plain text of the math, for places without styling (notes, exports).
pub fn plain(src: &str) -> String {
    runs(src)
        .into_iter()
        .map(|r| match r.script {
            Script::Normal => r.text,
            Script::Sup => superscript(&r.text),
            Script::Sub => subscript(&r.text),
        })
        .collect()
}

fn superscript(s: &str) -> String {
    const FROM: &str = "0123456789+-−=()niT";
    const TO: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻⁻⁼⁽⁾ⁿⁱᵀ";
    map_script(s, FROM, TO, '^')
}

fn subscript(s: &str) -> String {
    const FROM: &str = "0123456789+-−=()aeijox";
    const TO: &str = "₀₁₂₃₄₅₆₇₈₉₊₋₋₌₍₎ₐₑᵢⱼₒₓ";
    map_script(s, FROM, TO, '_')
}

fn map_script(s: &str, from: &str, to: &str, mark: char) -> String {
    let to: Vec<char> = to.chars().collect();
    let mapped: Option<String> = s
        .chars()
        .map(|c| from.chars().position(|f| f == c).map(|k| to[k]))
        .collect();
    mapped.unwrap_or_else(|| {
        if s.chars().count() == 1 {
            format!("{mark}{s}")
        } else {
            format!("{mark}({s})")
        }
    })
}

fn merge(runs: Vec<Run>) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    for r in runs {
        if r.text.is_empty() {
            continue;
        }
        match out.last_mut() {
            Some(last) if last.script == r.script && last.italic == r.italic => {
                last.text.push_str(&r.text)
            }
            _ => out.push(r),
        }
    }
    // Source spaces collapse to one, and none at the ends.
    for r in &mut out {
        let mut s = String::new();
        for c in r.text.chars() {
            if !(c == ' ' && s.ends_with(' ')) {
                s.push(c);
            }
        }
        r.text = s;
    }
    if let Some(first) = out.first_mut() {
        first.text = first.text.trim_start().to_string();
    }
    if let Some(last) = out.last_mut() {
        last.text = last.text.trim_end().to_string();
    }
    out.retain(|r| !r.text.is_empty());
    out
}

struct Parser {
    chars: Vec<char>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.i).copied()
    }

    /// Parses until `end` (or the end of input), appending to `out`.
    fn seq(&mut self, out: &mut Vec<Run>, script: Script, end: Option<char>) {
        while let Some(c) = self.peek() {
            if Some(c) == end {
                self.i += 1;
                return;
            }
            match c {
                '^' | '_' => {
                    self.i += 1;
                    let s = if c == '^' { Script::Sup } else { Script::Sub };
                    // Nested scripts stay at one level.
                    let s = if script == Script::Normal { s } else { script };
                    self.atom(out, s);
                }
                _ => self.atom(out, script),
            }
        }
    }

    /// One token: a `{group}`, a `\command` with its arguments, or a character.
    fn atom(&mut self, out: &mut Vec<Run>, script: Script) {
        let Some(c) = self.peek() else { return };
        self.i += 1;
        match c {
            '{' => self.seq(out, script, Some('}')),
            '}' => {}
            '\\' => self.command(out, script),
            '-' => push(out, "−", script, false),
            '*' => push(out, "∗", script, false),
            '\'' => push(out, "′", script, false),
            '~' => push(out, " ", script, false),
            c if c.is_whitespace() => push(out, " ", script, false),
            c => push(out, &c.to_string(), script, c.is_alphabetic()),
        }
    }

    /// Text of the next argument, rendered flat (for `\frac` and `\sqrt`).
    fn arg(&mut self, script: Script) -> Vec<Run> {
        while self.peek().is_some_and(char::is_whitespace) {
            self.i += 1;
        }
        let mut v = Vec::new();
        self.atom(&mut v, script);
        merge(v)
    }

    /// Raw text of a `{...}` argument (for `\text`, `\mathbb`).
    fn raw_arg(&mut self) -> String {
        while self.peek().is_some_and(char::is_whitespace) {
            self.i += 1;
        }
        if self.peek() != Some('{') {
            return self.peek().map(|c| {
                self.i += 1;
                c.to_string()
            })
            .unwrap_or_default();
        }
        self.i += 1;
        let mut depth = 1;
        let mut s = String::new();
        while let Some(c) = self.peek() {
            self.i += 1;
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            s.push(c);
        }
        s
    }

    fn command(&mut self, out: &mut Vec<Run>, script: Script) {
        let start = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
            self.i += 1;
        }
        if self.i == start {
            // A one-character command such as \, \{ or \\.
            let Some(c) = self.peek() else { return };
            self.i += 1;
            let t = match c {
                ',' | ':' | ';' | ' ' | '>' => " ",
                '!' => "",
                '\\' => " ",
                '|' => "‖",
                _ => return push(out, &c.to_string(), script, false),
            };
            return push(out, t, script, false);
        }
        let name: String = self.chars[start..self.i].iter().collect();
        match name.as_str() {
            "frac" | "dfrac" | "tfrac" => {
                let a = self.arg(script);
                let b = self.arg(script);
                wrap(out, a, script);
                push(out, "/", script, false);
                wrap(out, b, script);
            }
            "sqrt" => {
                push(out, "√", script, false);
                let a = self.arg(script);
                wrap(out, a, script);
            }
            "text" | "mathrm" | "textrm" | "operatorname" | "mbox" | "textbf"
            | "mathbf" | "mathsf" | "textit" | "mathit" => {
                let t = self.raw_arg();
                let italic = matches!(name.as_str(), "textit" | "mathit");
                push(out, &t, script, italic);
            }
            "mathbb" => {
                let t: String = self.raw_arg().chars().map(blackboard).collect();
                push(out, &t, script, false);
            }
            "left" | "right" | "big" | "Big" | "bigg" | "Bigg" | "displaystyle"
            | "limits" | "nolimits" => {
                // Sizing hints; for \left. / \right. drop the dot too.
                if self.peek() == Some('.') {
                    self.i += 1;
                }
            }
            "quad" => push(out, "  ", script, false),
            "qquad" => push(out, "    ", script, false),
            _ => {
                if let Some(s) = symbol(&name) {
                    push(out, s, script, false);
                } else if FUNCTIONS.contains(&name.as_str()) {
                    push(out, &name, script, false);
                } else {
                    // Unknown command: show it as written.
                    push(out, &format!("\\{name}"), script, false);
                }
            }
        }
    }
}

fn push(out: &mut Vec<Run>, text: &str, script: Script, italic: bool) {
    out.push(Run {
        text: text.to_string(),
        script,
        italic,
    });
}

/// Adds `runs`, in parentheses when it is more than one simple term.
fn wrap(out: &mut Vec<Run>, runs: Vec<Run>, script: Script) {
    let text: String = runs.iter().map(|r| r.text.as_str()).collect();
    // Needs brackets when an operator sits outside any brackets: (a+b)/2.
    let mut depth = 0i32;
    let simple = runs.iter().all(|r| r.script == Script::Normal || r.script == script)
        && text.chars().all(|c| {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            }
            depth > 0 || !"+−=±∓/ <>≤≥×·÷".contains(c)
        });
    if !simple {
        push(out, "(", script, false);
    }
    out.extend(runs);
    if !simple {
        push(out, ")", script, false);
    }
}

fn blackboard(c: char) -> char {
    match c {
        'C' => 'ℂ',
        'H' => 'ℍ',
        'N' => 'ℕ',
        'P' => 'ℙ',
        'Q' => 'ℚ',
        'R' => 'ℝ',
        'Z' => 'ℤ',
        c => c,
    }
}

const FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan", "cot", "sec", "csc", "arcsin", "arccos", "arctan", "sinh", "cosh",
    "tanh", "log", "ln", "lg", "exp", "lim", "liminf", "limsup", "max", "min", "sup", "inf",
    "det", "dim", "ker", "deg", "gcd", "arg", "mod", "Pr", "hom",
];

fn symbol(name: &str) -> Option<&'static str> {
    Some(match name {
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "epsilon" => "ϵ",
        "varepsilon" => "ε",
        "zeta" => "ζ",
        "eta" => "η",
        "theta" => "θ",
        "vartheta" => "ϑ",
        "iota" => "ι",
        "kappa" => "κ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "xi" => "ξ",
        "pi" => "π",
        "varpi" => "ϖ",
        "rho" => "ρ",
        "varrho" => "ϱ",
        "sigma" => "σ",
        "varsigma" => "ς",
        "tau" => "τ",
        "upsilon" => "υ",
        "phi" => "ϕ",
        "varphi" => "φ",
        "chi" => "χ",
        "psi" => "ψ",
        "omega" => "ω",
        "Gamma" => "Γ",
        "Delta" => "Δ",
        "Theta" => "Θ",
        "Lambda" => "Λ",
        "Xi" => "Ξ",
        "Pi" => "Π",
        "Sigma" => "Σ",
        "Upsilon" => "Υ",
        "Phi" => "Φ",
        "Psi" => "Ψ",
        "Omega" => "Ω",
        "times" => "×",
        "cdot" => "·",
        "div" => "÷",
        "pm" => "±",
        "mp" => "∓",
        "ast" => "∗",
        "star" => "⋆",
        "circ" => "∘",
        "bullet" => "∙",
        "le" | "leq" => "≤",
        "ge" | "geq" => "≥",
        "ll" => "≪",
        "gg" => "≫",
        "ne" | "neq" => "≠",
        "approx" => "≈",
        "equiv" => "≡",
        "cong" => "≅",
        "sim" => "∼",
        "simeq" => "≃",
        "propto" => "∝",
        "infty" => "∞",
        "sum" => "∑",
        "prod" => "∏",
        "coprod" => "∐",
        "int" => "∫",
        "iint" => "∬",
        "iiint" => "∭",
        "oint" => "∮",
        "partial" => "∂",
        "nabla" => "∇",
        "to" | "rightarrow" => "→",
        "leftarrow" | "gets" => "←",
        "leftrightarrow" => "↔",
        "Rightarrow" | "implies" => "⇒",
        "Leftarrow" => "⇐",
        "Leftrightarrow" | "iff" => "⇔",
        "mapsto" => "↦",
        "uparrow" => "↑",
        "downarrow" => "↓",
        "in" => "∈",
        "notin" => "∉",
        "ni" => "∋",
        "subset" => "⊂",
        "subseteq" => "⊆",
        "supset" => "⊃",
        "supseteq" => "⊇",
        "cup" => "∪",
        "cap" => "∩",
        "setminus" => "∖",
        "emptyset" | "varnothing" => "∅",
        "forall" => "∀",
        "exists" => "∃",
        "nexists" => "∄",
        "neg" | "lnot" => "¬",
        "land" | "wedge" => "∧",
        "lor" | "vee" => "∨",
        "oplus" => "⊕",
        "otimes" => "⊗",
        "perp" => "⊥",
        "parallel" => "∥",
        "mid" => "|",
        "angle" => "∠",
        "triangle" => "△",
        "degree" => "°",
        "prime" => "′",
        "cdots" => "⋯",
        "ldots" | "dots" => "…",
        "vdots" => "⋮",
        "ddots" => "⋱",
        "hbar" => "ℏ",
        "ell" => "ℓ",
        "Re" => "ℜ",
        "Im" => "ℑ",
        "aleph" => "ℵ",
        "langle" => "⟨",
        "rangle" => "⟩",
        "lfloor" => "⌊",
        "rfloor" => "⌋",
        "lceil" => "⌈",
        "rceil" => "⌉",
        "lbrace" => "{",
        "rbrace" => "}",
        "therefore" => "∴",
        "because" => "∵",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sc(src: &str) -> Vec<(String, Script)> {
        runs(src).into_iter().map(|r| (r.text, r.script)).collect()
    }

    #[test]
    fn superscripts() {
        let n = Script::Normal;
        let s = Script::Sup;
        assert_eq!(
            sc("x^n + y^n = z^n"),
            vec![
                ("x".into(), n),
                ("n".into(), s),
                (" + ".into(), n),
                ("y".into(), n),
                ("n".into(), s),
                (" = ".into(), n),
                ("z".into(), n),
                ("n".into(), s),
            ]
        );
    }

    #[test]
    fn groups_and_subscripts() {
        assert_eq!(
            sc("a_{ij}^2"),
            vec![
                ("a".into(), Script::Normal),
                ("ij".into(), Script::Sub),
                ("2".into(), Script::Sup),
            ]
        );
    }

    #[test]
    fn commands() {
        assert_eq!(plain("\\alpha \\le \\frac{1}{2}"), "α ≤ 1/2");
        assert_eq!(plain("\\frac{n(n+1)}{2}"), "n(n+1)/2");
        assert_eq!(plain("\\frac{a+b}{c d}"), "(a+b)/(c d)");
        assert_eq!(plain("\\sum_{i=1}^{n} i"), "∑ᵢ₌₁ⁿ i");
        assert_eq!(plain("\\sqrt{2}"), "√2");
        assert_eq!(plain("x \\in \\mathbb{R}"), "x ∈ ℝ");
        assert_eq!(plain("\\sin x"), "sin x");
        assert_eq!(plain("\\text{if } x"), "if x");
    }

    #[test]
    fn variables_are_italic() {
        let r = runs("2x");
        assert!(!r[0].italic && r[1].italic);
    }
}
