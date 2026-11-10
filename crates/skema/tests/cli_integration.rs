use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use skema::cli::{run_open_editor, run_render, run_validate, EditorJsonHandoff};
use skema_core::{parse_manifest, validate_manifest};

fn golden_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../skema-core/tests/fixtures/golden-manifest.json")
}

fn workspace_temp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "skema-cli-{name}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn skema_bin() -> PathBuf {
    env!("CARGO_BIN_EXE_skema").into()
}

#[test]
fn cli_validate_succeeds_on_golden() {
    let status = Command::new(skema_bin())
        .args(["validate", "--manifest", golden_manifest().to_str().unwrap()])
        .status()
        .expect("spawn skema");
    assert!(status.success());
}

#[test]
fn cli_validate_fails_on_malformed_json() {
    let dir = workspace_temp("validate-fail");
    fs::create_dir_all(&dir).unwrap();
    let manifest = dir.join("bad.json");
    fs::write(&manifest, "{ not json").unwrap();
    let output = Command::new(skema_bin())
        .args(["validate", "--manifest", manifest.to_str().unwrap()])
        .output()
        .expect("spawn skema");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("malformed JSON"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cli_render_writes_svg() {
    let dir = workspace_temp("render-ok");
    fs::create_dir_all(&dir).unwrap();
    let manifest = dir.join("manifest.json");
    fs::copy(golden_manifest(), &manifest).unwrap();
    let svg = dir.join("diagram.svg");
    let status = Command::new(skema_bin())
        .args([
            "render",
            "--manifest",
            manifest.to_str().unwrap(),
            "--svg",
            svg.to_str().unwrap(),
        ])
        .status()
        .expect("spawn skema");
    assert!(status.success());
    let body = fs::read_to_string(&svg).unwrap();
    assert!(body.starts_with("<svg "));
    assert!(body.contains("Start process"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cli_render_png_writes_a_png_beside_the_svg() {
    let dir = workspace_temp("render-png");
    fs::create_dir_all(&dir).unwrap();
    let manifest = dir.join("manifest.json");
    fs::copy(golden_manifest(), &manifest).unwrap();
    let svg = dir.join("diagram.svg");
    let png = dir.join("diagram.png");
    let status = Command::new(skema_bin())
        .args([
            "render",
            "--manifest",
            manifest.to_str().unwrap(),
            "--svg",
            svg.to_str().unwrap(),
            "--png",
            png.to_str().unwrap(),
        ])
        .status()
        .expect("spawn skema");
    assert!(status.success());
    let bytes = fs::read(&png).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cli_render_fails_validation_without_changing_svg() {
    let dir = workspace_temp("render-fail-preserve");
    fs::create_dir_all(&dir).unwrap();
    let manifest = dir.join("manifest.json");
    fs::copy(golden_manifest(), &manifest).unwrap();
    let svg = dir.join("diagram.svg");
    assert!(Command::new(skema_bin())
        .args([
            "render",
            "--manifest",
            manifest.to_str().unwrap(),
            "--svg",
            svg.to_str().unwrap(),
        ])
        .status()
        .unwrap()
        .success());
    let before = fs::read(&svg).unwrap();

    fs::write(
        &manifest,
        r#"{"nodes":[{"id":"a","label":"A","type":"process","size":"small","gridPosition":{"x":0,"y":0}}],"edges":[{"from":"a","to":"missing"}]}"#,
    )
    .unwrap();
    let output = Command::new(skema_bin())
        .args([
            "render",
            "--manifest",
            manifest.to_str().unwrap(),
            "--svg",
            svg.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(fs::read(&svg).unwrap(), before);
    let _ = fs::remove_dir_all(&dir);
}

/// AC-agent-render: Agent edits Manifest text and renders SVG via CLI without Editor.
#[test]
fn ac_agent_render() {
    let dir = workspace_temp("agent-render");
    fs::create_dir_all(&dir).unwrap();
    let manifest = dir.join("manifest.json");
    fs::copy(golden_manifest(), &manifest).unwrap();
    let svg = dir.join("diagram.svg");
    run_render(&manifest, &svg).expect("initial render");
    let mut json = fs::read_to_string(&manifest).unwrap();
    json = json.replace("Start process", "Agent updated label");
    fs::write(&manifest, &json).unwrap();
    run_render(&manifest, &svg).expect("render after edit");
    let svg_body = fs::read_to_string(&svg).unwrap();
    assert!(svg_body.contains("Agent updated label"));
    assert!(!svg_body.contains("Start process"));
    let _ = fs::remove_dir_all(&dir);
}

/// AC-reviewable-change: single-field label diff visible in SVG via CLI only.
#[test]
fn ac_reviewable_change() {
    let dir = workspace_temp("reviewable");
    fs::create_dir_all(&dir).unwrap();
    let manifest = dir.join("manifest.json");
    fs::write(
        &manifest,
        r#"{"nodes":[{"id":"n1","label":"Before","type":"process","size":"small","gridPosition":{"x":0,"y":0}}],"edges":[]}"#,
    )
    .unwrap();
    let svg = dir.join("out.svg");
    run_render(&manifest, &svg).unwrap();
    assert!(fs::read_to_string(&svg).unwrap().contains("Before"));
    fs::write(
        &manifest,
        r#"{"nodes":[{"id":"n1","label":"After","type":"process","size":"small","gridPosition":{"x":0,"y":0}}],"edges":[]}"#,
    )
    .unwrap();
    run_render(&manifest, &svg).unwrap();
    assert!(fs::read_to_string(&svg).unwrap().contains("After"));
    let _ = fs::remove_dir_all(&dir);
}

/// AC-empty-start: handoff carries valid empty Manifest when no file exists.
#[test]
fn ac_empty_start_handoff() {
    let dir = workspace_temp("empty-start");
    fs::create_dir_all(&dir).unwrap();
    let manifest = dir.join("manifest.json");
    let svg = dir.join("diagram.svg");
    let handoff = dir.join("handoff.json");
    run_open_editor(&manifest, &svg, &handoff).expect("open-editor empty start");
    assert!(!manifest.exists());
    let payload: EditorJsonHandoff =
        serde_json::from_str(&fs::read_to_string(&handoff).unwrap()).expect("handoff JSON");
    assert_eq!(payload.manifest_path, manifest.to_str().unwrap());
    assert_eq!(payload.svg_path, svg.to_str().unwrap());
    let doc = parse_manifest(&payload.manifest_json).expect("parse handoff manifest");
    assert!(doc.nodes.is_empty() && doc.edges.is_empty());
    validate_manifest(&doc).expect("empty manifest validates");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn handoff_matches_existing_manifest_file() {
    let dir = workspace_temp("handoff-existing");
    fs::create_dir_all(&dir).unwrap();
    let manifest = dir.join("manifest.json");
    let on_disk = fs::read_to_string(golden_manifest()).unwrap();
    fs::write(&manifest, &on_disk).unwrap();
    let svg = dir.join("diagram.svg");
    let handoff = dir.join("handoff.json");
    run_open_editor(&manifest, &svg, &handoff).unwrap();
    let payload: EditorJsonHandoff =
        serde_json::from_str(&fs::read_to_string(&handoff).unwrap()).unwrap();
    assert_eq!(payload.manifest_json, on_disk);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn run_validate_rejects_missing_manifest() {
    let path = workspace_temp("missing").join("nope.json");
    assert!(run_validate(&path).is_err());
}

#[test]
fn version_flag_prints_the_package_version() {
    let output = Command::new(skema_bin()).arg("--version").output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), format!("skema {}", env!("CARGO_PKG_VERSION")));
}

/// One release version everywhere: the binary, the version the skill asks for,
/// and the four plugin manifests. Bump them together.
#[test]
fn skill_and_plugin_manifests_name_the_binary_version() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let version = env!("CARGO_PKG_VERSION");
    let skill = fs::read_to_string(root.join("skills/skema-diagram/SKILL.md")).unwrap();
    assert!(skill.contains(&format!("needs `skema` {version} or newer")), "SKILL.md must ask for skema {version}");
    for manifest in [".claude-plugin/plugin.json", ".claude-plugin/marketplace.json", ".cursor-plugin/plugin.json"] {
        let text = fs::read_to_string(root.join(manifest)).unwrap();
        assert!(text.contains(&format!("\"version\": \"{version}\"")), "{manifest} must be {version}");
    }
    let herdr = fs::read_to_string(root.join("herdr-plugin/herdr-plugin.toml")).unwrap();
    assert!(herdr.contains(&format!("version = \"{version}\"")), "herdr-plugin.toml must be {version}");
}

/// With nothing chosen, renders use Match terminal: the cached terminal palette,
/// or Phosphor when there is none yet. The PNG comes from the same style.
#[test]
fn render_defaults_to_the_terminal_palette_and_falls_back_to_phosphor() {
    let dir = workspace_temp("default-theme");
    let config = dir.join("config");
    fs::create_dir_all(&config).unwrap();
    let manifest = dir.join("manifest.json");
    fs::copy(golden_manifest(), &manifest).unwrap();
    let svg = dir.join("diagram.svg");
    let render = || {
        let status = Command::new(skema_bin())
            .env("SKEMA_CONFIG_DIR", &config)
            .args(["render", "--manifest", manifest.to_str().unwrap(), "--svg", svg.to_str().unwrap()])
            .args(["--png", dir.join("diagram.png").to_str().unwrap()])
            .status()
            .expect("spawn skema");
        assert!(status.success());
        fs::read_to_string(&svg).unwrap()
    };
    let phosphor = skema_core::style::diagram_style().color.canvas.clone();
    assert!(render().contains(&format!("fill=\"{phosphor}\"")), "no palette cached: Phosphor");

    let mut palette = skema_core::theme::nord_colors();
    palette.canvas = "#123456".into();
    fs::write(config.join("terminal-palette.json"), serde_json::to_string(&palette).unwrap()).unwrap();
    let svg_text = render();
    assert!(svg_text.contains("fill=\"#123456\""), "the cached terminal palette");
    assert!(!svg_text.contains(&format!("fill=\"{phosphor}\"")));
    let _ = fs::remove_dir_all(&dir);
}

/// A Manifest with no Label warnings.
fn clean_manifest(dir: &Path) -> PathBuf {
    let manifest = dir.join("clean.json");
    fs::write(
        &manifest,
        r#"{"nodes":[
        {"id":"a","label":"Client","type":"process","size":"medium","gridPosition":{"x":1,"y":1}},
        {"id":"b","label":"Server","type":"process","size":"medium","gridPosition":{"x":6,"y":1}}
        ],"edges":[{"from":"a","to":"b","label":"call","pulse":"both"}]}"#,
    )
    .unwrap();
    manifest
}

fn aws_front_end_client(dir: &Path) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let json = fs::read_to_string(root.join("examples/aws-microservices.json"))
        .unwrap()
        .replace("\"Web client\"", "\"Front-end client\"");
    let manifest = dir.join("aws.json");
    fs::write(&manifest, json).unwrap();
    manifest
}

fn render_cmd(manifest: &Path, svg: &Path, extra: &[&str]) -> std::process::Output {
    Command::new(skema_bin())
        .args(["render", "--theme", "phosphor", "--manifest", manifest.to_str().unwrap(), "--svg", svg.to_str().unwrap()])
        .args(extra)
        .output()
        .expect("spawn skema")
}

/// AC-warn-truncate: warnings never block output; --strict fails and leaves existing files alone.
#[test]
fn ac_warn_truncate_cli_and_strict() {
    let dir = workspace_temp("warn-truncate");
    fs::create_dir_all(&dir).unwrap();
    let manifest = aws_front_end_client(&dir);
    let svg = dir.join("aws.svg");
    let out = render_cmd(&manifest, &svg, &[]);
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let truncated: Vec<&str> = stderr.lines().filter(|l| l.starts_with("warning[label-truncated] node \"client\"")).collect();
    assert_eq!(
        truncated,
        [r#"warning[label-truncated] node "client": "Front-end client" → "Front-end cli…" (fits 13 chars at 2×1)"#]
    );
    let before = fs::read(&svg).unwrap();

    // Make the next render differ, so an unwanted write would show.
    let json = fs::read_to_string(&manifest).unwrap().replace("\"Shared DB\"", "\"Shared store\"");
    fs::write(&manifest, json).unwrap();
    let png = dir.join("aws.png");
    let gif = dir.join("aws.gif");
    let out = render_cmd(&manifest, &svg, &["--strict", "--png", png.to_str().unwrap(), "--gif", gif.to_str().unwrap()]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("warning[label-truncated] node \"client\""), "{stderr}");
    assert!(stderr.contains("--strict"), "{stderr}");
    assert_eq!(fs::read(&svg).unwrap(), before, "existing SVG keeps its bytes");
    assert!(!png.exists() && !gif.exists(), "no outputs written");

    // validate prints the same warnings.
    let out = Command::new(skema_bin()).args(["validate", "--manifest", manifest.to_str().unwrap()]).output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("node \"client\""));
    let _ = fs::remove_dir_all(&dir);
}

/// AC-warn-json: one parseable JSON array, one object per warning.
#[test]
fn ac_warn_json() {
    let dir = workspace_temp("warn-json");
    fs::create_dir_all(&dir).unwrap();
    let manifest = aws_front_end_client(&dir);
    let svg = dir.join("aws.svg");
    let text = render_cmd(&manifest, &svg, &[]);
    let lines = String::from_utf8_lossy(&text.stderr).lines().filter(|l| l.starts_with("warning[")).count();
    let out = render_cmd(&manifest, &svg, &["--warnings=json"]);
    assert!(out.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&out.stderr).expect("stderr is one JSON array");
    let items = parsed.as_array().unwrap();
    assert_eq!(items.len(), lines);
    let client = items.iter().find(|w| w["id"] == "client").unwrap();
    assert_eq!(client["code"], "label-truncated");
    assert_eq!(client["kind"], "node");
    assert_eq!(client["text"], "Front-end client");
    assert_eq!(client["drawn"], "Front-end cli…");
    assert_eq!(client["fits"], 13);

    // Nothing to report: an empty array.
    let out = Command::new(skema_bin())
        .args(["validate", "--warnings", "json", "--manifest", clean_manifest(&dir).to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stderr).trim(), "[]");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn render_gif_writes_a_looping_gif() {
    let dir = workspace_temp("render-gif");
    fs::create_dir_all(&dir).unwrap();
    let manifest = clean_manifest(&dir);
    let svg = dir.join("m.svg");
    let gif = dir.join("m.gif");
    let out = render_cmd(&manifest, &svg, &["--gif", gif.to_str().unwrap()]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(out.stderr.is_empty(), "no warnings: {}", String::from_utf8_lossy(&out.stderr));
    let bytes = fs::read(&gif).unwrap();
    assert_eq!(&bytes[..6], b"GIF89a");
    let _ = fs::remove_dir_all(&dir);
}
