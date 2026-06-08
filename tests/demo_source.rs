use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_web_default_source() -> String {
    let path = repo_root().join("web/src/defaultSource.ts");
    let ts = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

    let marker = "export const DEFAULT_SOURCE = `";
    let start = ts
        .find(marker)
        .unwrap_or_else(|| panic!("{} is missing DEFAULT_SOURCE export", path.display()))
        + marker.len();
    let rest = &ts[start..];
    let end = rest.find('`').unwrap_or_else(|| {
        panic!(
            "{} has an unterminated DEFAULT_SOURCE template",
            path.display()
        )
    });

    rest[..end].to_owned()
}

#[test]
fn demo_jianpu_parses_and_renders() {
    let source = include_str!("../demo.jianpu");
    let svgs =
        jianpu_generator::render_svgs_from_source(source, "demo.jianpu").unwrap_or_else(|e| {
            panic!("demo.jianpu failed to parse/render: {e}");
        });
    assert!(
        !svgs.is_empty(),
        "demo.jianpu should produce at least one SVG page"
    );
    assert!(
        svgs.iter()
            .all(|svg| svg.starts_with("<svg") && svg.ends_with("</svg>")),
        "demo.jianpu SVG output should be well-formed"
    );
}

#[test]
fn web_default_source_matches_demo_jianpu() {
    let demo = include_str!("../demo.jianpu");
    let web_default = read_web_default_source();
    assert_eq!(
        web_default, demo,
        "web/src/defaultSource.ts DEFAULT_SOURCE must match demo.jianpu"
    );
}
