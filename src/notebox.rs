use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_editor};
use iced::{Color, Element, Font, Length, Theme};
use std::collections::HashSet;

const SYMBOL_HEADING_ASTERISK: char = '*';
const SYMBOL_TABLE_PIPE: char = '|';
const SYMBOL_TABLE_PLUS: char = '+';
const SYMBOL_TABLE_DASH: char = '-';
const SYMBOL_SPACE: char = ' ';
const SYMBOL_NEWLINE: char = '\n';

const PREFIX_UNCHECKED_BOX: &str = "- [ ]";
const PREFIX_CHECKED_LOWER_BOX: &str = "- [x]";
const PREFIX_CHECKED_UPPER_BOX: &str = "- [X]";

const FONT_FAMILY_NAME: &str = "Material Symbols Outlined";
const MATERIAL_ICON_COLLAPSED: &str = "\u{e5cc}";
const MATERIAL_ICON_EXPANDED: &str = "\u{e5cf}";

const DEFAULT_INDENTATION_SPACES_PER_LEVEL: usize = 2;
const MINIMUM_HEADER_LEVEL: usize = 1;

const COLOR_PRIMARY_GOLD_R: f32 = 0.83;
const COLOR_PRIMARY_GOLD_G: f32 = 0.68;
const COLOR_PRIMARY_GOLD_B: f32 = 0.21;

const COLOR_TEXT_NORMAL_R: f32 = 0.80;
const COLOR_TEXT_NORMAL_G: f32 = 0.80;
const COLOR_TEXT_NORMAL_B: f32 = 0.83;

const COLOR_TEXT_CHECKED_R: f32 = 0.48;
const COLOR_TEXT_CHECKED_G: f32 = 0.66;
const COLOR_TEXT_CHECKED_B: f32 = 0.48;

const COLOR_CONTAINER_BG_R: f32 = 0.09;
const COLOR_CONTAINER_BG_G: f32 = 0.09;
const COLOR_CONTAINER_BG_B: f32 = 0.12;

const COLOR_BORDER_R: f32 = 0.16;
const COLOR_BORDER_G: f32 = 0.16;
const COLOR_BORDER_B: f32 = 0.20;

const UI_CONTAINER_HEIGHT: f32 = 360.0;
const UI_SCROLLBAR_WIDTH: f32 = 1.0;
const UI_BORDER_WIDTH: f32 = 1.0;
const UI_BORDER_RADIUS: f32 = 4.0;
const UI_ROW_SPACING: f32 = 4.0;
const UI_COLUMN_SPACING: f32 = 6.0;
const UI_PADDING_AMOUNT: f32 = 8.0;

const FONT_SIZE_HEADER_LEVEL_1: u16 = 16;
const FONT_SIZE_HEADER_LEVEL_2: u16 = 14;
const FONT_SIZE_HEADER_LEVEL_3: u16 = 13;
const FONT_SIZE_BODY_TEXT: u16 = 12;
const FONT_SIZE_FOLDING_ICON: u16 = 12;

const FONT_MATERIAL_SYMBOLS: Font = Font {
    family: iced::font::Family::Name(FONT_FAMILY_NAME),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

#[derive(Debug, Clone)]
pub enum NoteBoxMessage {
    EditorActionPerformed(text_editor::Action),
    HeaderFoldToggled(usize),
    TaskStateToggled(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrgLineType<'a> {
    Header { level: usize, text: &'a str },
    Task { is_checked: bool, label: &'a str },
    TableBorder,
    TableRow { cells: Vec<&'a str> },
    StandardText { text: &'a str },
}

#[derive(Debug, Clone)]
pub struct OrgParsedLine<'a> {
    pub line_index: usize,
    pub indentation_level: usize,
    pub content_type: OrgLineType<'a>,
}

#[derive(Debug, Clone, Default)]
pub struct NoteBoxState {
    pub collapsed_line_indices: HashSet<usize>,
}

pub struct NoteBoxView;

impl NoteBoxView {
    pub fn render_editor<'a>(
        editor_content: &'a text_editor::Content,
    ) -> Element<'a, NoteBoxMessage> {
        let editor_widget = text_editor(editor_content)
            .on_action(NoteBoxMessage::EditorActionPerformed)
            .padding(UI_PADDING_AMOUNT)
            .height(Length::Shrink);

        let vertical_scrollbar = scrollable::Scrollbar::new()
            .width(UI_SCROLLBAR_WIDTH)
            .margin(0.0)
            .scroller_width(UI_SCROLLBAR_WIDTH);

        let scrollable_container = scrollable(editor_widget)
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(scrollable::Direction::Vertical(vertical_scrollbar))
            .style(custom_scrollbar_style);

        container(scrollable_container)
            .height(UI_CONTAINER_HEIGHT)
            .width(Length::Fill)
            .style(build_container_style)
            .into()
    }

    pub fn render_viewer<'a>(
        raw_text: &str,
        state: &NoteBoxState,
    ) -> Element<'a, NoteBoxMessage> {
        let mut main_column = column![].spacing(UI_COLUMN_SPACING);

        let parsed_lines = OrgParser::parse_document(raw_text);
        let visible_lines = OrgVisibilityEngine::filter_visible_lines(&parsed_lines, &state.collapsed_line_indices);

        for line in visible_lines {
            let is_collapsed = state.collapsed_line_indices.contains(&line.line_index);
            let rendered_row = render_single_org_line(line, is_collapsed);
            main_column = main_column.push(rendered_row);
        }

        let vertical_scrollbar = scrollable::Scrollbar::new()
            .width(UI_SCROLLBAR_WIDTH)
            .margin(0.0)
            .scroller_width(UI_SCROLLBAR_WIDTH);

        let scrollable_content = scrollable(main_column.padding(UI_PADDING_AMOUNT))
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(scrollable::Direction::Vertical(vertical_scrollbar))
            .style(custom_scrollbar_style);

        container(scrollable_content)
            .height(UI_CONTAINER_HEIGHT)
            .width(Length::Fill)
            .style(build_container_style)
            .into()
    }
}

pub struct OrgParser;

impl OrgParser {
    pub fn parse_document<'a>(raw_text: &'a str) -> Vec<OrgParsedLine<'a>> {
        let mut parsed_lines = Vec::new();

        for (index, line_str) in raw_text.lines().enumerate() {
            let indentation_level = calculate_indentation_level(line_str);
            let trimmed_content = line_str.trim_start();
            let content_type = parse_line_content_type(trimmed_content);

            parsed_lines.push(OrgParsedLine {
                line_index: index,
                indentation_level,
                content_type,
            });
        }

        parsed_lines
    }
}

pub struct OrgFormatter;

impl OrgFormatter {
    pub fn format_entire_document(raw_text: &str) -> String {
        let lines: Vec<&str> = raw_text.lines().collect();
        let mut formatted_lines = Vec::new();

        let mut index = 0;
        while index < lines.len() {
            if is_table_line(lines[index]) {
                let table_end = find_table_end_index(&lines, index);
                let formatted_table_block = format_table_block(&lines[index..table_end]);
                formatted_lines.extend(formatted_table_block);
                index = table_end;
            } else {
                formatted_lines.push(format_standard_line(lines[index]));
                index += 1;
            }
        }

        let mut result = formatted_lines.join(&SYMBOL_NEWLINE.to_string());
        if raw_text.ends_with(SYMBOL_NEWLINE) {
            result.push(SYMBOL_NEWLINE);
        }
        result
    }

    pub fn toggle_task_state_at_line(raw_text: &str, target_line_index: usize) -> String {
        let lines: Vec<&str> = raw_text.lines().collect();
        if target_line_index >= lines.len() {
            return raw_text.to_string();
        }

        let mut updated_lines = Vec::with_capacity(lines.len());
        for (index, line) in lines.iter().enumerate() {
            if index == target_line_index {
                updated_lines.push(toggle_task_prefix_in_string(line));
            } else {
                updated_lines.push(line.to_string());
            }
        }

        let mut result = updated_lines.join(&SYMBOL_NEWLINE.to_string());
        if raw_text.ends_with(SYMBOL_NEWLINE) {
            result.push(SYMBOL_NEWLINE);
        }
        result
    }
}

pub struct OrgVisibilityEngine;

impl OrgVisibilityEngine {
    pub fn filter_visible_lines<'a>(
        parsed_lines: &'a [OrgParsedLine<'a>],
        collapsed_indices: &HashSet<usize>,
    ) -> Vec<&'a OrgParsedLine<'a>> {
        let mut visible_lines = Vec::new();
        let mut active_hiding_level: Option<usize> = None;

        for line in parsed_lines {
            if let Some(hidden_level) = active_hiding_level {
                if line.indentation_level > hidden_level {
                    continue;
                } else {
                    active_hiding_level = None;
                }
            }

            visible_lines.push(line);

            if collapsed_indices.contains(&line.line_index) {
                if let OrgLineType::Header { .. } = line.content_type {
                    active_hiding_level = Some(line.indentation_level);
                }
            }
        }

        visible_lines
    }
}

fn calculate_indentation_level(line_str: &str) -> usize {
    let leading_spaces = line_str.chars().take_while(|&c| c == SYMBOL_SPACE).count();
    let trimmed = line_str.trim_start();

    if trimmed.starts_with(SYMBOL_HEADING_ASTERISK) {
        let asterisk_count = trimmed.chars().take_while(|&c| c == SYMBOL_HEADING_ASTERISK).count();
        if asterisk_count > 0 {
            return asterisk_count.saturating_sub(MINIMUM_HEADER_LEVEL);
        }
    }

    leading_spaces / DEFAULT_INDENTATION_SPACES_PER_LEVEL
}

fn parse_line_content_type<'a>(trimmed_content: &'a str) -> OrgLineType<'a> {
    if trimmed_content.starts_with(SYMBOL_HEADING_ASTERISK) {
        let level = trimmed_content.chars().take_while(|&c| c == SYMBOL_HEADING_ASTERISK).count();
        let text = trimmed_content[level..].trim();
        return OrgLineType::Header { level, text };
    }

    if trimmed_content.starts_with(PREFIX_UNCHECKED_BOX) {
        let label = trimmed_content[PREFIX_UNCHECKED_BOX.len()..].trim();
        return OrgLineType::Task { is_checked: false, label };
    }

    if trimmed_content.starts_with(PREFIX_CHECKED_LOWER_BOX) {
        let label = trimmed_content[PREFIX_CHECKED_LOWER_BOX.len()..].trim();
        return OrgLineType::Task { is_checked: true, label };
    }

    if trimmed_content.starts_with(PREFIX_CHECKED_UPPER_BOX) {
        let label = trimmed_content[PREFIX_CHECKED_UPPER_BOX.len()..].trim();
        return OrgLineType::Task { is_checked: true, label };
    }

    if trimmed_content.starts_with(SYMBOL_TABLE_PIPE) && trimmed_content.contains(SYMBOL_TABLE_PLUS) {
        return OrgLineType::TableBorder;
    }

    if trimmed_content.starts_with(SYMBOL_TABLE_PIPE) {
        let cells = trimmed_content
            .split(SYMBOL_TABLE_PIPE)
            .skip(1)
            .filter(|s| !s.is_empty())
            .map(|s| s.trim())
            .collect();
        return OrgLineType::TableRow { cells };
    }

    OrgLineType::StandardText { text: trimmed_content }
}

fn is_table_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with(SYMBOL_TABLE_PIPE)
}

fn find_table_end_index(lines: &[&str], start_index: usize) -> usize {
    let mut current_index = start_index;
    while current_index < lines.len() && is_table_line(lines[current_index]) {
        current_index += 1;
    }
    current_index
}

fn format_table_block(table_lines: &[&str]) -> Vec<String> {
    let mut parsed_rows: Vec<Vec<String>> = Vec::new();
    let mut column_max_widths: Vec<usize> = Vec::new();

    for line in table_lines {
        let trimmed = line.trim_start();
        if trimmed.contains(SYMBOL_TABLE_PLUS) || trimmed.contains(SYMBOL_TABLE_DASH) {
            continue;
        }

        let cells: Vec<String> = trimmed
            .split(SYMBOL_TABLE_PIPE)
            .skip(1)
            .map(|cell| cell.trim().to_string())
            .collect();

        if cells.is_empty() {
            continue;
        }

        for (i, cell) in cells.iter().enumerate() {
            if i >= column_max_widths.len() {
                column_max_widths.push(cell.chars().count());
            } else if cell.chars().count() > column_max_widths[i] {
                column_max_widths[i] = cell.chars().count();
            }
        }

        parsed_rows.push(cells);
    }

    let mut formatted_output = Vec::new();
    let border_string = build_table_border_line(&column_max_widths);

    formatted_output.push(border_string.clone());

    for row_cells in parsed_rows {
        let mut row_string = String::new();
        row_string.push(SYMBOL_TABLE_PIPE);

        for (i, width) in column_max_widths.iter().enumerate() {
            let cell_content = row_cells.get(i).map(|s| s.as_str()).unwrap_or("");
            let padding = width.saturating_sub(cell_content.chars().count());
            row_string.push(SYMBOL_SPACE);
            row_string.push_str(cell_content);
            row_string.push_str(&SYMBOL_SPACE.to_string().repeat(padding));
            row_string.push(SYMBOL_SPACE);
            row_string.push(SYMBOL_TABLE_PIPE);
        }

        formatted_output.push(row_string);
        formatted_output.push(border_string.clone());
    }

    formatted_output
}

fn build_table_border_line(column_widths: &[usize]) -> String {
    let mut border = String::new();
    border.push(SYMBOL_TABLE_PLUS);

    for width in column_widths {
        border.push_str(&SYMBOL_TABLE_DASH.to_string().repeat(width + 2));
        border.push(SYMBOL_TABLE_PLUS);
    }

    border
}

fn format_standard_line(line: &str) -> String {
    let indentation = line.chars().take_while(|&c| c == SYMBOL_SPACE).count();
    let trimmed = line.trim_start();

    if trimmed.starts_with(SYMBOL_HEADING_ASTERISK) {
        let asterisks = trimmed.chars().take_while(|&c| c == SYMBOL_HEADING_ASTERISK).count();
        let content = trimmed[asterisks..].trim();
        let required_indentation = asterisks.saturating_sub(MINIMUM_HEADER_LEVEL) * DEFAULT_INDENTATION_SPACES_PER_LEVEL;
        return format!("{}{}{} {}", SYMBOL_SPACE.to_string().repeat(required_indentation), SYMBOL_HEADING_ASTERISK.to_string().repeat(asterisks), "", content);
    }

    format!("{}{}", SYMBOL_SPACE.to_string().repeat(indentation), trimmed)
}

fn toggle_task_prefix_in_string(line: &str) -> String {
    if line.contains(PREFIX_UNCHECKED_BOX) {
        line.replacen(PREFIX_UNCHECKED_BOX, PREFIX_CHECKED_LOWER_BOX, 1)
    } else if line.contains(PREFIX_CHECKED_LOWER_BOX) {
        line.replacen(PREFIX_CHECKED_LOWER_BOX, PREFIX_UNCHECKED_BOX, 1)
    } else if line.contains(PREFIX_CHECKED_UPPER_BOX) {
        line.replacen(PREFIX_CHECKED_UPPER_BOX, PREFIX_UNCHECKED_BOX, 1)
    } else {
        line.to_string()
    }
}

fn render_single_org_line<'a>(
    parsed_line: &OrgParsedLine<'_>,
    is_collapsed: bool,
) -> Element<'a, NoteBoxMessage> {
    let indentation_padding = parsed_line.indentation_level * DEFAULT_INDENTATION_SPACES_PER_LEVEL * 8;

    let content_element: Element<'a, NoteBoxMessage> = match &parsed_line.content_type {
        OrgLineType::Header { level, text: header_text } => {
            let icon = if is_collapsed { MATERIAL_ICON_COLLAPSED } else { MATERIAL_ICON_EXPANDED };
            let font_size = match level {
                1 => FONT_SIZE_HEADER_LEVEL_1,
                2 => FONT_SIZE_HEADER_LEVEL_2,
                _ => FONT_SIZE_HEADER_LEVEL_3,
            };

            let fold_button = button(text(icon).font(FONT_MATERIAL_SYMBOLS).size(FONT_SIZE_FOLDING_ICON))
                .on_press(NoteBoxMessage::HeaderFoldToggled(parsed_line.line_index))
                .padding(0);

            let header_label = text(format!("{} {}", SYMBOL_HEADING_ASTERISK.to_string().repeat(*level), header_text))
                .size(font_size)
                .color(Color::from_rgb(COLOR_PRIMARY_GOLD_R, COLOR_PRIMARY_GOLD_G, COLOR_PRIMARY_GOLD_B));

            row![fold_button, header_label].spacing(UI_ROW_SPACING).into()
        }
        OrgLineType::Task { is_checked, label } => {
            let line_index = parsed_line.line_index;
            let check_box = checkbox("", *is_checked).on_toggle(move |_| NoteBoxMessage::TaskStateToggled(line_index));

            let text_color = if *is_checked {
                Color::from_rgb(COLOR_TEXT_CHECKED_R, COLOR_TEXT_CHECKED_G, COLOR_TEXT_CHECKED_B)
            } else {
                Color::from_rgb(COLOR_TEXT_NORMAL_R, COLOR_TEXT_NORMAL_G, COLOR_TEXT_NORMAL_B)
            };

            // label'ı String'e çevirerek widget'a veriyoruz (owned)
            let label_text = text(label.to_string()).size(FONT_SIZE_BODY_TEXT).color(text_color);

            row![check_box, label_text].spacing(UI_ROW_SPACING).into()
        }
        OrgLineType::TableBorder => {
            text("")
                .size(FONT_SIZE_BODY_TEXT)
                .color(Color::from_rgb(COLOR_TEXT_NORMAL_R, COLOR_TEXT_NORMAL_G, COLOR_TEXT_NORMAL_B))
                .into()
        }
        OrgLineType::TableRow { cells } => {
            let mut table_row = row![text("| ").size(FONT_SIZE_BODY_TEXT).color(Color::from_rgb(COLOR_TEXT_NORMAL_R, COLOR_TEXT_NORMAL_G, COLOR_TEXT_NORMAL_B))];
            for cell in cells {
                table_row = table_row.push(
                    text(cell.to_string())
                        .size(FONT_SIZE_BODY_TEXT)
                        .color(Color::from_rgb(COLOR_TEXT_NORMAL_R, COLOR_TEXT_NORMAL_G, COLOR_TEXT_NORMAL_B))
                );
                table_row = table_row.push(
                    text(" | ")
                        .size(FONT_SIZE_BODY_TEXT)
                        .color(Color::from_rgb(COLOR_TEXT_NORMAL_R, COLOR_TEXT_NORMAL_G, COLOR_TEXT_NORMAL_B))
                );
            }
            table_row.into()
        }
        OrgLineType::StandardText { text: t } => {
            text(t.to_string())
                .size(FONT_SIZE_BODY_TEXT)
                .color(Color::from_rgb(COLOR_TEXT_NORMAL_R, COLOR_TEXT_NORMAL_G, COLOR_TEXT_NORMAL_B))
                .into()
        }
    };

    container(content_element)
        .padding(iced::Padding {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: indentation_padding as f32,
        })
        .into()
}

fn build_container_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(
            COLOR_CONTAINER_BG_R,
            COLOR_CONTAINER_BG_G,
            COLOR_CONTAINER_BG_B,
        ))),
        border: iced::Border {
            color: Color::from_rgb(COLOR_BORDER_R, COLOR_BORDER_G, COLOR_BORDER_B),
            width: UI_BORDER_WIDTH,
            radius: UI_BORDER_RADIUS.into(),
        },
        ..Default::default()
    }
}

fn custom_scrollbar_style(_theme: &Theme, _status: scrollable::Status) -> scrollable::Style {
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: scrollable::Rail {
            background: Some(iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.15))),
            border: iced::Border::default(),
            scroller: scrollable::Scroller {
                color: Color::from_rgb(COLOR_PRIMARY_GOLD_R, COLOR_PRIMARY_GOLD_G, COLOR_PRIMARY_GOLD_B),
                border: iced::Border::default(),
            },
        },
        horizontal_rail: scrollable::Rail {
            background: None,
            border: iced::Border::default(),
            scroller: scrollable::Scroller {
                color: Color::TRANSPARENT,
                border: iced::Border::default(),
            },
        },
        gap: None,
    }
}
