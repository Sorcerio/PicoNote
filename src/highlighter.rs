use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontFamily, FontId, Stroke};

use crate::parser::{self, MdStyle};

/// Memoized markdown highlighter. Caches the last input/output to avoid
/// re-parsing identical text every frame in egui's immediate-mode loop.
#[derive(Default)]
pub struct MemoizedMarkdownHighlighter {
    prev_text: String,
    prev_job: LayoutJob,
    prev_font_size: f32,
    prev_dark: bool,
}

impl MemoizedMarkdownHighlighter {
    pub fn highlight(&mut self, style: &egui::Style, text: &str, font_size: f32) -> LayoutJob {
        let is_dark = style.visuals.dark_mode;
        if self.prev_text == text
            && (self.prev_font_size - font_size).abs() < f32::EPSILON
            && self.prev_dark == is_dark
        {
            return self.prev_job.clone();
        }

        text.clone_into(&mut self.prev_text);
        self.prev_font_size = font_size;
        self.prev_dark = is_dark;
        self.prev_job = build_layout_job(style, text, font_size);
        self.prev_job.clone()
    }
}

fn build_layout_job(style: &egui::Style, text: &str, font_size: f32) -> LayoutJob {
    let spans = parser::parse_markdown(text);
    let colors = Colors::for_visuals(&style.visuals);
    let mut job = LayoutJob::default();

    for span in &spans {
        let format = md_style_to_format(&span.style, &colors, font_size);
        job.append(&span.text, 0.0, format);
    }

    job
}

struct Colors {
    text: Color32,
    strong: Color32,
    weak: Color32,
    marker: Color32,
    heading: Color32,
    link: Color32,
    link_url: Color32,
    code_fg: Color32,
    code_bg: Color32,
    highlight_bg: Color32,
    strike: Color32,
    task: Color32,
    task_checked: Color32,
    table_pipe: Color32,
    footnote: Color32,
    emoji: Color32,
    sub_super: Color32,
}

impl Colors {
    fn for_visuals(v: &egui::Visuals) -> Self {
        if v.dark_mode {
            Self {
                text: v.text_color(),
                strong: Color32::from_rgb(240, 240, 250),
                weak: v.weak_text_color(),
                marker: Color32::from_rgb(100, 100, 120),
                heading: Color32::from_rgb(100, 180, 255),
                link: Color32::from_rgb(80, 160, 240),
                link_url: Color32::from_rgb(90, 130, 170),
                code_fg: Color32::from_rgb(200, 180, 160),
                code_bg: Color32::from_rgb(45, 42, 40),
                highlight_bg: Color32::from_rgb(80, 70, 20),
                strike: Color32::from_rgb(140, 140, 150),
                task: Color32::from_rgb(100, 180, 255),
                task_checked: Color32::from_rgb(80, 180, 100),
                table_pipe: Color32::from_rgb(100, 160, 220),
                footnote: Color32::from_rgb(150, 130, 200),
                emoji: Color32::from_rgb(220, 180, 100),
                sub_super: Color32::from_rgb(130, 200, 180),
            }
        } else {
            Self {
                text: v.text_color(),
                strong: Color32::from_rgb(20, 20, 30),
                weak: v.weak_text_color(),
                marker: Color32::from_rgb(150, 150, 170),
                heading: Color32::from_rgb(0, 90, 180),
                link: Color32::from_rgb(0, 100, 200),
                link_url: Color32::from_rgb(80, 120, 160),
                code_fg: Color32::from_rgb(120, 80, 40),
                code_bg: Color32::from_rgb(240, 235, 228),
                highlight_bg: Color32::from_rgb(255, 245, 140),
                strike: Color32::from_rgb(120, 120, 130),
                task: Color32::from_rgb(0, 90, 180),
                task_checked: Color32::from_rgb(30, 140, 60),
                table_pipe: Color32::from_rgb(0, 100, 180),
                footnote: Color32::from_rgb(100, 70, 160),
                emoji: Color32::from_rgb(180, 130, 30),
                sub_super: Color32::from_rgb(0, 130, 100),
            }
        }
    }
}

fn md_style_to_format(md: &MdStyle, c: &Colors, base_size: f32) -> TextFormat {
    // --- Font ---
    let family = if md.code || md.code_block {
        FontFamily::Monospace
    } else {
        FontFamily::Proportional
    };

    let size = if md.heading_level > 0 {
        match md.heading_level {
            1 => base_size * 1.6,
            2 => base_size * 1.4,
            3 => base_size * 1.2,
            4 => base_size * 1.1,
            _ => base_size * 1.05,
        }
    } else {
        base_size
    };

    // --- Color ---
    let color = if md.syntax_marker {
        // Syntax markers are always dimmed, but tinted by context
        if md.task_checkbox {
            if md.task_checked {
                c.task_checked
            } else {
                c.task
            }
        } else {
            c.marker
        }
    } else if md.heading_level > 0 {
        c.heading
    } else if md.bold {
        c.strong
    } else if md.link_text {
        c.link
    } else if md.link_url {
        c.link_url
    } else if md.code || md.code_block {
        c.code_fg
    } else if md.blockquote {
        c.weak
    } else if md.strikethrough {
        c.strike
    } else if md.highlight {
        c.text
    } else if md.footnote_ref || md.footnote_def {
        c.footnote
    } else if md.table_pipe {
        c.table_pipe
    } else if md.table_align {
        c.marker
    } else if md.emoji_shortcode {
        c.emoji
    } else if md.subscript || md.superscript {
        c.sub_super
    } else {
        c.text
    };

    // --- Background ---
    let background = if md.code || md.code_block {
        c.code_bg
    } else if md.highlight && !md.syntax_marker {
        c.highlight_bg
    } else {
        Color32::TRANSPARENT
    };

    // --- Decorations ---
    let italics = md.italic;

    let underline = if md.link_text {
        Stroke::new(1.0, c.link)
    } else if md.link_url && !md.syntax_marker {
        Stroke::new(1.0, c.link_url)
    } else {
        Stroke::NONE
    };

    let strikethrough = if md.strikethrough {
        Stroke::new(1.0, c.strike)
    } else {
        Stroke::NONE
    };

    TextFormat {
        font_id: FontId::new(size, family),
        color,
        background,
        italics,
        underline,
        strikethrough,
        ..Default::default()
    }
}
