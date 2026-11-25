//! Finding named definitions in source files with tree-sitter, so that
//! `@src/file.rs#Type::method` can be checked and sliced.

use tree_sitter::{Language, Node, Parser};

/// A named definition in a source file.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Symbol {
    /// Qualified name: `Type::method` in Rust, `Class.method` elsewhere.
    pub name: String,
    pub kind: &'static str,
    /// 1-based, inclusive; starts at any doc comments or attributes directly above.
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust,
    Python,
    TypeScript,
    Tsx,
    JavaScript,
}

pub const SUPPORTED: &str = ".rs, .py, .ts, .tsx, .js, .jsx, .mjs, .cjs, .mts, .cts";

impl Lang {
    pub fn for_path(path: &str) -> Option<Lang> {
        let ext = path.rsplit_once('.')?.1;
        Some(match ext {
            "rs" => Lang::Rust,
            "py" | "pyi" => Lang::Python,
            "ts" | "mts" | "cts" => Lang::TypeScript,
            "tsx" => Lang::Tsx,
            "js" | "jsx" | "mjs" | "cjs" => Lang::JavaScript,
            _ => return None,
        })
    }

    fn language(self) -> Language {
        match self {
            Lang::Rust => tree_sitter_rust::LANGUAGE.into(),
            Lang::Python => tree_sitter_python::LANGUAGE.into(),
            Lang::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Lang::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Lang::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        }
    }

    pub fn separator(self) -> &'static str {
        match self {
            Lang::Rust => "::",
            _ => ".",
        }
    }
}

/// A parsed source file: its definitions, and where its comments are.
pub struct Parsed {
    pub symbols: Vec<Symbol>,
    /// Byte ranges of every comment, in file order.
    pub comments: Vec<std::ops::Range<usize>>,
}

/// Lists the named definitions in `source`, in file order.
pub fn symbols(lang: Lang, source: &str) -> Result<Vec<Symbol>, String> {
    parse(lang, source).map(|p| p.symbols)
}

/// Parses `source` once for both definitions and comments.
///
/// Spec: @docs/syntax#code-fragments
pub fn parse(lang: Lang, source: &str) -> Result<Parsed, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang.language())
        .map_err(|e| e.to_string())?;
    let tree = parser
        .parse(source, None)
        .ok_or("tree-sitter failed to parse the file")?;
    let mut w = Walker {
        src: source.as_bytes(),
        sep: lang.separator(),
        out: Vec::new(),
    };
    let root = tree.root_node();
    match lang {
        Lang::Rust => w.rust(root, ""),
        Lang::Python => w.python(root, ""),
        _ => w.script_block(root, ""),
    }
    w.out.sort_by_key(|s| s.start_line);
    Ok(Parsed {
        symbols: w.out,
        comments: comment_ranges(root),
    })
}

/// Every comment node in the tree, found by walking it depth first.
fn comment_ranges(root: Node) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut cursor = root.walk();
    loop {
        let node = cursor.node();
        if node.kind().contains("comment") {
            out.push(node.byte_range());
        } else if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return out;
            }
        }
    }
}

/// Looks up a symbol by name. `.` and `::` are interchangeable so that a
/// reference reads naturally whichever language it points into.
pub fn find<'a>(symbols: &'a [Symbol], name: &str) -> Option<&'a Symbol> {
    symbols.iter().find(|s| s.name == name).or_else(|| {
        let want = name.replace("::", ".");
        symbols.iter().find(|s| s.name.replace("::", ".") == want)
    })
}

struct Walker<'a> {
    src: &'a [u8],
    sep: &'static str,
    out: Vec<Symbol>,
}

impl<'a> Walker<'a> {
    fn text(&self, node: Node) -> &'a str {
        node.utf8_text(self.src).unwrap_or("")
    }

    fn name_of(&self, node: Node) -> Option<&'a str> {
        node.child_by_field_name("name")
            .map(|n| self.text(n))
            .filter(|s| !s.is_empty())
    }

    /// Records `prefix + name`, spanning `range` plus leading comments/attributes.
    fn push(&mut self, range: Node, prefix: &str, name: &str, kind: &'static str) -> String {
        let mut start = range.start_position().row;
        let mut cur = range;
        while let Some(prev) = cur.prev_named_sibling() {
            let k = prev.kind();
            let leading = k.contains("comment") || k == "attribute_item" || k == "decorator";
            if leading && prev.end_position().row + 1 >= start {
                start = prev.start_position().row;
                cur = prev;
            } else {
                break;
            }
        }
        let end = range.end_position();
        let end_row = if end.column == 0 && end.row > start {
            end.row - 1
        } else {
            end.row
        };
        let full = format!("{prefix}{name}");
        self.out.push(Symbol {
            name: full.clone(),
            kind,
            start_line: start + 1,
            end_line: end_row + 1,
        });
        full
    }

    fn rust(&mut self, node: Node, prefix: &str) {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.named_children(&mut cursor).collect();
        for child in children {
            let kind = match child.kind() {
                "function_item" | "function_signature_item" => "fn",
                "struct_item" => "struct",
                "enum_item" => "enum",
                "union_item" => "union",
                "type_item" => "type",
                "const_item" => "const",
                "static_item" => "static",
                "macro_definition" => "macro",
                "trait_item" => "trait",
                "mod_item" => "mod",
                "impl_item" => {
                    if let (Some(ty), Some(body)) = (
                        child.child_by_field_name("type"),
                        child.child_by_field_name("body"),
                    ) {
                        let ty = bare_type(self.text(ty));
                        let p = format!("{prefix}{ty}{}", self.sep);
                        self.rust(body, &p);
                    }
                    continue;
                }
                _ => continue,
            };
            let Some(name) = self.name_of(child) else {
                continue;
            };
            let full = self.push(child, prefix, name, kind);
            if matches!(kind, "trait" | "mod") {
                if let Some(body) = child.child_by_field_name("body") {
                    let p = format!("{full}{}", self.sep);
                    self.rust(body, &p);
                }
            }
        }
    }

    fn python(&mut self, node: Node, prefix: &str) {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.named_children(&mut cursor).collect();
        for child in children {
            let def = if child.kind() == "decorated_definition" {
                match child.child_by_field_name("definition") {
                    Some(d) => d,
                    None => continue,
                }
            } else {
                child
            };
            match def.kind() {
                "function_definition" => {
                    if let Some(name) = self.name_of(def) {
                        self.push(child, prefix, name, "def");
                    }
                }
                "class_definition" => {
                    if let Some(name) = self.name_of(def) {
                        let full = self.push(child, prefix, name, "class");
                        if let Some(body) = def.child_by_field_name("body") {
                            self.python(body, &format!("{full}."));
                        }
                    }
                }
                "expression_statement" => {
                    let mut c = def.walk();
                    let assign = def
                        .named_children(&mut c)
                        .find(|n| n.kind() == "assignment");
                    if let Some(left) = assign.and_then(|a| a.child_by_field_name("left")) {
                        if left.kind() == "identifier" {
                            let name = self.text(left);
                            self.push(child, prefix, name, "var");
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn script_block(&mut self, node: Node, prefix: &str) {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.named_children(&mut cursor).collect();
        for child in children {
            self.script_decl(child, child, prefix);
        }
    }

    /// One TypeScript/JavaScript declaration; `range` is the outermost node
    /// (for example the `export` statement wrapping a function).
    fn script_decl(&mut self, decl: Node, range: Node, prefix: &str) {
        let simple = match decl.kind() {
            "function_declaration" | "generator_function_declaration" | "function_signature" => {
                Some("function")
            }
            "interface_declaration" => Some("interface"),
            "type_alias_declaration" => Some("type"),
            "enum_declaration" => Some("enum"),
            _ => None,
        };
        if let Some(kind) = simple {
            if let Some(name) = self.name_of(decl) {
                self.push(range, prefix, name, kind);
            }
            return;
        }
        match decl.kind() {
            "export_statement" => {
                let inner = decl
                    .child_by_field_name("declaration")
                    .or_else(|| decl.child_by_field_name("value"));
                if let Some(inner) = inner {
                    self.script_decl(inner, range, prefix);
                }
            }
            "ambient_declaration" | "expression_statement" => {
                let mut c = decl.walk();
                let kids: Vec<Node> = decl.named_children(&mut c).collect();
                for k in kids {
                    self.script_decl(k, range, prefix);
                }
            }
            "class_declaration" | "abstract_class_declaration" | "class" => {
                let Some(name) = self.name_of(decl) else {
                    return;
                };
                let full = self.push(range, prefix, name, "class");
                if let Some(body) = decl.child_by_field_name("body") {
                    let p = format!("{full}.");
                    let mut c = body.walk();
                    let members: Vec<Node> = body.named_children(&mut c).collect();
                    for m in members {
                        let kind = match m.kind() {
                            "method_definition"
                            | "method_signature"
                            | "abstract_method_signature" => "method",
                            "public_field_definition"
                            | "field_definition"
                            | "property_signature" => "field",
                            _ => continue,
                        };
                        let name = self
                            .name_of(m)
                            .or_else(|| m.child_by_field_name("property").map(|n| self.text(n)));
                        if let Some(name) = name {
                            self.push(m, &p, name, kind);
                        }
                    }
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                let mut c = decl.walk();
                let decls: Vec<Node> = decl
                    .named_children(&mut c)
                    .filter(|n| n.kind() == "variable_declarator")
                    .collect();
                for d in decls {
                    if let Some(n) = d
                        .child_by_field_name("name")
                        .filter(|n| n.kind() == "identifier")
                    {
                        let name = self.text(n);
                        self.push(range, prefix, name, "const");
                    }
                }
            }
            "internal_module" | "module" => {
                let Some(name) = self.name_of(decl) else {
                    return;
                };
                let full = self.push(range, prefix, name, "namespace");
                if let Some(body) = decl.child_by_field_name("body") {
                    self.script_block(body, &format!("{full}."));
                }
            }
            _ => {}
        }
    }
}

/// `Foo<T>` -> `Foo`, `&mut crate::a::Foo` -> `Foo`.
fn bare_type(ty: &str) -> String {
    let ty = ty.split('<').next().unwrap_or(ty);
    let ty = ty.trim_start_matches('&').trim_start_matches("mut ").trim();
    ty.rsplit("::").next().unwrap_or(ty).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(lang: Lang, src: &str) -> Vec<String> {
        symbols(lang, src)
            .unwrap()
            .into_iter()
            .map(|s| format!("{} {}-{}", s.name, s.start_line, s.end_line))
            .collect()
    }

    #[test]
    fn rust_symbols() {
        let src = "/// Doc\n#[derive(Debug)]\npub struct Graph;\n\nimpl<T> Graph {\n    /// Slices.\n    pub fn slice(&self) {}\n}\n\npub trait Walk {\n    fn step(&self);\n}\n\nmod inner {\n    pub fn deep() {}\n}\n";
        assert_eq!(
            names(Lang::Rust, src),
            vec![
                "Graph 1-3",
                "Graph::slice 6-7",
                "Walk 10-12",
                "Walk::step 11-11",
                "inner 14-16",
                "inner::deep 15-15"
            ]
        );
    }

    #[test]
    fn python_symbols() {
        let src = "MAX = 3\n\n@dataclass\nclass Session:\n    def refresh(self):\n        pass\n\ndef main():\n    pass\n";
        assert_eq!(
            names(Lang::Python, src),
            vec!["MAX 1-1", "Session 3-6", "Session.refresh 5-6", "main 8-9"]
        );
    }

    #[test]
    fn typescript_symbols() {
        let src = "// Refreshes.\nexport async function refreshAccessToken() {}\nexport class SessionManager {\n  lock() {}\n}\nexport interface Token { a: string }\nexport const TTL = 15;\ntype Id = string;\n";
        assert_eq!(
            names(Lang::TypeScript, src),
            vec![
                "refreshAccessToken 1-2",
                "SessionManager 3-5",
                "SessionManager.lock 4-4",
                "Token 6-6",
                "TTL 7-7",
                "Id 8-8"
            ]
        );
    }

    #[test]
    fn separators_are_interchangeable() {
        let syms = symbols(Lang::Rust, "struct A;\nimpl A { fn b(&self) {} }\n").unwrap();
        assert_eq!(find(&syms, "A.b").map(|s| s.name.as_str()), Some("A::b"));
    }
}
