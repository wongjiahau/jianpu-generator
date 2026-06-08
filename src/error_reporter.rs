use crate::error::JianPuError;

pub fn render(e: &JianPuError) {
    render_to_writer(e, std::io::stderr());
}

fn render_to_writer(e: &JianPuError, mut writer: impl std::io::Write) {
    use ariadne::{Label, Report, ReportKind, Source};

    let Some(path) = &e.path else {
        writeln!(writer, "error: {}", e.message).ok();
        return;
    };

    let Ok(source) = std::fs::read_to_string(path) else {
        writeln!(writer, "error: {}", e.message).ok();
        return;
    };

    let filename = path.to_string_lossy().into_owned();
    // ariadne indexes by Unicode character count, not by byte offset.
    let char_start = source[..e.span.start.min(source.len())].chars().count();
    let char_end = source[..e.span.end.min(source.len())].chars().count();
    let span = (filename.clone(), char_start..char_end);

    if Report::build(ReportKind::Error, span.clone())
        .with_message(&e.message)
        .with_label(Label::new(span).with_message(&e.message))
        .finish()
        .write((filename, Source::from(source.as_str())), &mut writer)
        .is_err()
    {
        writeln!(writer, "error: {}", e.message).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Span;
    use std::path::PathBuf;

    fn write_temp_file(name: &str, content: &str) -> PathBuf {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn render_output_contains_message() {
        let path = write_temp_file("test_render.jianpu", "1 2 x 4\n");
        let e = JianPuError::new(Span::new(4, 5), "expected pitch digit 0-7").with_path(&path);

        let mut buf = Vec::new();
        render_to_writer(&e, &mut buf);
        let output = String::from_utf8_lossy(&buf);
        assert!(
            output.contains("expected pitch digit 0-7"),
            "output was: {output}"
        );
    }

    #[test]
    fn render_shows_code_block_when_source_contains_multibyte_unicode() {
        // Each Chinese character is 3 bytes. The error token "x" is at byte offset 12
        // (3 bytes × 4 chars = 12), but at character offset 4.
        // Without the byte→char conversion ariadne would look past end-of-source
        // and silently omit the code block.
        let source = "你好世界 x 4\n";
        let path = write_temp_file("test_unicode_render.jianpu", source);
        let token_byte_start = "你好世界 ".len(); // 3*4 + 1 = 13
        let e = JianPuError::new(
            Span::new(token_byte_start, token_byte_start + 1),
            "bad token",
        )
        .with_path(&path);

        let mut buf = Vec::new();
        render_to_writer(&e, &mut buf);
        let output = String::from_utf8_lossy(&buf);
        // The code block must appear — presence of '│' confirms ariadne rendered it.
        assert!(
            output.contains('│'),
            "expected ariadne code block (│) in output, got: {output}"
        );
        assert!(output.contains("bad token"), "output was: {output}");
    }

    #[test]
    fn render_falls_back_when_path_is_none() {
        let e = JianPuError::new(Span::new(0, 1), "some error");
        let mut buf = Vec::new();
        render_to_writer(&e, &mut buf);
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("some error"), "output was: {output}");
    }

    #[test]
    fn render_falls_back_when_file_unreadable() {
        let e =
            JianPuError::new(Span::new(0, 1), "some error").with_path("/nonexistent/path.jianpu");
        let mut buf = Vec::new();
        render_to_writer(&e, &mut buf);
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("some error"), "output was: {output}");
    }
}
