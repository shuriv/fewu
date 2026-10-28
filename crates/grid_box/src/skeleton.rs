//! Context compressors for feeding architecture to agents cheaply.
//!
//! `skeleton` reduces a source file to its top-level declarations, eliding
//! bodies, and `subgraph_bundle` combines a file with the skeletons of its
//! one-hop neighbours. Both are textual and language-agnostic heuristics, so
//! they need neither a language server nor a syntax tree.

use std::path::{Path, PathBuf};

/// The declaration keywords that are kept for a given language.
fn declaration_prefixes(extension: &str) -> &'static [&'static str] {
    match extension {
        "rs" => &[
            "pub fn ",
            "pub async fn ",
            "pub const fn ",
            "pub unsafe fn ",
            "fn ",
            "async fn ",
            "unsafe fn ",
            "pub struct ",
            "struct ",
            "pub enum ",
            "enum ",
            "pub trait ",
            "trait ",
            "pub union ",
            "union ",
            "impl ",
            "pub mod ",
            "mod ",
            "pub const ",
            "const ",
            "pub static ",
            "static ",
            "pub type ",
            "type ",
            "pub use ",
            "use ",
        ],
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => &[
            "export default ",
            "export async function ",
            "export function ",
            "export class ",
            "export interface ",
            "export type ",
            "export const ",
            "export let ",
            "export var ",
            "export enum ",
            "async function ",
            "function ",
            "class ",
            "interface ",
            "type ",
            "const ",
            "let ",
            "var ",
            "enum ",
        ],
        "py" => &["def ", "async def ", "class ", "async class "],
        "go" => &["func ", "type ", "const ", "var "],
        "c" | "h" | "cc" | "cpp" | "hpp" => &["struct ", "class ", "enum ", "union ", "typedef "],
        _ => &[],
    }
}

/// Declarations whose bodies are elided and replaced with `{ /* ... */ }`.
fn is_body_declaration(prefix: &str) -> bool {
    prefix.contains("fn ")
        || prefix == "impl "
        || prefix.contains("class ")
        || prefix.contains("interface ")
        || prefix.contains("function ")
        || prefix.contains("def ")
        || prefix.contains("func ")
        || prefix.contains("trait ")
}

/// Reduces `text` to a skeleton of its top-level declarations.
pub fn skeleton(extension: &str, text: &str) -> String {
    let prefixes = declaration_prefixes(extension);
    if prefixes.is_empty() {
        return text.to_string();
    }

    let ignore = ["use ", "pub use "];
    let mut output = String::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let Some(prefix) = prefixes.iter().find(|prefix| trimmed.starts_with(**prefix)) else {
            continue;
        };

        if ignore.contains(prefix) {
            output.push_str(line);
            output.push('\n');
        } else if is_body_declaration(prefix) {
            match line.find('{') {
                Some(brace) => {
                    output.push_str(&line[..=brace]);
                    output.push_str(" /* ... */ }");
                    output.push('\n');
                }
                None => {
                    output.push_str(&line.trim_end());
                    output.push_str(" { /* ... */ }");
                    output.push('\n');
                }
            }
        } else if line.trim_end().ends_with(';') || line.trim_end().ends_with('}') {
            output.push_str(line);
            output.push('\n');
        } else {
            output.push_str(&line.trim_end());
            output.push('\n');
        }
    }
    output
}

/// Reads `path` and returns its skeleton, if the file has a known language.
pub fn skeleton_for_file(path: &Path) -> Option<String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase();
    let text = std::fs::read_to_string(path).ok()?;
    Some(skeleton(&extension, &text))
}

/// Bundles a target file's full source with the skeletons of its neighbours.
pub fn subgraph_bundle(target: &Path, neighbours: &[std::path::PathBuf]) -> Option<String> {
    let mut bundle = String::new();
    bundle.push_str("// === target: ");
    bundle.push_str(&target.display().to_string());
    bundle.push_str(" ===\n");
    bundle.push_str(&std::fs::read_to_string(target).ok()?);
    bundle.push('\n');

    for neighbour in neighbours {
        let Some(skeleton) = skeleton_for_file(neighbour) else {
            continue;
        };
        bundle.push_str("\n// === signatures: ");
        bundle.push_str(&neighbour.display().to_string());
        bundle.push_str(" ===\n");
        bundle.push_str(&skeleton);
    }
    Some(bundle)
}

/// Bundles a target file's full source with the skeletons of its one-hop
/// import neighbours, resolved on disk relative to the target.
pub fn subgraph_for_file(target: &Path) -> Option<String> {
    let text = std::fs::read_to_string(target).ok()?;
    let directory = target.parent()?;
    let mut neighbours = Vec::new();
    for import in crate::indexer::extract_imports(target, &text) {
        if let Some(path) = resolve_neighbour(directory, &import)
            && path != target
        {
            neighbours.push(path);
        }
    }
    subgraph_bundle(target, &neighbours)
}

fn resolve_neighbour(directory: &Path, import: &str) -> Option<PathBuf> {
    let import = import.replace("::", "/");
    let candidate = directory.join(import.trim_start_matches('/'));
    let mut bases = vec![candidate.clone()];
    bases.push(candidate.join("mod"));
    bases.push(candidate.join("index"));

    for base in bases {
        if base.is_file() {
            return Some(base);
        }
        for extension in ["rs", "ts", "tsx", "js", "jsx", "mjs", "py", "go", "c", "h"] {
            let with_extension = base.with_extension(extension);
            if with_extension.is_file() {
                return Some(with_extension);
            }
        }
    }
    None
}

/// Extracts the file path from a `skeleton:<path>` or `subgraph:<path>` token,
/// accepting an optional leading `@`.
pub fn parse_compressor_token(token: &str) -> Option<(&str, &str)> {
    let token = token.strip_prefix('@').unwrap_or(token);
    let (kind, path) = token.split_once(':')?;
    match kind {
        "skeleton" | "subgraph" if !path.is_empty() => Some((kind, path)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_skeleton_elides_bodies() {
        let source = "use std::fmt;\n\npub struct Foo {\n    bar: u32,\n}\n\nimpl Foo {\n    pub fn new() -> Self {\n        Self { bar: 0 }\n    }\n}\n";
        let skeleton = skeleton("rs", source);
        assert!(skeleton.contains("pub struct Foo {"));
        assert!(skeleton.contains("impl Foo { /* ... */ }"));
        assert!(skeleton.contains("pub fn new() -> Self { /* ... */ }"));
        assert!(!skeleton.contains("Self { bar: 0 }"));
    }

    #[test]
    fn parses_compressor_tokens() {
        assert_eq!(parse_compressor_token("@skeleton:src/a.rs"), Some(("skeleton", "src/a.rs")));
        assert_eq!(parse_compressor_token("subgraph:a.rs"), Some(("subgraph", "a.rs")));
        assert_eq!(parse_compressor_token("@file:a.rs"), None);
    }
}
