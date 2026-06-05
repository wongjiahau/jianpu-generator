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
    let span = (filename.clone(), e.span.start..e.span.end);

    Report::build(ReportKind::Error, span.clone())
        .with_message(&e.message)
        .with_label(Label::new(span).with_message(&e.message))
        .finish()
        .write((filename.clone(), Source::from(source.as_str())), writer)
        .unwrap();
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
