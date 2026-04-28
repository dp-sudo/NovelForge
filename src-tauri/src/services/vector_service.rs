use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::fs_utils::{read_text_if_exists, write_file_atomic};
use crate::services::project_service::get_project_id;

const VECTOR_DIM: usize = 96;
const VECTOR_INDEX_RELATIVE_PATH: &str = "database/vector-index.json";
const MAX_CHUNKS_PER_CHAPTER: usize = 24;
const MAX_TOTAL_CHUNKS: usize = 2000;
const MAX_SNIPPET_CHARS: usize = 220;
const MIN_SCORE: f32 = 0.12;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorSearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub body_snippet: String,
    pub rank: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VectorIndexFile {
    dim: usize,
    chunks: Vec<VectorChunk>,
}

impl Default for VectorIndexFile {
    fn default() -> Self {
        Self {
            dim: VECTOR_DIM,
            chunks: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VectorChunk {
    chapter_id: String,
    title: String,
    snippet: String,
    vector: Vec<f32>,
}

#[derive(Default)]
pub struct VectorService;

impl VectorService {
    pub fn rebuild_index(&self, project_root: &str) -> Result<usize, AppErrorDto> {
        let root = Path::new(project_root);
        let conn = open_database(root).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "Cannot open project database", false)
                .with_detail(err.to_string())
        })?;
        let project_id = get_project_id(&conn)?;

        let mut stmt = conn
            .prepare(
                "
                SELECT id, title, COALESCE(summary, ''), content_path
                FROM chapters
                WHERE project_id = ?1 AND is_deleted = 0
                ORDER BY chapter_index
                ",
            )
            .map_err(|err| {
                AppErrorDto::new("VECTOR_INDEX_QUERY_FAILED", "Cannot query chapter data", true)
                    .with_detail(err.to_string())
            })?;

        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|err| {
                AppErrorDto::new("VECTOR_INDEX_QUERY_FAILED", "Cannot query chapter data", true)
                    .with_detail(err.to_string())
            })?;

        let mut chunks = Vec::new();
        for row in rows {
            let (chapter_id, title, summary, content_path) = row.map_err(|err| {
                AppErrorDto::new("VECTOR_INDEX_QUERY_FAILED", "Cannot parse chapter data", true)
                    .with_detail(err.to_string())
            })?;
            let raw_content = fs::read_to_string(root.join(content_path)).unwrap_or_default();
            let chapter_text = strip_chapter_markdown(&raw_content);
            let parts = split_chapter_chunks(&summary, &chapter_text);
            for part in parts.into_iter().take(MAX_CHUNKS_PER_CHAPTER) {
                let vector = embed_text(&part);
                if vector.is_empty() {
                    continue;
                }
                chunks.push(VectorChunk {
                    chapter_id: chapter_id.clone(),
                    title: title.clone(),
                    snippet: truncate_chars(&part, MAX_SNIPPET_CHARS),
                    vector,
                });
                if chunks.len() >= MAX_TOTAL_CHUNKS {
                    break;
                }
            }
            if chunks.len() >= MAX_TOTAL_CHUNKS {
                break;
            }
        }

        let index = VectorIndexFile {
            dim: VECTOR_DIM,
            chunks,
        };
        let payload = serde_json::to_string(&index).map_err(|err| {
            AppErrorDto::new("VECTOR_INDEX_WRITE_FAILED", "Cannot serialize vector index", true)
                .with_detail(err.to_string())
        })?;
        write_file_atomic(&self.index_file_path(root), &payload).map_err(|err| {
            AppErrorDto::new("VECTOR_INDEX_WRITE_FAILED", "Cannot write vector index file", true)
                .with_detail(err.to_string())
        })?;

        Ok(index.chunks.len())
    }

    pub fn search(
        &self,
        project_root: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<VectorSearchResult>, AppErrorDto> {
        if query.trim().is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let query_vec = embed_text(query);
        if query_vec.is_empty() {
            return Ok(Vec::new());
        }

        let root = Path::new(project_root);
        let mut index = self.load_index(root)?;
        if index.chunks.is_empty() {
            let _ = self.rebuild_index(project_root)?;
            index = self.load_index(root)?;
        }

        let mut scored = index
            .chunks
            .iter()
            .map(|chunk| {
                let score = dot_product(&query_vec, &chunk.vector);
                (score, chunk)
            })
            .filter(|(score, _)| *score >= MIN_SCORE)
            .collect::<Vec<_>>();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut seen = HashSet::<(String, String)>::new();
        let mut out = Vec::new();
        for (score, chunk) in scored {
            if out.len() >= limit {
                break;
            }
            let key = (chunk.chapter_id.clone(), chunk.snippet.clone());
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key);
            out.push(VectorSearchResult {
                entity_type: "chapter".to_string(),
                entity_id: chunk.chapter_id.clone(),
                title: chunk.title.clone(),
                body_snippet: chunk.snippet.clone(),
                rank: score as f64,
            });
        }
        Ok(out)
    }

    fn load_index(&self, root: &Path) -> Result<VectorIndexFile, AppErrorDto> {
        let index_path = self.index_file_path(root);
        let raw = read_text_if_exists(&index_path).map_err(|err| {
            AppErrorDto::new("VECTOR_INDEX_READ_FAILED", "Cannot read vector index file", true)
                .with_detail(err.to_string())
        })?;
        let Some(raw) = raw else {
            return Ok(VectorIndexFile::default());
        };
        serde_json::from_str::<VectorIndexFile>(&raw).map_err(|err| {
            AppErrorDto::new("VECTOR_INDEX_READ_FAILED", "Cannot parse vector index file", true)
                .with_detail(err.to_string())
        })
    }

    fn index_file_path(&self, root: &Path) -> PathBuf {
        root.join(VECTOR_INDEX_RELATIVE_PATH)
    }
}

fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for ch in text.chars().take(max_chars) {
        out.push(ch);
    }
    out
}

fn split_chapter_chunks(summary: &str, body: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut seen = HashSet::new();
    let normalized_summary = summary.trim();
    if !normalized_summary.is_empty() {
        let summary_text = normalized_summary.to_string();
        seen.insert(summary_text.clone());
        chunks.push(summary_text);
    }

    for part in body.split("\n\n") {
        let trimmed = part.trim();
        if trimmed.chars().count() < 8 {
            continue;
        }
        for sentence in split_long_piece(trimmed) {
            if sentence.chars().count() < 8 {
                continue;
            }
            if seen.insert(sentence.clone()) {
                chunks.push(sentence);
            }
            if chunks.len() >= MAX_CHUNKS_PER_CHAPTER {
                return chunks;
            }
        }
    }
    chunks
}

fn split_long_piece(text: &str) -> Vec<String> {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.len() <= MAX_SNIPPET_CHARS {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut start = 0usize;
    while start < chars.len() {
        let end = (start + MAX_SNIPPET_CHARS).min(chars.len());
        let piece = chars[start..end].iter().collect::<String>().trim().to_string();
        if !piece.is_empty() {
            out.push(piece);
        }
        start = end;
    }
    out
}

fn strip_chapter_markdown(raw: &str) -> String {
    let mut body = raw;
    if raw.starts_with("---") {
        if let Some(end) = raw[3..].find("\n---") {
            let offset = 3 + end + "\n---".len();
            body = raw.get(offset..).unwrap_or(raw);
        }
    }

    let mut out = String::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            out.push('\n');
            continue;
        }
        if trimmed.starts_with('#') || trimmed.starts_with("```") {
            continue;
        }
        out.push_str(trimmed);
        out.push('\n');
    }
    out
}

fn embed_text(text: &str) -> Vec<f32> {
    let mut frequency = HashMap::<String, u32>::new();
    for token in tokenize(text) {
        *frequency.entry(token).or_insert(0) += 1;
    }
    if frequency.is_empty() {
        return Vec::new();
    }

    let mut vector = vec![0f32; VECTOR_DIM];
    for (token, count) in frequency {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        token.hash(&mut hasher);
        let hash = hasher.finish();
        let index = (hash as usize) % VECTOR_DIM;
        let sign = if hash & (1 << 63) == 0 { 1.0 } else { -1.0 };
        let weight = 1.0 + (count as f32).ln();
        vector[index] += sign * weight;
    }

    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut ascii = String::new();
    let mut cjk = String::new();

    for ch in text.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            if !cjk.is_empty() {
                flush_cjk(&mut cjk, &mut tokens);
            }
            ascii.push(ch);
            continue;
        }

        if !ascii.is_empty() {
            if ascii.len() >= 2 {
                tokens.push(ascii.clone());
            }
            ascii.clear();
        }

        if is_cjk(ch) {
            cjk.push(ch);
        } else if !cjk.is_empty() {
            flush_cjk(&mut cjk, &mut tokens);
        }
    }

    if !ascii.is_empty() && ascii.len() >= 2 {
        tokens.push(ascii);
    }
    if !cjk.is_empty() {
        flush_cjk(&mut cjk, &mut tokens);
    }
    tokens
}

fn flush_cjk(cjk: &mut String, tokens: &mut Vec<String>) {
    let chars = cjk.chars().collect::<Vec<_>>();
    if chars.len() == 1 {
        tokens.push(chars[0].to_string());
    } else {
        for window in chars.windows(2) {
            tokens.push(window.iter().collect());
        }
        if chars.len() >= 3 {
            for window in chars.windows(3) {
                tokens.push(window.iter().collect());
            }
        }
    }
    cjk.clear();
}

fn is_cjk(ch: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&ch) || ('\u{3400}'..='\u{4DBF}').contains(&ch)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use uuid::Uuid;

    use crate::services::chapter_service::{ChapterInput, ChapterService};
    use crate::services::project_service::{CreateProjectInput, ProjectService};

    use super::VectorService;

    fn create_temp_workspace() -> PathBuf {
        let workspace =
            std::env::temp_dir().join(format!("novelforge-vector-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&workspace).expect("create temp workspace");
        workspace
    }

    #[test]
    fn rebuild_and_search_vector_index_succeeds() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let chapter_service = ChapterService;
        let vector_service = VectorService;

        let created = project_service
            .create_project(CreateProjectInput {
                name: "向量检索测试".to_string(),
                author: None,
                genre: "科幻".to_string(),
                target_words: Some(60000),
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("create project");

        let chapter = chapter_service
            .create_chapter(
                &created.project_root,
                ChapterInput {
                    title: "第一章 雨夜追踪".to_string(),
                    summary: Some("主角追查失踪线索".to_string()),
                    target_words: Some(1800),
                    status: None,
                },
            )
            .expect("create chapter");

        chapter_service
            .save_chapter_content(
                &created.project_root,
                &chapter.id,
                "夜潮覆盖旧港，沈烬沿着失踪者留下的光痕追查真相。\n\n他在废弃码头发现了关键的航运日志。",
            )
            .expect("save chapter content");

        let indexed = vector_service
            .rebuild_index(&created.project_root)
            .expect("rebuild vector index");
        assert!(indexed > 0);

        let results = vector_service
            .search(&created.project_root, "失踪线索 旧港", 5)
            .expect("vector search");
        assert!(!results.is_empty());
        assert_eq!(results[0].entity_id, chapter.id);

        let _ = fs::remove_dir_all(workspace);
    }
}
