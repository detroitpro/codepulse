//! Static symbol indexer and structural pattern search (tree-sitter).

use std::fs;
use std::path::{Path, PathBuf};

use codepulse_protocol::{
    StructuralMatch, StructuralSearchResponse, SymbolId,
};
use codepulse_store::Store;
use thiserror::Error;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Node, Parser, Query, QueryCursor, Tree};
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),
    #[error("invalid pattern: {0}")]
    InvalidPattern(String),
    #[error("tree-sitter: {0}")]
    TreeSitter(String),
    #[error(transparent)]
    Store(#[from] codepulse_store::StoreError),
}

pub type Result<T> = std::result::Result<T, IndexerError>;

#[derive(Debug, Clone)]
pub struct StructuralSearchRequest {
    pub language: String,
    pub pattern: String,
    pub path_prefix: Option<String>,
    pub limit: u32,
}

pub struct Indexer {
    store: Store,
    root: PathBuf,
}

impl Indexer {
    pub fn new(store: Store, root: impl Into<PathBuf>) -> Self {
        Self {
            store,
            root: root.into(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn index_root(&self) -> Result<usize> {
        let build_id = self.store.begin_build(&self.root.to_string_lossy())?;
        // Keep previous symbols until we finish; then we could prune. For MVP overwrite by id.
        let mut count = 0usize;

        for entry in WalkDir::new(&self.root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let Some(lang) = language_for_path(path) else {
                continue;
            };
            if should_skip(path) {
                continue;
            }
            let rel = path
                .strip_prefix(&self.root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            let source = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            count += self.index_source(&build_id, lang, &rel, &source)?;
        }
        Ok(count)
    }

    fn index_source(
        &self,
        build_id: &str,
        language: &str,
        rel_path: &str,
        source: &str,
    ) -> Result<usize> {
        let tree = parse(language, source)?;
        let root = tree.root_node();
        let mut count = 0usize;
        match language {
            "python" => {
                count += self.walk_python(build_id, rel_path, source.as_bytes(), root, &[])?;
            }
            "csharp" => {
                count += self.walk_csharp(build_id, rel_path, source.as_bytes(), root, &[])?;
            }
            _ => {}
        }
        Ok(count)
    }

    fn walk_python(
        &self,
        build_id: &str,
        rel_path: &str,
        source: &[u8],
        node: Node,
        stack: &[&str],
    ) -> Result<usize> {
        let mut count = 0;
        let kind = node.kind();

        if kind == "class_definition" {
            if let Some(name) = child_text(node, "name", source) {
                let mut owned: Vec<String> = stack.iter().map(|s| (*s).to_string()).collect();
                owned.push(name);
                let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    count += self.walk_python(build_id, rel_path, source, child, &refs)?;
                }
                return Ok(count);
            }
        }

        if kind == "function_definition" || kind == "async_function_definition" {
            let name = child_text(node, "name", source).unwrap_or_else(|| "<anon>".into());
            let mut owned: Vec<String> = stack.iter().map(|s| (*s).to_string()).collect();
            owned.push(name.clone());
            let qualname = owned.join(".");
            let params = node.child_by_field_name("parameters");
            let param_count = params.map(|p| count_params(p, source)).unwrap_or(0);
            let complexity = cyclomatic(node, source);
            let callees = count_calls(node);
            let symbol = SymbolId::new("python", rel_path, &qualname);
            self.store.insert_indexed_symbol(
                build_id,
                &symbol,
                if stack.is_empty() {
                    "function"
                } else {
                    "method"
                },
                node.start_position().row as u32 + 1,
                node.end_position().row as u32 + 1,
                param_count,
                complexity,
                callees,
            )?;
            count += 1;
            let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                count += self.walk_python(build_id, rel_path, source, child, &refs)?;
            }
            return Ok(count);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            count += self.walk_python(build_id, rel_path, source, child, stack)?;
        }
        Ok(count)
    }

    fn walk_csharp(
        &self,
        build_id: &str,
        rel_path: &str,
        source: &[u8],
        node: Node,
        stack: &[&str],
    ) -> Result<usize> {
        let mut count = 0;
        let kind = node.kind();

        if kind == "namespace_declaration" || kind == "file_scoped_namespace_declaration" {
            if let Some(name) = namespace_name(node, source) {
                let mut owned: Vec<String> = stack.iter().map(|s| (*s).to_string()).collect();
                owned.push(name);
                let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    count += self.walk_csharp(build_id, rel_path, source, child, &refs)?;
                }
                return Ok(count);
            }
        }

        if kind == "class_declaration"
            || kind == "struct_declaration"
            || kind == "record_declaration"
        {
            if let Some(name) = child_text(node, "name", source) {
                let mut owned: Vec<String> = stack.iter().map(|s| (*s).to_string()).collect();
                owned.push(name);
                let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    count += self.walk_csharp(build_id, rel_path, source, child, &refs)?;
                }
                return Ok(count);
            }
        }

        if kind == "method_declaration"
            || kind == "constructor_declaration"
            || kind == "local_function_statement"
        {
            let name = child_text(node, "name", source).unwrap_or_else(|| ".ctor".into());
            let mut owned: Vec<String> = stack.iter().map(|s| (*s).to_string()).collect();
            owned.push(name.clone());
            let qualname = owned.join(".");
            let param_count = node
                .child_by_field_name("parameters")
                .map(|p| count_csharp_params(p))
                .unwrap_or(0);
            let complexity = cyclomatic(node, source);
            let callees = count_calls(node);
            let symbol = SymbolId::new("csharp", rel_path, &qualname);
            self.store.insert_indexed_symbol(
                build_id,
                &symbol,
                "method",
                node.start_position().row as u32 + 1,
                node.end_position().row as u32 + 1,
                param_count,
                complexity,
                callees,
            )?;
            count += 1;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            count += self.walk_csharp(build_id, rel_path, source, child, stack)?;
        }
        Ok(count)
    }

    pub fn structural_search(
        &self,
        req: &StructuralSearchRequest,
    ) -> Result<StructuralSearchResponse> {
        let language = normalize_language(&req.language)?;
        let limit = if req.limit == 0 { 50 } else { req.limit.min(200) };
        let pattern = req.pattern.trim();
        if pattern.is_empty() {
            return Err(IndexerError::InvalidPattern("empty pattern".into()));
        }

        // Normalize common metavariable sugar to tree-sitter query wildcards via text match.
        // Strategy: find candidate nodes whose structure loosely matches by scanning for
        // distinctive anchors extracted from the pattern.
        let mut matches = Vec::new();
        let mut truncated = false;

        let prefix = req
            .path_prefix
            .as_deref()
            .map(|p| p.trim_matches('/').to_string());

        for entry in WalkDir::new(&self.root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if language_for_path(path) != Some(language) {
                continue;
            }
            if should_skip(path) {
                continue;
            }
            let rel = path
                .strip_prefix(&self.root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            if let Some(pref) = &prefix {
                if !rel.starts_with(pref) {
                    continue;
                }
            }
            let source = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let tree = match parse(language, &source) {
                Ok(t) => t,
                Err(_) => continue,
            };

            let found = match_pattern(language, &tree, source.as_bytes(), pattern)?;
            for m in found {
                if matches.len() as u32 >= limit {
                    truncated = true;
                    break;
                }
                matches.push(StructuralMatch {
                    path: rel.clone(),
                    start_line: m.0,
                    end_line: m.1,
                    matched_text: truncate_text(&m.2, 240),
                });
            }
            if truncated {
                break;
            }
        }

        Ok(StructuralSearchResponse {
            language: language.to_string(),
            match_count: matches.len(),
            truncated,
            matches,
        })
    }
}

fn normalize_language(lang: &str) -> Result<&'static str> {
    match lang.to_ascii_lowercase().as_str() {
        "python" | "py" => Ok("python"),
        "csharp" | "cs" | "c#" => Ok("csharp"),
        other => Err(IndexerError::UnsupportedLanguage(other.to_string())),
    }
}

fn language_for_path(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("py") => Some("python"),
        Some("cs") => Some("csharp"),
        _ => None,
    }
}

fn should_skip(path: &Path) -> bool {
    path.components().any(|c| {
        matches!(
            c.as_os_str().to_str(),
            Some("node_modules" | "target" | ".git" | "bin" | "obj" | ".venv" | "venv" | "__pycache__")
        )
    })
}

fn parse(language: &str, source: &str) -> Result<Tree> {
    let mut parser = Parser::new();
    let lang: Language = match language {
        "python" => tree_sitter_python::LANGUAGE.into(),
        "csharp" => tree_sitter_c_sharp::LANGUAGE.into(),
        other => return Err(IndexerError::UnsupportedLanguage(other.into())),
    };
    parser
        .set_language(&lang)
        .map_err(|e| IndexerError::TreeSitter(e.to_string()))?;
    parser
        .parse(source, None)
        .ok_or_else(|| IndexerError::TreeSitter("parse returned None".into()))
}

fn child_text(node: Node, field: &str, source: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .map(|n| n.utf8_text(source).unwrap_or("").to_string())
}

fn namespace_name(node: Node, source: &[u8]) -> Option<String> {
    if let Some(n) = node.child_by_field_name("name") {
        return Some(n.utf8_text(source).unwrap_or("").to_string());
    }
    // file-scoped namespace: first identifier-ish child
    let mut c = node.walk();
    for child in node.children(&mut c) {
        if child.kind() == "identifier" || child.kind() == "qualified_name" {
            return Some(child.utf8_text(source).unwrap_or("").to_string());
        }
    }
    None
}

fn count_params(node: Node, source: &[u8]) -> u32 {
    let mut count = 0u32;
    let mut c = node.walk();
    for child in node.children(&mut c) {
        if child.kind() == "identifier"
            || child.kind() == "typed_parameter"
            || child.kind() == "default_parameter"
            || child.kind() == "list_splat_pattern"
            || child.kind() == "dictionary_splat_pattern"
        {
            let text = child.utf8_text(source).unwrap_or("");
            if text != "self" && text != "cls" {
                count += 1;
            }
        }
    }
    count
}

fn count_csharp_params(node: Node) -> u32 {
    let mut count = 0u32;
    let mut c = node.walk();
    for child in node.children(&mut c) {
        if child.kind() == "parameter" {
            count += 1;
        }
    }
    count
}

fn cyclomatic(node: Node, _source: &[u8]) -> u32 {
    let mut score = 1u32;
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        match n.kind() {
            "if_statement"
            | "elif_clause"
            | "for_statement"
            | "while_statement"
            | "with_statement"
            | "except_clause"
            | "conditional_expression"
            | "case_statement"
            | "switch_statement"
            | "catch_clause"
            | "for_each_statement"
            | "do_statement" => score += 1,
            "boolean_operator" | "binary_expression" => {
                // rough: and/or add complexity
                score += 1;
            }
            _ => {}
        }
        let mut c = n.walk();
        for child in n.children(&mut c) {
            stack.push(child);
        }
    }
    score
}

fn count_calls(node: Node) -> u32 {
    let mut count = 0u32;
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        if n.kind() == "call" || n.kind() == "invocation_expression" {
            count += 1;
        }
        let mut c = n.walk();
        for child in n.children(&mut c) {
            stack.push(child);
        }
    }
    count
}

/// Match pattern using tree-sitter queries for common shapes + text anchors.
fn match_pattern(
    language: &str,
    tree: &Tree,
    source: &[u8],
    pattern: &str,
) -> Result<Vec<(u32, u32, String)>> {
    let compact = collapse_ws(pattern);
    if let Some(query_src) = pattern_to_query(language, &compact) {
        if let Ok(results) = run_query(language, tree, source, &query_src) {
            if !results.is_empty() {
                return Ok(results);
            }
        }
    }
    // Fallback: node-kind / text structural scan
    Ok(fallback_scan(language, tree.root_node(), source, &compact))
}

fn pattern_to_query(language: &str, pattern: &str) -> Option<String> {
    let p = pattern.replace('\n', " ");
    match language {
        "python" => {
            if p.contains("async def") {
                return Some(
                    r#"
                    (async_function_definition) @m
                    "#
                    .into(),
                );
            }
            if p.starts_with("def ") || p.contains("def $") || p.contains("def execute") {
                // specific name?
                if let Some(name) = extract_def_name(&p) {
                    return Some(format!(
                        r#"(function_definition name: (identifier) @name (#eq? @name "{name}")) @m"#
                    ));
                }
                return Some(r#"(function_definition) @m"#.into());
            }
            if p.starts_with("except:") || p.contains("except:") {
                return Some(r#"(except_clause) @m"#.into());
            }
            if p.starts_with("print(") {
                return Some(
                    r#"
                    (call function: (identifier) @fn (#eq? @fn "print")) @m
                    "#
                    .into(),
                );
            }
            if p.contains("requests.get") {
                return Some(
                    r#"
                    (call
                      function: (attribute
                        object: (identifier) @obj
                        attribute: (identifier) @attr)
                      (#eq? @obj "requests")
                      (#eq? @attr "get")) @m
                    "#
                    .into(),
                );
            }
            if p.starts_with("open(") {
                return Some(
                    r#"
                    (call function: (identifier) @fn (#eq? @fn "open")) @m
                    "#
                    .into(),
                );
            }
            if p.starts_with("raise ") {
                if let Some(exc) = p.strip_prefix("raise ").and_then(|s| s.split('(').next()) {
                    let exc = exc.trim();
                    return Some(format!(
                        r#"(raise_statement (call function: (identifier) @fn (#eq? @fn "{exc}"))) @m"#
                    ));
                }
            }
            if p.starts_with("await ") {
                return Some(r#"(await) @m"#.into());
            }
            if p.contains("@$") || p.starts_with('@') {
                return Some(r#"(decorated_definition) @m"#.into());
            }
            None
        }
        "csharp" => {
            if p.contains("async ") && p.contains("(") {
                return Some(r#"(method_declaration) @m"#.into());
            }
            if p.contains("Console.WriteLine") {
                return Some(
                    r#"
                    (invocation_expression
                      function: (member_access_expression) @fn) @m
                    "#
                    .into(),
                );
            }
            None
        }
        _ => None,
    }
}

fn extract_def_name(pattern: &str) -> Option<String> {
    // def execute( or def $NAME(
    let rest = pattern.split("def ").nth(1)?;
    let name = rest.split('(').next()?.trim();
    if name.starts_with('$') || name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

fn run_query(
    language: &str,
    tree: &Tree,
    source: &[u8],
    query_src: &str,
) -> Result<Vec<(u32, u32, String)>> {
    let lang: Language = match language {
        "python" => tree_sitter_python::LANGUAGE.into(),
        "csharp" => tree_sitter_c_sharp::LANGUAGE.into(),
        other => return Err(IndexerError::UnsupportedLanguage(other.into())),
    };
    let query = Query::new(&lang, query_src)
        .map_err(|e| IndexerError::InvalidPattern(format!("{e:?}")))?;
    let mut cursor = QueryCursor::new();
    let mut out = Vec::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source);
    while let Some(m) = matches.next() {
        for cap in m.captures {
            if query.capture_names()[cap.index as usize] == "m" {
                let n = cap.node;
                let text = n.utf8_text(source).unwrap_or("").to_string();
                out.push((
                    n.start_position().row as u32 + 1,
                    n.end_position().row as u32 + 1,
                    text,
                ));
            }
        }
    }
    Ok(out)
}

fn fallback_scan(
    language: &str,
    root: Node,
    source: &[u8],
    pattern: &str,
) -> Vec<(u32, u32, String)> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    let needle = pattern
        .replace("$NAME", "")
        .replace("$$$ARGS", "")
        .replace("$$$BODY", "")
        .replace("$$$", "")
        .replace("$FUNC", "")
        .replace("$DECORATOR", "")
        .replace("$KEY", "");
    let needle = collapse_ws(&needle);

    while let Some(n) = stack.pop() {
        let text = n.utf8_text(source).unwrap_or("");
        let compact = collapse_ws(text);
        let interesting = match language {
            "python" => matches!(
                n.kind(),
                "function_definition"
                    | "async_function_definition"
                    | "call"
                    | "except_clause"
                    | "raise_statement"
                    | "decorated_definition"
                    | "await"
            ),
            "csharp" => matches!(
                n.kind(),
                "method_declaration" | "invocation_expression" | "constructor_declaration"
            ),
            _ => false,
        };
        if interesting && !needle.is_empty() {
            // Check anchors present
            let anchors: Vec<&str> = needle
                .split(|c: char| c == '(' || c == ':' || c == ' ')
                .filter(|s| s.len() > 1 && !s.starts_with('$'))
                .collect();
            if anchors.iter().all(|a| compact.contains(a)) {
                out.push((
                    n.start_position().row as u32 + 1,
                    n.end_position().row as u32 + 1,
                    text.to_string(),
                ));
            }
        }
        let mut c = n.walk();
        for child in n.children(&mut c) {
            stack.push(child);
        }
    }
    out
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_text(s: &str, max: usize) -> String {
    let flat = s.lines().take(8).collect::<Vec<_>>().join("\n");
    if flat.len() <= max {
        flat
    } else {
        format!("{}…", &flat[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn index_python_and_search_async() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("demo.py");
        let mut f = fs::File::create(&file).unwrap();
        writeln!(
            f,
            r#"
async def handle_checkout():
    pass

def execute(x):
    print(x)
    requests.get("http://x")

try:
    open("f")
except:
    pass
"#
        )
        .unwrap();

        let store = Store::open_in_memory().unwrap();
        let indexer = Indexer::new(store.clone(), dir.path());
        let n = indexer.index_root().unwrap();
        assert!(n >= 2);

        let resp = indexer
            .structural_search(&StructuralSearchRequest {
                language: "python".into(),
                pattern: "async def $NAME($$$ARGS): $$$BODY".into(),
                path_prefix: None,
                limit: 20,
            })
            .unwrap();
        assert!(resp.match_count >= 1);

        let resp2 = indexer
            .structural_search(&StructuralSearchRequest {
                language: "python".into(),
                pattern: "def execute($$$ARGS): $$$BODY".into(),
                path_prefix: None,
                limit: 20,
            })
            .unwrap();
        assert!(resp2.match_count >= 1);

        let resp3 = indexer
            .structural_search(&StructuralSearchRequest {
                language: "python".into(),
                pattern: "except: $$$BODY".into(),
                path_prefix: None,
                limit: 20,
            })
            .unwrap();
        assert!(resp3.match_count >= 1);
    }
}
