use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::errors::AppErrorDto;
use crate::infra::database::open_database;
use crate::infra::fs_utils::{write_bytes_atomic, write_file_atomic};
use crate::infra::path_utils::resolve_project_relative_path;
use crate::infra::time::now_iso;

const DEFAULT_EXPORT_LANGUAGE: &str = "zh-CN";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ExportOptions {
    pub include_chapter_title: Option<bool>,
    pub include_chapter_summary: Option<bool>,
    pub separate_by_volume: Option<bool>,
    pub include_world_settings: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOutput {
    pub output_path: String,
}

#[derive(Debug)]
struct ChapterExportRow {
    title: String,
    summary: Option<String>,
    content_path: String,
    volume_id: Option<String>,
    volume_title: Option<String>,
}

#[derive(Debug, Clone)]
struct RenderedChapter {
    title: String,
    summary: Option<String>,
    body: String,
    volume_id: Option<String>,
    volume_title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportFormat {
    Txt,
    Md,
    Docx,
    Pdf,
    Epub,
}

impl ExportFormat {
    fn from_raw(value: &str) -> Result<Self, AppErrorDto> {
        match value {
            "txt" => Ok(Self::Txt),
            "md" => Ok(Self::Md),
            "docx" => Ok(Self::Docx),
            "pdf" => Ok(Self::Pdf),
            "epub" => Ok(Self::Epub),
            _ => Err(
                AppErrorDto::new("EXPORT_FORMAT_UNSUPPORTED", "导出格式不支持", true)
                    .with_detail(value.to_string())
                    .with_suggested_action("仅支持 txt / md / docx / pdf / epub"),
            ),
        }
    }
}

#[derive(Default)]
pub struct ExportService;

impl ExportService {
    pub fn export_chapter(
        &self,
        project_root: &str,
        chapter_id: &str,
        format: &str,
        output_path: &str,
        options: Option<ExportOptions>,
    ) -> Result<ExportOutput, AppErrorDto> {
        let format = ExportFormat::from_raw(format)?;
        let opts = options.unwrap_or(ExportOptions {
            include_chapter_title: None,
            include_chapter_summary: None,
            separate_by_volume: None,
            include_world_settings: None,
        });

        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
                .with_suggested_action("请检查 database/project.sqlite 是否存在并可读写")
        })?;
        let (project_id, project_name) = load_project_identity(&conn)?;
        let project_language = load_project_language(project_root_path);

        let chapter = conn
            .query_row(
                "
        SELECT ch.title, ch.summary, ch.content_path, ch.volume_id, v.title
        FROM chapters ch
        LEFT JOIN volumes v ON ch.volume_id = v.id
        WHERE ch.project_id = ?1 AND ch.id = ?2 AND ch.is_deleted = 0
        ",
                params![project_id, chapter_id],
                |row| {
                    Ok(ChapterExportRow {
                        title: row.get(0)?,
                        summary: row.get::<_, Option<String>>(1)?,
                        content_path: row.get(2)?,
                        volume_id: row.get::<_, Option<String>>(3)?,
                        volume_title: row.get::<_, Option<String>>(4)?,
                    })
                },
            )
            .optional()
            .map_err(|err| {
                AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查章节数据是否完整")
            })?
            .ok_or_else(|| AppErrorDto::new("CHAPTER_NOT_FOUND", "章节不存在", true))?;

        let chapter_file = resolve_export_path(project_root_path, &chapter.content_path)?;
        let content = fs::read_to_string(chapter_file).map_err(|err| {
            AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查章节文件是否存在")
        })?;
        let chapter_payload = RenderedChapter {
            title: chapter.title,
            summary: chapter.summary,
            body: strip_frontmatter(&content),
            volume_id: chapter.volume_id,
            volume_title: chapter.volume_title,
        };

        let resolved = write_export_output(
            project_root_path,
            output_path,
            format,
            &[chapter_payload],
            &opts,
            &project_name,
            &project_language,
        )?;

        Ok(ExportOutput {
            output_path: resolved.to_string_lossy().to_string(),
        })
    }

    pub fn export_book(
        &self,
        project_root: &str,
        format: &str,
        output_path: &str,
        options: Option<ExportOptions>,
    ) -> Result<ExportOutput, AppErrorDto> {
        let format = ExportFormat::from_raw(format)?;
        let opts = options.unwrap_or(ExportOptions {
            include_chapter_title: None,
            include_chapter_summary: None,
            separate_by_volume: None,
            include_world_settings: None,
        });

        let project_root_path = Path::new(project_root);
        let conn = open_database(project_root_path).map_err(|err| {
            AppErrorDto::new("DB_OPEN_FAILED", "数据库打开失败", false)
                .with_detail(err.to_string())
                .with_suggested_action("请检查 database/project.sqlite 是否存在并可读写")
        })?;
        let (project_id, project_name) = load_project_identity(&conn)?;
        let project_language = load_project_language(project_root_path);

        let mut stmt = conn
            .prepare(
                "
        SELECT ch.title, ch.summary, ch.content_path, ch.volume_id, v.title
        FROM chapters ch
        LEFT JOIN volumes v ON ch.volume_id = v.id
        WHERE ch.project_id = ?1 AND ch.is_deleted = 0
        ORDER BY ch.chapter_index
        ",
            )
            .map_err(|err| {
                AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查章节数据是否完整")
            })?;
        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok(ChapterExportRow {
                    title: row.get(0)?,
                    summary: row.get::<_, Option<String>>(1)?,
                    content_path: row.get(2)?,
                    volume_id: row.get::<_, Option<String>>(3)?,
                    volume_title: row.get::<_, Option<String>>(4)?,
                })
            })
            .map_err(|err| {
                AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查章节查询语句")
            })?;

        let mut chapters = Vec::new();
        for row in rows {
            let chapter = row.map_err(|err| {
                AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查章节读取过程")
            })?;
            let chapter_file = resolve_export_path(project_root_path, &chapter.content_path)?;
            let content = fs::read_to_string(chapter_file).map_err(|err| {
                AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查章节文件是否存在")
            })?;
            chapters.push(RenderedChapter {
                title: chapter.title,
                summary: chapter.summary,
                body: strip_frontmatter(&content),
                volume_id: chapter.volume_id,
                volume_title: chapter.volume_title,
            });
        }

        if chapters.is_empty() {
            return Err(AppErrorDto::new(
                "EXPORT_EMPTY_BOOK",
                "当前项目没有可导出章节",
                true,
            ));
        }

        let resolved = write_export_output(
            project_root_path,
            output_path,
            format,
            &chapters,
            &opts,
            &project_name,
            &project_language,
        )?;

        Ok(ExportOutput {
            output_path: resolved.to_string_lossy().to_string(),
        })
    }
}

fn load_project_identity(conn: &Connection) -> Result<(String, String), AppErrorDto> {
    conn.query_row("SELECT id, name FROM projects LIMIT 1", [], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })
    .map_err(|err| {
        AppErrorDto::new("PROJECT_NOT_INITIALIZED", "项目未初始化", false)
            .with_detail(err.to_string())
            .with_suggested_action("请重新创建或打开有效项目")
    })
}

fn load_project_language(project_root: &Path) -> String {
    let project_json_path = project_root.join("project.json");
    let raw = match fs::read_to_string(project_json_path) {
        Ok(raw) => raw,
        Err(_) => return DEFAULT_EXPORT_LANGUAGE.to_string(),
    };

    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(_) => return DEFAULT_EXPORT_LANGUAGE.to_string(),
    };

    let language = value
        .get("settings")
        .and_then(|settings| settings.get("language"))
        .and_then(|lang| lang.as_str())
        .map(str::trim)
        .filter(|lang| !lang.is_empty())
        .unwrap_or(DEFAULT_EXPORT_LANGUAGE);
    language.to_string()
}

fn normalize_export_language(language: &str) -> String {
    let trimmed = language.trim();
    if trimmed.is_empty() {
        DEFAULT_EXPORT_LANGUAGE.to_string()
    } else {
        trimmed.to_string()
    }
}

fn write_export_output(
    project_root: &Path,
    output_path: &str,
    format: ExportFormat,
    chapters: &[RenderedChapter],
    options: &ExportOptions,
    project_name: &str,
    project_language: &str,
) -> Result<PathBuf, AppErrorDto> {
    let resolved = resolve_output_path(project_root, output_path);
    if let Some(parent) = resolved.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
                .with_detail(err.to_string())
                .with_suggested_action("请检查导出目录写入权限")
        })?;
    }

    match format {
        ExportFormat::Txt | ExportFormat::Md => {
            let content = render_text_export(chapters, options, format);
            write_file_atomic(&resolved, &content).map_err(|err| {
                AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查导出文件写入权限")
            })?;
        }
        ExportFormat::Docx => write_docx_output(&resolved, chapters, options)?,
        ExportFormat::Pdf => write_pdf_output(&resolved, chapters, options)?,
        ExportFormat::Epub => {
            write_epub_output(&resolved, chapters, options, project_name, project_language)?
        }
    }

    Ok(resolved)
}

fn resolve_output_path(project_root: &Path, output_path: &str) -> PathBuf {
    let path = Path::new(output_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    }
}

fn resolve_export_path(project_root: &Path, stored_path: &str) -> Result<PathBuf, AppErrorDto> {
    resolve_project_relative_path(project_root, stored_path).map_err(|detail| {
        AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
            .with_detail(detail)
            .with_suggested_action("请检查项目数据库中的路径字段")
    })
}

fn render_text_export(
    chapters: &[RenderedChapter],
    options: &ExportOptions,
    format: ExportFormat,
) -> String {
    let divider = if format == ExportFormat::Md {
        "\n\n---\n\n"
    } else {
        "\n\n====================\n\n"
    };

    let mut chunks = Vec::new();
    let mut current_volume: Option<&str> = None;
    for chapter in chapters {
        if options.separate_by_volume.unwrap_or(false) {
            let volume_key = chapter.volume_id.as_deref();
            if volume_key != current_volume {
                if let Some(volume_title) = chapter.volume_title.as_deref() {
                    chunks.push(if format == ExportFormat::Md {
                        format!("## {volume_title}")
                    } else {
                        format!("【{volume_title}】")
                    });
                }
                current_volume = volume_key;
            }
        }

        chunks.push(render_chapter(
            &chapter.title,
            chapter.summary.as_deref(),
            &chapter.body,
            options,
            format,
        ));
    }
    chunks.join(divider)
}

fn write_docx_output(
    output_path: &Path,
    chapters: &[RenderedChapter],
    options: &ExportOptions,
) -> Result<(), AppErrorDto> {
    let temp_output = temporary_output_path(output_path);
    let file = fs::File::create(&temp_output).map_err(|err| {
        AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
            .with_detail(err.to_string())
            .with_suggested_action("请检查导出文件写入权限")
    })?;
    let mut zip = ZipWriter::new(file);
    let zip_options = FileOptions::<()>::default().compression_method(CompressionMethod::Deflated);

    let mut body = String::new();
    let mut current_volume: Option<&str> = None;
    for (index, chapter) in chapters.iter().enumerate() {
        if index > 0 {
            body.push_str("<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>");
        }
        if options.separate_by_volume.unwrap_or(false) {
            let volume_key = chapter.volume_id.as_deref();
            if volume_key != current_volume {
                if let Some(volume_title) = chapter.volume_title.as_deref() {
                    body.push_str(&docx_paragraph(volume_title, true));
                }
                current_volume = volume_key;
            }
        }
        if options.include_chapter_title.unwrap_or(true) {
            body.push_str(&docx_paragraph(&chapter.title, true));
        }
        if options.include_chapter_summary.unwrap_or(false) {
            if let Some(summary) = chapter.summary.as_deref() {
                body.push_str(&docx_paragraph(&format!("摘要：{summary}"), false));
            }
        }
        for line in chapter.body.lines() {
            body.push_str(&docx_paragraph(line.trim_end(), false));
        }
        if chapter.body.trim().is_empty() {
            body.push_str("<w:p/>");
        }
    }
    if body.is_empty() {
        body.push_str("<w:p/>");
    }

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;

    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;

    let document_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    {body}
    <w:sectPr>
      <w:pgSz w:w="11906" w:h="16838"/>
      <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="708" w:footer="708" w:gutter="0"/>
    </w:sectPr>
  </w:body>
</w:document>"#
    );

    zip_write_entry(
        &mut zip,
        "[Content_Types].xml",
        zip_options,
        content_types.as_bytes(),
        "请检查 DOCX 写入权限",
    )?;
    zip_write_entry(
        &mut zip,
        "_rels/.rels",
        zip_options,
        rels.as_bytes(),
        "请检查 DOCX 写入权限",
    )?;
    zip_write_entry(
        &mut zip,
        "word/document.xml",
        zip_options,
        document_xml.as_bytes(),
        "请检查 DOCX 写入权限",
    )?;
    zip.finish().map_err(|err| {
        AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
            .with_detail(err.to_string())
            .with_suggested_action("请检查 DOCX 文件完整性")
    })?;
    commit_temporary_output(&temp_output, output_path)?;

    Ok(())
}

fn docx_paragraph(text: &str, bold: bool) -> String {
    if text.trim().is_empty() {
        return "<w:p/>".to_string();
    }
    let escaped = xml_escape(text);
    if bold {
        format!(
            "<w:p><w:r><w:rPr><w:b/></w:rPr><w:t xml:space=\"preserve\">{escaped}</w:t></w:r></w:p>"
        )
    } else {
        format!("<w:p><w:r><w:t xml:space=\"preserve\">{escaped}</w:t></w:r></w:p>")
    }
}

fn write_pdf_output(
    output_path: &Path,
    chapters: &[RenderedChapter],
    options: &ExportOptions,
) -> Result<(), AppErrorDto> {
    let lines = collect_export_lines(chapters, options, false);
    let bytes = build_pdf_bytes(&lines);
    write_bytes_atomic(output_path, &bytes).map_err(|err| {
        AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
            .with_detail(err.to_string())
            .with_suggested_action("请检查 PDF 文件写入权限")
    })
}

fn build_pdf_bytes(lines: &[String]) -> Vec<u8> {
    let safe_lines = if lines.is_empty() {
        vec![" ".to_string()]
    } else {
        lines.to_vec()
    };
    let per_page = 40usize;
    let chunks = safe_lines.chunks(per_page).collect::<Vec<_>>();

    let mut objects: Vec<String> = vec![
        String::new(),
        String::new(),
        "<< /Type /Font /Subtype /Type0 /BaseFont /STSong-Light /Encoding /UniGB-UCS2-H /DescendantFonts [4 0 R] >>".to_string(),
        "<< /Type /Font /Subtype /CIDFontType0 /BaseFont /STSong-Light /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 4 >> /DW 1000 >>".to_string(),
    ];
    let mut page_ids = Vec::new();

    for chunk in chunks {
        let page_id = objects.len() + 1;
        let content_id = page_id + 1;
        page_ids.push(page_id);

        let stream = build_pdf_page_stream(chunk);
        let content_obj = format!(
            "<< /Length {} >>\nstream\n{}endstream",
            stream.len(),
            stream
        );
        let page_obj = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
            content_id
        );
        objects.push(page_obj);
        objects.push(content_obj);
    }

    objects[0] = "<< /Type /Catalog /Pages 2 0 R >>".to_string();
    let kids = page_ids
        .iter()
        .map(|id| format!("{id} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    objects[1] = format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        kids,
        page_ids.len()
    );

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets = vec![0usize];
    for (index, obj) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, obj).as_bytes());
    }

    let xref_offset = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    bytes.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        bytes.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref_offset
        )
        .as_bytes(),
    );
    bytes
}

fn build_pdf_page_stream(lines: &[String]) -> String {
    let mut stream = String::from("BT\n/F1 12 Tf\n1 0 0 1 40 800 Tm\n");
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            stream.push_str("0 -18 Td\n");
        }
        let display = if line.trim().is_empty() { " " } else { line };
        stream.push_str(&format!("<{}> Tj\n", encode_pdf_hex(display)));
    }
    stream.push_str("ET\n");
    stream
}

fn encode_pdf_hex(text: &str) -> String {
    let mut encoded = String::new();
    for ch in text.chars() {
        let code = if (ch as u32) <= 0xFFFF {
            ch as u16
        } else {
            0x003F
        };
        encoded.push_str(&format!("{code:04X}"));
    }
    if encoded.is_empty() {
        encoded.push_str("0020");
    }
    encoded
}

fn write_epub_output(
    output_path: &Path,
    chapters: &[RenderedChapter],
    options: &ExportOptions,
    project_name: &str,
    project_language: &str,
) -> Result<(), AppErrorDto> {
    let temp_output = temporary_output_path(output_path);
    let file = fs::File::create(&temp_output).map_err(|err| {
        AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
            .with_detail(err.to_string())
            .with_suggested_action("请检查导出文件写入权限")
    })?;
    let mut zip = ZipWriter::new(file);

    let stored = FileOptions::<()>::default().compression_method(CompressionMethod::Stored);
    zip_write_entry(
        &mut zip,
        "mimetype",
        stored,
        b"application/epub+zip",
        "请检查 EPUB 写入权限",
    )?;

    let compressed = FileOptions::<()>::default().compression_method(CompressionMethod::Deflated);
    let epub_language = normalize_export_language(project_language);
    let container_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    zip_write_entry(
        &mut zip,
        "META-INF/container.xml",
        compressed,
        container_xml.as_bytes(),
        "请检查 EPUB 写入权限",
    )?;

    let mut manifest_items = Vec::new();
    let mut spine_items = Vec::new();
    let mut nav_links = Vec::new();

    for (index, chapter) in chapters.iter().enumerate() {
        let id = format!("chapter-{}", index + 1);
        let file_name = format!("{id}.xhtml");
        let title = chapter.title.trim();

        manifest_items.push(format!(
            "<item id=\"{id}\" href=\"{file_name}\" media-type=\"application/xhtml+xml\"/>"
        ));
        spine_items.push(format!("<itemref idref=\"{id}\"/>"));
        nav_links.push(format!(
            "<li><a href=\"{file_name}\">{}</a></li>",
            xml_escape(title)
        ));

        let chapter_xhtml = build_epub_chapter_xhtml(chapter, options, &epub_language);
        zip_write_entry(
            &mut zip,
            &format!("OEBPS/{file_name}"),
            compressed,
            chapter_xhtml.as_bytes(),
            "请检查 EPUB 写入权限",
        )?;
    }

    let nav_xhtml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="{}">
  <head><title>目录</title></head>
  <body>
    <nav epub:type="toc" xmlns:epub="http://www.idpf.org/2007/ops">
      <h1>目录</h1>
      <ol>{}</ol>
    </nav>
  </body>
</html>"#,
        xml_escape(&epub_language),
        nav_links.join("")
    );
    zip_write_entry(
        &mut zip,
        "OEBPS/nav.xhtml",
        compressed,
        nav_xhtml.as_bytes(),
        "请检查 EPUB 写入权限",
    )?;

    let package_id = Uuid::new_v4().to_string();
    let modified_at = now_iso();
    let opf = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" unique-identifier="bookid" xmlns="http://www.idpf.org/2007/opf" xml:lang="{}">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="bookid">urn:uuid:{package_id}</dc:identifier>
    <dc:title>{}</dc:title>
    <dc:language>{}</dc:language>
    <meta property="dcterms:modified">{modified_at}</meta>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    {}
  </manifest>
  <spine>
    {}
  </spine>
</package>"#,
        xml_escape(&epub_language),
        xml_escape(project_name),
        xml_escape(&epub_language),
        manifest_items.join(""),
        spine_items.join("")
    );
    zip_write_entry(
        &mut zip,
        "OEBPS/content.opf",
        compressed,
        opf.as_bytes(),
        "请检查 EPUB 写入权限",
    )?;

    zip.finish().map_err(|err| {
        AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
            .with_detail(err.to_string())
            .with_suggested_action("请检查 EPUB 文件完整性")
    })?;
    commit_temporary_output(&temp_output, output_path)?;
    Ok(())
}

fn temporary_output_path(output_path: &Path) -> PathBuf {
    let file_name = output_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("export");
    let temp_name = format!("{file_name}.{}.tmp", Uuid::new_v4());
    match output_path.parent() {
        Some(parent) => parent.join(temp_name),
        None => PathBuf::from(temp_name),
    }
}

fn commit_temporary_output(temp_path: &Path, target_path: &Path) -> Result<(), AppErrorDto> {
    match fs::rename(temp_path, target_path) {
        Ok(()) => Ok(()),
        Err(_) => {
            let _ = fs::remove_file(target_path);
            fs::rename(temp_path, target_path).map_err(|err| {
                AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
                    .with_detail(err.to_string())
                    .with_suggested_action("请检查导出目录写入权限")
            })
        }
    }
}

fn build_epub_chapter_xhtml(
    chapter: &RenderedChapter,
    options: &ExportOptions,
    language: &str,
) -> String {
    let mut body_parts = Vec::new();
    if options.include_chapter_title.unwrap_or(true) {
        body_parts.push(format!("<h1>{}</h1>", xml_escape(&chapter.title)));
    }
    if options.include_chapter_summary.unwrap_or(false) {
        if let Some(summary) = chapter.summary.as_deref() {
            body_parts.push(format!(
                "<p><strong>摘要：</strong>{}</p>",
                xml_escape(summary)
            ));
        }
    }
    for line in chapter.body.lines() {
        let text = line.trim();
        if text.is_empty() {
            continue;
        }
        body_parts.push(format!("<p>{}</p>", xml_escape(text)));
    }
    if body_parts.is_empty() {
        body_parts.push("<p> </p>".to_string());
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="{}">
  <head><title>{}</title></head>
  <body>{}</body>
</html>"#,
        xml_escape(language),
        xml_escape(&chapter.title),
        body_parts.join("")
    )
}

fn collect_export_lines(
    chapters: &[RenderedChapter],
    options: &ExportOptions,
    markdown: bool,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_volume: Option<&str> = None;
    for chapter in chapters {
        if options.separate_by_volume.unwrap_or(false) {
            let volume_key = chapter.volume_id.as_deref();
            if volume_key != current_volume {
                if let Some(volume_title) = chapter.volume_title.as_deref() {
                    lines.push(if markdown {
                        format!("## {volume_title}")
                    } else {
                        format!("【{volume_title}】")
                    });
                    lines.push(String::new());
                }
                current_volume = volume_key;
            }
        }
        if options.include_chapter_title.unwrap_or(true) {
            lines.push(if markdown {
                format!("# {}", chapter.title)
            } else {
                chapter.title.to_string()
            });
            lines.push(String::new());
        }
        if options.include_chapter_summary.unwrap_or(false) {
            if let Some(summary) = chapter.summary.as_deref() {
                lines.push(if markdown {
                    format!("> {summary}")
                } else {
                    format!("摘要：{summary}")
                });
                lines.push(String::new());
            }
        }
        for line in chapter.body.lines() {
            lines.push(line.trim_end().to_string());
        }
        lines.push(String::new());
        lines.push(String::new());
    }
    while matches!(lines.last(), Some(last) if last.is_empty()) {
        lines.pop();
    }
    lines
}

fn strip_frontmatter(content: &str) -> String {
    if !content.starts_with("---\n") {
        return content.to_string();
    }
    match content[4..].find("\n---\n") {
        Some(offset) => content[(offset + 9)..].trim().to_string(),
        None => content.to_string(),
    }
}

fn render_chapter(
    title: &str,
    summary: Option<&str>,
    body: &str,
    options: &ExportOptions,
    format: ExportFormat,
) -> String {
    let mut lines = Vec::new();
    if options.include_chapter_title.unwrap_or(true) {
        lines.push(if format == ExportFormat::Md {
            format!("# {title}")
        } else {
            title.to_string()
        });
    }
    if options.include_chapter_summary.unwrap_or(false) {
        if let Some(summary) = summary {
            lines.push(if format == ExportFormat::Md {
                format!("> {summary}")
            } else {
                format!("摘要：{summary}")
            });
        }
    }
    lines.push(body.trim().to_string());
    lines.join("\n\n")
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn zip_write_entry(
    zip: &mut ZipWriter<fs::File>,
    path: &str,
    options: FileOptions<()>,
    content: &[u8],
    suggested_action: &str,
) -> Result<(), AppErrorDto> {
    zip.start_file(path, options).map_err(|err| {
        AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
            .with_detail(err.to_string())
            .with_suggested_action(suggested_action)
    })?;
    zip.write_all(content).map_err(|err| {
        AppErrorDto::new("EXPORT_FAILED", "导出失败", true)
            .with_detail(err.to_string())
            .with_suggested_action(suggested_action)
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Read;
    use std::path::PathBuf;

    use serde_json::Value;
    use uuid::Uuid;
    use zip::read::ZipArchive;

    use super::{ExportOptions, ExportService};
    use crate::services::chapter_service::{ChapterInput, ChapterService};
    use crate::services::project_service::{CreateProjectInput, ProjectService};

    fn create_temp_workspace() -> PathBuf {
        let workspace =
            std::env::temp_dir().join(format!("novelforge-rust-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&workspace).expect("create temp workspace");
        workspace
    }

    fn remove_temp_workspace(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn export_chapter_and_book_succeeds() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let chapter_service = ChapterService;
        let export_service = ExportService;
        let project = project_service
            .create_project(CreateProjectInput {
                name: "导出测试".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project created");
        let chapter = chapter_service
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第一章".to_string(),
                    summary: Some("摘要".to_string()),
                    target_words: None,
                    status: None,
                },
            )
            .expect("chapter created");
        chapter_service
            .save_chapter_content(&project.project_root, &chapter.id, "正文内容")
            .expect("save chapter content");

        let chapter_out = workspace.join("chapter.md");
        export_service
            .export_chapter(
                &project.project_root,
                &chapter.id,
                "md",
                &chapter_out.to_string_lossy(),
                Some(ExportOptions {
                    include_chapter_title: Some(true),
                    include_chapter_summary: Some(true),
                    separate_by_volume: None,
                    include_world_settings: None,
                }),
            )
            .expect("export chapter");
        assert!(chapter_out.exists());

        let book_out = workspace.join("book.txt");
        export_service
            .export_book(
                &project.project_root,
                "txt",
                &book_out.to_string_lossy(),
                Some(ExportOptions {
                    include_chapter_title: Some(true),
                    include_chapter_summary: Some(false),
                    separate_by_volume: None,
                    include_world_settings: None,
                }),
            )
            .expect("export book");
        let payload = fs::read_to_string(book_out).expect("read book");
        assert!(payload.contains("第一章"));

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn export_docx_pdf_epub_succeeds() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let chapter_service = ChapterService;
        let export_service = ExportService;
        let project = project_service
            .create_project(CreateProjectInput {
                name: "导出格式测试".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project created");
        let chapter = chapter_service
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "第一章".to_string(),
                    summary: Some("摘要".to_string()),
                    target_words: None,
                    status: None,
                },
            )
            .expect("chapter created");
        chapter_service
            .save_chapter_content(&project.project_root, &chapter.id, "夜雨落在青瓦上。")
            .expect("save chapter content");

        let docx_out = workspace.join("book.docx");
        export_service
            .export_book(
                &project.project_root,
                "docx",
                &docx_out.to_string_lossy(),
                Some(ExportOptions {
                    include_chapter_title: Some(true),
                    include_chapter_summary: Some(true),
                    separate_by_volume: None,
                    include_world_settings: None,
                }),
            )
            .expect("export docx");
        let docx_bytes = fs::read(&docx_out).expect("read docx");
        assert!(docx_bytes.starts_with(b"PK"));

        let pdf_out = workspace.join("book.pdf");
        export_service
            .export_book(
                &project.project_root,
                "pdf",
                &pdf_out.to_string_lossy(),
                Some(ExportOptions {
                    include_chapter_title: Some(true),
                    include_chapter_summary: Some(false),
                    separate_by_volume: None,
                    include_world_settings: None,
                }),
            )
            .expect("export pdf");
        let pdf_bytes = fs::read(&pdf_out).expect("read pdf");
        assert!(pdf_bytes.starts_with(b"%PDF-1.4"));

        let epub_out = workspace.join("book.epub");
        export_service
            .export_book(
                &project.project_root,
                "epub",
                &epub_out.to_string_lossy(),
                Some(ExportOptions {
                    include_chapter_title: Some(true),
                    include_chapter_summary: Some(true),
                    separate_by_volume: None,
                    include_world_settings: None,
                }),
            )
            .expect("export epub");
        let epub_bytes = fs::read(&epub_out).expect("read epub");
        assert!(epub_bytes.starts_with(b"PK"));

        remove_temp_workspace(&workspace);
    }

    #[test]
    fn export_epub_uses_project_language_from_project_json() {
        let workspace = create_temp_workspace();
        let project_service = ProjectService;
        let chapter_service = ChapterService;
        let export_service = ExportService;
        let project = project_service
            .create_project(CreateProjectInput {
                name: "语言导出测试".to_string(),
                author: None,
                genre: "测试".to_string(),
                target_words: None,
                save_directory: workspace.to_string_lossy().to_string(),
            })
            .expect("project created");
        let chapter = chapter_service
            .create_chapter(
                &project.project_root,
                ChapterInput {
                    title: "Chapter One".to_string(),
                    summary: Some("summary".to_string()),
                    target_words: None,
                    status: None,
                },
            )
            .expect("chapter created");
        chapter_service
            .save_chapter_content(&project.project_root, &chapter.id, "Body text.")
            .expect("save chapter content");

        let project_json_path = PathBuf::from(&project.project_root).join("project.json");
        let mut project_json: Value = serde_json::from_str(
            &fs::read_to_string(&project_json_path).expect("read project json"),
        )
        .expect("parse project json");
        project_json["settings"]["language"] = Value::String("en-US".to_string());
        fs::write(
            &project_json_path,
            serde_json::to_string_pretty(&project_json).expect("serialize project json"),
        )
        .expect("write project json");

        let epub_out = workspace.join("book-lang.epub");
        export_service
            .export_book(
                &project.project_root,
                "epub",
                &epub_out.to_string_lossy(),
                Some(ExportOptions {
                    include_chapter_title: Some(true),
                    include_chapter_summary: Some(true),
                    separate_by_volume: None,
                    include_world_settings: None,
                }),
            )
            .expect("export epub");

        let file = fs::File::open(&epub_out).expect("open epub");
        let mut archive = ZipArchive::new(file).expect("read epub archive");

        let mut content_opf = String::new();
        archive
            .by_name("OEBPS/content.opf")
            .expect("content.opf exists")
            .read_to_string(&mut content_opf)
            .expect("read content.opf");
        assert!(content_opf.contains("xml:lang=\"en-US\""));
        assert!(content_opf.contains("<dc:language>en-US</dc:language>"));

        let mut nav_xhtml = String::new();
        archive
            .by_name("OEBPS/nav.xhtml")
            .expect("nav.xhtml exists")
            .read_to_string(&mut nav_xhtml)
            .expect("read nav.xhtml");
        assert!(nav_xhtml.contains("xml:lang=\"en-US\""));

        let mut chapter_xhtml = String::new();
        archive
            .by_name("OEBPS/chapter-1.xhtml")
            .expect("chapter xhtml exists")
            .read_to_string(&mut chapter_xhtml)
            .expect("read chapter xhtml");
        assert!(chapter_xhtml.contains("xml:lang=\"en-US\""));

        remove_temp_workspace(&workspace);
    }
}
