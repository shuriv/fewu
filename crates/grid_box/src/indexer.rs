//! A zero-LSP syntactic indexer for the Grid-Box graph.
//!
//! It walks the project's source files on disk and pairs each one with the
//! files it imports. Import extraction is a per-language syntactic scan of the
//! file text rather than a full parse, so it needs neither a running language
//! server nor a syntax tree.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use project::ProjectPath;

use crate::{EdgeKind, GridEdge, GridNode, NodeId};

/// Directories whose contents are never indexed.
const EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "target",
    "node_modules",
    "dist",
    "vendor",
    "__pycache__",
];

/// Source file extensions the indexer scans.
const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "go", "c", "h", "cc", "cpp", "hpp",
];

/// A project file considered for indexing.
#[derive(Clone, Debug)]
pub struct SourceFile {
    pub id: NodeId,
    pub project_path: ProjectPath,
    pub abs_path: PathBuf,
}

/// Returns true when `path` is a source file that should be indexed.
pub fn is_indexable_source(path: &Path) -> bool {
    if path.components().any(|component| match component {
        Component::Normal(name) => name
            .to_str()
            .is_some_and(|name| EXCLUDED_DIRS.contains(&name)),
        _ => false,
    }) {
        return false;
    }

    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .is_some_and(|extension| SOURCE_EXTENSIONS.contains(&extension.as_str()))
}

/// Reads the given source files and returns the graph they form.
///
/// This runs off the foreground thread and performs blocking file reads, so it
/// is only ever called from a background task.
pub fn index(files: Vec<SourceFile>) -> (Vec<GridNode>, Vec<GridEdge>) {
    let mut keys = HashMap::with_capacity(files.len());
    let mut nodes = Vec::with_capacity(files.len());
    for file in &files {
        let relative = file.project_path.path.as_std_path();
        keys.insert(module_key(relative), file.id);
        nodes.push(GridNode {
            id: file.id,
            path: file.project_path.clone(),
            label: relative
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default()
                .into(),
        });
    }

    let mut edges = Vec::new();
    let mut seen = HashSet::new();
    for file in &files {
        let Ok(text) = std::fs::read_to_string(&file.abs_path) else {
            continue;
        };
        let relative = file.project_path.path.as_std_path();
        let from_dir = relative.parent().unwrap_or_else(|| Path::new(""));
        for import in extract_imports(&file.abs_path, &text) {
            let Some(target) = resolve_import(&import, from_dir, &keys) else {
                continue;
            };
            if target != file.id && seen.insert((file.id, target)) {
                edges.push(GridEdge {
                    source: file.id,
                    target,
                    kind: EdgeKind::Import,
                });
            }
        }
    }

    (nodes, edges)
}

/// Extracts the raw import/module specifiers from a file's text.
pub(crate) fn extract_imports(path: &Path, text: &str) -> Vec<String> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("rs") => extract_rust_imports(text),
        Some("ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs") => extract_js_imports(text),
        Some("py") => extract_python_imports(text),
        Some("go") => extract_go_imports(text),
        Some("c" | "h" | "cc" | "cpp" | "hpp") => extract_c_includes(text),
        _ => Vec::new(),
    }
}

fn extract_rust_imports(text: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("use ") {
            let path = rest.split(';').next().unwrap_or(rest);
            let path = path.split('{').next().unwrap_or(path);
            if !path.is_empty() {
                imports.push(path.replace("::", "/").trim().to_string());
            }
        } else if let Some(rest) = line.strip_prefix("mod ") {
            let name = rest.split(';').next().unwrap_or(rest).trim();
            if !name.is_empty() {
                imports.push(name.to_string());
            }
        }
    }
    imports
}

fn extract_js_imports(text: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if !(line.starts_with("import ") || line.starts_with("export ") || line.starts_with("require("))
        {
            continue;
        }
        for quote in ['\'', '"'] {
            if let Some(start) = line.find(quote)
                && let Some(end) = line[start + 1..].find(quote)
            {
                let specifier = &line[start + 1..start + 1 + end];
                if specifier.starts_with('.') {
                    imports.push(specifier.to_string());
                }
            }
        }
    }
    imports
}

fn extract_python_imports(text: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("from ") {
            let module = rest.split_whitespace().next().unwrap_or("");
            let module = module.trim_matches('.');
            if !module.is_empty() {
                imports.push(module.replace('.', "/"));
            }
        } else if let Some(rest) = line.strip_prefix("import ") {
            for module in rest.split(',') {
                let module = module.split_whitespace().next().unwrap_or("").trim();
                if !module.is_empty() && !module.starts_with('.') {
                    imports.push(module.replace('.', "/"));
                }
            }
        }
    }
    imports
}

fn extract_go_imports(text: &str) -> Vec<String> {
    let mut imports = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        let line = line.trim();
        if in_block {
            if line == ")" {
                in_block = false;
            } else if let Some(path) = quoted_value(line) {
                imports.push(path.rsplit('/').next().unwrap_or(path.as_str()).to_string());
            }
            continue;
        }
        if line.starts_with("import (") {
            in_block = true;
        } else if let Some(rest) = line.strip_prefix("import ")
            && let Some(path) = quoted_value(rest)
        {
            imports.push(path.rsplit('/').next().unwrap_or(path.as_str()).to_string());
        }
    }
    imports
}

/// Returns the contents of the first double-quoted string in `text`.
fn quoted_value(text: &str) -> Option<String> {
    let start = text.find('"')?;
    let end = text[start + 1..].find('"')?;
    Some(text[start + 1..start + 1 + end].to_string())
}

fn extract_c_includes(text: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("#include")
            && let Some(path) = quoted_value(rest)
        {
            imports.push(path);
        }
    }
    imports
}

/// Resolves a specifier to an indexed file, if one matches.
fn resolve_import(
    specifier: &str,
    from_dir: &Path,
    keys: &HashMap<String, NodeId>,
) -> Option<NodeId> {
    let specifier = specifier.trim();
    if specifier.is_empty() {
        return None;
    }

    let raw = if specifier.starts_with('.') {
        normalize_path(&from_dir.join(specifier))
    } else {
        specifier.replace('\\', "/")
    };

    // Try the module itself and its `mod`/`index` entry points.
    let mut candidates = vec![raw.clone()];
    candidates.push(format!("{raw}/mod"));
    candidates.push(format!("{raw}/index"));
    if let Some(parent) = raw.rsplit_once('/').map(|(parent, _)| parent.to_string()) {
        candidates.push(parent);
    }

    for candidate in candidates {
        let key = module_key(Path::new(&candidate));
        if let Some(id) = keys.get(&key) {
            return Some(*id);
        }
        let suffix = format!("/{key}");
        if let Some((_, id)) = keys.iter().find(|(candidate, _)| candidate.ends_with(&suffix)) {
            return Some(*id);
        }
    }

    None
}

/// Normalizes a path into a lowercased, extension-less key used for matching.
fn module_key(path: &Path) -> String {
    let mut value = path.to_string_lossy().replace('\\', "/");
    for extension in SOURCE_EXTENSIONS {
        let suffix = format!(".{extension}");
        if value.ends_with(&suffix) {
            value.truncate(value.len() - suffix.len());
            break;
        }
    }
    value
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

/// Removes `.` and `..` components from a path without touching the filesystem.
fn normalize_path(path: &Path) -> String {
    let mut result: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(name) => result.push(name.to_string_lossy().into_owned()),
            Component::RootDir | Component::Prefix(_) => {}
        }
    }
    result.join("/")
}
