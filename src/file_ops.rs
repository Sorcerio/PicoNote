use std::path::PathBuf;

pub struct FileState {
    pub path: Option<PathBuf>,
    pub dirty: bool,
}

impl FileState {
    pub fn new() -> Self {
        Self {
            path: None,
            dirty: false,
        }
    }
}

pub fn open_file_dialog() -> Option<(String, PathBuf)> {
    let path = rfd::FileDialog::new()
        .add_filter("Markdown", &["md", "markdown", "txt"])
        .add_filter("All Files", &["*"])
        .pick_file()?;
    let content = std::fs::read_to_string(&path).ok()?;
    Some((content, path))
}

pub fn save_file_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Markdown", &["md", "markdown", "txt"])
        .set_file_name("untitled.md")
        .save_file()
}

pub fn write_file(path: &PathBuf, content: &str) -> std::io::Result<()> {
    std::fs::write(path, content)
}
