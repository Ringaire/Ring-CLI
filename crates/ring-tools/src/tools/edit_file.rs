use async_trait::async_trait;
use ring_core::tools::{Tool, ToolContext, ToolResult};
use serde_json::{json, Value};
use similar::{ChangeTag, TextDiff};

pub struct EditFileTool;

/// 纯函数编辑结果：写盘内容与用于 diff 的归一化文本分离，便于同步单测。
struct EditOutcome {
    /// 已按原文件行尾风格还原、可直接写盘的内容。
    write_content: String,
    /// 归一化（LF）后的原文，供 diff 展示。
    orig_n: String,
    /// 归一化（LF）后的替换结果，供 diff 展示。
    replaced_n: String,
}

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str { "edit_file" }
    fn description(&self) -> &str {
        "Perform an exact string replacement in a file. Fails if old_string is not found or is ambiguous (found more than once)."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path":       { "type": "string", "description": "File path" },
                "old_string": { "type": "string", "description": "Exact text to replace" },
                "new_string": { "type": "string", "description": "Replacement text" },
                "replace_all": { "type": "boolean", "description": "Replace all occurrences (default false)" }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    async fn execute(&self, input: Value, ctx: &ToolContext) -> ToolResult {
        let path_str  = match input["path"].as_str()       { Some(p) => p, None => return ToolResult::err("missing 'path'") };
        let old_str   = match input["old_string"].as_str() { Some(s) => s, None => return ToolResult::err("missing 'old_string'") };
        let new_str   = match input["new_string"].as_str() { Some(s) => s, None => return ToolResult::err("missing 'new_string'") };
        let replace_all = input["replace_all"].as_bool().unwrap_or(false);

        let path = resolve_path(&ctx.cwd, path_str);

        let original = match tokio::fs::read_to_string(&path).await {
            Ok(s) => s,
            Err(e) => return ToolResult::err(format!("{}: {}", path.display(), e)),
        };

        let outcome = match apply_edit(&original, old_str, new_str, replace_all) {
            Ok(o) => o,
            Err(msg) => return ToolResult::err(format!("{msg} in {}", path.display())),
        };

        if let Err(e) = tokio::fs::write(&path, outcome.write_content.as_bytes()).await {
            return ToolResult::err(format!("{}: {}", path.display(), e));
        }

        let diff = TextDiff::from_lines(&outcome.orig_n, &outcome.replaced_n);
        let mut diff_out = String::new();
        for change in diff.iter_all_changes() {
            let prefix = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal  => " ",
            };
            diff_out.push_str(&format!("{}{}", prefix, change));
        }

        ToolResult::ok_text(format!("edited {}\n\n{}", path.display(), diff_out))
    }
}

/// 行尾归一化的精确替换核心。
///
/// `read_file` 用 `lines()` 剥除了 `\r`，模型回传的 `old_string` 通常是 LF；而磁盘文件
/// 可能是 CRLF（Windows 检出 / `core.autocrlf`）。纯字节匹配会因 `\r` 失配，导致
/// "old_string not found" 反复空转。故统一归一化为 LF 后匹配/替换，写回时按原文件行尾
/// 风格还原，保持仓库一致性。
fn apply_edit(original: &str, old_str: &str, new_str: &str, replace_all: bool) -> Result<EditOutcome, String> {
    let is_crlf = original.contains("\r\n");
    let orig_n = normalize_eol(original);
    let old_n  = normalize_eol(old_str);
    let new_n  = normalize_eol(new_str);

    if old_n == new_n {
        return Err("old_string equals new_string — no change made".into());
    }

    let count = orig_n.matches(&old_n).count();
    if count == 0 {
        return Err("old_string not found".into());
    }
    if !replace_all && count > 1 {
        return Err(format!(
            "old_string found {count} times — use replace_all=true or provide more context to make it unique"
        ));
    }

    let replaced_n = if replace_all {
        orig_n.replace(&old_n, &new_n)
    } else {
        orig_n.replacen(&old_n, &new_n, 1)
    };

    let write_content = if is_crlf {
        replaced_n.replace('\n', "\r\n")
    } else {
        replaced_n.clone()
    };

    Ok(EditOutcome { write_content, orig_n, replaced_n })
}

fn resolve_path(cwd: &std::path::Path, p: &str) -> std::path::PathBuf {
    let pb = std::path::PathBuf::from(p);
    if pb.is_absolute() { pb } else { cwd.join(pb) }
}

/// 将任意行尾（\r\n / 孤立 \r / \n）统一为 \n，供精确匹配使用。
fn normalize_eol(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lf_file_plain_replace() {
        let o = apply_edit("aaa\nbbb\nccc\n", "bbb", "BBB", false).unwrap();
        assert_eq!(o.write_content, "aaa\nBBB\nccc\n");
        assert!(!o.write_content.contains('\r'));
    }

    #[test]
    fn crlf_file_single_line_old_lf_preserves_crlf() {
        // 模型回传 LF 的 old_string，磁盘是 CRLF —— 修复前这里会 not found。
        let o = apply_edit("aaa\r\nbbb\r\nccc\r\n", "bbb", "BBB", false).unwrap();
        assert_eq!(o.write_content, "aaa\r\nBBB\r\nccc\r\n");
    }

    #[test]
    fn crlf_file_multiline_old_lf_matches() {
        // 多行 old_string（LF）必须能匹配 CRLF 文件中的对应多行片段。
        let o = apply_edit("x\r\naaa\r\nbbb\r\ny\r\n", "aaa\nbbb", "AAA\nBBB", false).unwrap();
        assert_eq!(o.write_content, "x\r\nAAA\r\nBBB\r\ny\r\n");
    }

    #[test]
    fn crlf_file_old_with_crlf_also_matches() {
        // 即便 old_string 自带 \r\n 也应归一化后匹配。
        let o = apply_edit("aaa\r\nbbb\r\n", "aaa\r\nbbb", "zzz", false).unwrap();
        assert_eq!(o.write_content, "zzz\r\n");
    }

    #[test]
    fn not_found_returns_err() {
        assert!(apply_edit("aaa\nbbb\n", "zzz", "q", false).is_err());
    }

    #[test]
    fn ambiguous_without_replace_all_returns_err() {
        assert!(apply_edit("x\nx\n", "x", "y", false).is_err());
    }

    #[test]
    fn replace_all_replaces_every_occurrence() {
        let o = apply_edit("x\r\nx\r\n", "x", "y", true).unwrap();
        assert_eq!(o.write_content, "y\r\ny\r\n");
    }

    #[test]
    fn equal_old_new_returns_err() {
        assert!(apply_edit("aaa\n", "aaa", "aaa", false).is_err());
    }

    #[test]
    fn diff_uses_normalized_lines() {
        let o = apply_edit("aaa\r\nbbb\r\n", "bbb", "BBB", false).unwrap();
        assert_eq!(o.orig_n, "aaa\nbbb\n");
        assert_eq!(o.replaced_n, "aaa\nBBB\n");
    }
}
