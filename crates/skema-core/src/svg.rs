/// Generated SVG bytes (wire content-type: image/svg+xml).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgOutput {
    pub bytes: Vec<u8>,
}

impl SvgOutput {
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes).expect("SVG is UTF-8")
    }
}
