use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::errors::AppErrorDto;
use crate::services::chapter_service::{ChapterInput, ChapterService};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFileEntry {
    pub file_name: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportInput {
    pub project_root: String,
    pub files: Vec<ImportFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedChapter {
    pub id: String,
    pub title: String,
    pub chapter_index: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported_count: usize,
    pub chapters: Vec<ImportedChapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetExtractionCandidate {
    pub label: String,
    pub asset_type: String,
    pub occurrences: i64,
    pub confidence: f32,
    pub evidence: String,
}

fn rollback_created_chapters(
    chapter_service: &ChapterService,
    project_root: &str,
    created_chapter_ids: &[String],
) {
    for chapter_id in created_chapter_ids.iter().rev() {
        let _ = chapter_service.delete_chapter(project_root, chapter_id);
    }
}

fn infer_title(file_name: &str) -> String {
    let stem = match file_name.rfind('.') {
        Some(pos) => &file_name[..pos],
        None => file_name,
    };
    stem.replace('_', " ").replace('-', " ").trim().to_string()
}

fn is_supported_file(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    lower.ends_with(".txt") || lower.ends_with(".md")
}

fn normalize_key(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn is_cjk(ch: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&ch)
}

fn is_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '·' || ch == '-' || ch == '_' || is_cjk(ch)
}

fn is_stopword(token: &str) -> bool {
    const STOPWORDS: &[&str] = &[
        "我们", "你们", "他们", "自己", "时候", "已经", "因为", "所以", "如果", "但是", "不是",
        "一个", "这个", "那个", "然后", "什么", "怎么", "可以", "没有", "就是", "还是", "今天",
        "昨天", "刚才", "这里", "那里", "事情", "问题", "可能",
    ];
    STOPWORDS.contains(&token)
}

fn push_candidate_token(raw: &str, target: &mut Vec<String>) {
    let token = raw.trim_matches(|ch: char| ch == '-' || ch == '_' || ch == '·');
    let len = token.chars().count();
    if len < 2 || len > 12 {
        return;
    }
    if token.chars().all(|ch| ch.is_ascii_digit()) {
        return;
    }
    if is_stopword(token) {
        return;
    }
    if !token
        .chars()
        .any(|ch| is_cjk(ch) || ch.is_ascii_alphabetic())
    {
        return;
    }
    target.push(token.to_string());
}

fn collect_tokens(content: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let flush = |raw: &str, buffer: &mut Vec<String>| {
        if raw.is_empty() {
            return;
        }
        push_candidate_token(raw, buffer);

        if raw.chars().all(is_cjk) {
            let chars: Vec<char> = raw.chars().collect();
            if chars.len() >= 2 {
                let max_n = chars.len().min(4);
                for n in 2..=max_n {
                    for start in 0..=(chars.len() - n) {
                        let sub = chars[start..start + n].iter().collect::<String>();
                        push_candidate_token(&sub, buffer);
                    }
                }
            }
        }
    };

    for ch in content.chars() {
        if is_token_char(ch) {
            current.push(ch);
        } else if !current.is_empty() {
            flush(&current, &mut tokens);
            current.clear();
        }
    }
    if !current.is_empty() {
        flush(&current, &mut tokens);
    }
    tokens
}

fn infer_asset_type(token: &str) -> &'static str {
    if token.ends_with("城")
        || token.ends_with("镇")
        || token.ends_with("村")
        || token.ends_with("国")
        || token.ends_with("州")
        || token.ends_with("山")
        || token.ends_with("湖")
        || token.ends_with("海")
        || token.ends_with("宫")
    {
        return "location";
    }
    if token.ends_with("宗")
        || token.ends_with("门")
        || token.ends_with("派")
        || token.ends_with("帮")
        || token.ends_with("会")
        || token.ends_with("团")
        || token.ends_with("司")
        || token.ends_with("局")
        || token.ends_with("学院")
        || token.ends_with("公司")
    {
        return "organization";
    }
    if token.ends_with("术")
        || token.ends_with("法")
        || token.ends_with("诀")
        || token.ends_with("阵")
        || token.ends_with("体系")
    {
        return "world_rule";
    }
    if token.chars().count() <= 4 {
        return "character";
    }
    "term"
}

fn strip_frontmatter(content: &str) -> &str {
    if !content.starts_with("---\n") {
        return content;
    }
    if let Some(offset) = content[4..].find("\n---\n") {
        return &content[(offset + 9)..];
    }
    content
}

fn extract_evidence_snippet(content: &str, token: &str) -> String {
    if let Some(byte_index) = content.find(token) {
        let left_chars = content[..byte_index].chars().count();
        let token_chars = token.chars().count();
        let all_chars: Vec<char> = content.chars().collect();
        let start = left_chars.saturating_sub(18);
        let end = (left_chars + token_chars + 18).min(all_chars.len());
        return all_chars[start..end]
            .iter()
            .collect::<String>()
            .replace('\n', " ")
            .trim()
            .to_string();
    }
    token.to_string()
}

pub fn extract_asset_candidates(
    content: &str,
    existing_labels: &[String],
    limit: usize,
) -> Vec<AssetExtractionCandidate> {
    if limit == 0 {
        return Vec::new();
    }

    let plain = strip_frontmatter(content);
    let tokens = collect_tokens(plain);
    if tokens.is_empty() {
        return Vec::new();
    }

    let existing: HashSet<String> = existing_labels
        .iter()
        .map(|item| normalize_key(item))
        .collect();
    let mut counts: HashMap<String, i64> = HashMap::new();
    for token in tokens {
        *counts.entry(token).or_insert(0) += 1;
    }

    let mut ranked = counts
        .into_iter()
        .filter(|(label, count)| *count >= 2 && !existing.contains(&normalize_key(label)))
        .collect::<Vec<_>>();
    ranked.sort_by(|(label_a, count_a), (label_b, count_b)| {
        count_b
            .cmp(count_a)
            .then_with(|| label_b.chars().count().cmp(&label_a.chars().count()))
    });

    ranked
        .into_iter()
        .take(limit)
        .map(|(label, occurrences)| AssetExtractionCandidate {
            asset_type: infer_asset_type(&label).to_string(),
            confidence: ((occurrences as f32 * 0.12) + 0.25).min(0.95),
            evidence: extract_evidence_snippet(plain, &label),
            label,
            occurrences,
        })
        .collect()
}

#[derive(Default)]
pub struct ImportService;

impl ImportService {
    /// Import multiple TXT/MD files as chapters in batch.
    pub fn import_files(&self, input: ImportInput) -> Result<ImportResult, AppErrorDto> {
        if input.files.is_empty() {
            return Err(AppErrorDto::new(
                "IMPORT_FILES_REQUIRED",
                "请至少选择一个 TXT/MD 文件",
                true,
            ));
        }

        let chapter_service = ChapterService;
        let mut chapters = Vec::new();
        let mut created_chapter_ids = Vec::new();

        for file_entry in &input.files {
            if !is_supported_file(&file_entry.file_name) {
                rollback_created_chapters(
                    &chapter_service,
                    &input.project_root,
                    &created_chapter_ids,
                );
                return Err(AppErrorDto::new(
                    "IMPORT_FILE_TYPE_UNSUPPORTED",
                    "仅支持导入 TXT 或 MD 文件",
                    true,
                )
                .with_detail(file_entry.file_name.clone()));
            }
            let title = infer_title(&file_entry.file_name);

            let record = match chapter_service.create_chapter(
                &input.project_root,
                ChapterInput {
                    title: title.clone(),
                    summary: Some(format!("从 {} 导入", file_entry.file_name)),
                    target_words: None,
                    status: Some("drafting".into()),
                },
            ) {
                Ok(record) => record,
                Err(err) => {
                    rollback_created_chapters(
                        &chapter_service,
                        &input.project_root,
                        &created_chapter_ids,
                    );
                    return Err(err);
                }
            };
            created_chapter_ids.push(record.id.clone());

            if let Err(err) = chapter_service.save_chapter_content(
                &input.project_root,
                &record.id,
                &file_entry.content,
            ) {
                rollback_created_chapters(
                    &chapter_service,
                    &input.project_root,
                    &created_chapter_ids,
                );
                return Err(err);
            }

            chapters.push(ImportedChapter {
                id: record.id,
                title,
                chapter_index: record.chapter_index,
            });
        }

        Ok(ImportResult {
            imported_count: chapters.len(),
            chapters,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::extract_asset_candidates;

    #[test]
    fn extract_asset_candidates_filters_existing_labels() {
        let content =
            "林夜走进玄霄城。玄霄城的夜风里，林夜听见天衡司的钟声。天衡司的执事在城门等他。";
        let candidates = extract_asset_candidates(content, &[String::from("林夜")], 10);
        assert!(candidates.iter().any(|item| item.label == "玄霄城"));
        assert!(candidates.iter().any(|item| item.label == "天衡司"));
        assert!(candidates.iter().all(|item| item.label != "林夜"));
    }

    #[test]
    fn extract_asset_candidates_requires_repeated_terms() {
        let content = "青石街很长。林远路过青石街，又回到青石街。";
        let candidates = extract_asset_candidates(content, &[], 10);
        assert!(candidates.iter().any(|item| item.label == "青石街"));
        assert!(candidates.iter().all(|item| item.label != "林远"));
    }
}
