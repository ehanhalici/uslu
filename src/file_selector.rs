// src/file_selector.rs
use iced::widget::{button, column, container, image, row, scrollable, text, text_input};
use iced::{Color, Element, Font, Length, Theme};
use ::image::{load_from_memory, ImageFormat};
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path as StdPath, PathBuf};
use std::time::{Duration, Instant, SystemTime};

// =============================================================================
// Material Font & İkon Sabitleri
// =============================================================================
const FONT_FAMILY_NAME: &str = "Material Symbols Outlined";

const MATERIAL_FONT: Font = Font {
    family: iced::font::Family::Name(FONT_FAMILY_NAME),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

// Material Unicode İkonları
const ICON_DIRECTORY: &str = "\u{e2c7}";   // folder
const ICON_FILE_IMAGE: &str = "\u{e3f4}";  // image
const ICON_ARROW_UP: &str = "\u{e5d8}";    // arrow_upward
const ICON_SORT_ASC: &str = "\u{e5d8}";    // arrow_upward
const ICON_SORT_DESC: &str = "\u{e5db}";   // arrow_downward

const TEXT_UP_DIRECTORY: &str = " Üst Dizin";
const TEXT_SELECT_BUTTON: &str = "Seç";
const TEXT_CANCEL_BUTTON: &str = "İptal";
const TEXT_NO_PREVIEW: &str = "Ön İzleme Yok";

// =============================================================================
// Yapılandırma ve Ölçü Sabitleri
// =============================================================================
const DOUBLE_CLICK_TIME_THRESHOLD_MILLISECONDS: u64 = 400;

const INITIAL_SPLITTER_RATIO: f32 = 0.68;
const MIN_PANEL_RATIO: f32 = 0.30;
const MAX_PANEL_RATIO: f32 = 0.85;
const SPLITTER_BAR_WIDTH: f32 = 6.0;

const MAX_PREVIEW_DIMENSION: u32 = 480;

const UI_PADDING_SMALL: f32 = 6.0;
const UI_PADDING_MEDIUM: f32 = 12.0;
const UI_SPACING_SMALL: f32 = 4.0;
const UI_SPACING_MEDIUM: f32 = 10.0;
const UI_BORDER_WIDTH: f32 = 1.0;
const UI_BORDER_RADIUS: f32 = 6.0;
const UI_SCROLLBAR_WIDTH: f32 = 4.0;

// Dark Modern Renk Paleti (Apple Finder Teması)
const COLOR_BG_ROOT_R: f32 = 0.09;
const COLOR_BG_ROOT_G: f32 = 0.09;
const COLOR_BG_ROOT_B: f32 = 0.11;

const COLOR_BG_PANEL_R: f32 = 0.12;
const COLOR_BG_PANEL_G: f32 = 0.12;
const COLOR_BG_PANEL_B: f32 = 0.15;

const COLOR_BG_ROW_EVEN_R: f32 = 0.13;
const COLOR_BG_ROW_EVEN_G: f32 = 0.13;
const COLOR_BG_ROW_EVEN_B: f32 = 0.16;

const COLOR_BG_ROW_ODD_R: f32 = 0.15;
const COLOR_BG_ROW_ODD_G: f32 = 0.15;
const COLOR_BG_ROW_ODD_B: f32 = 0.18;

const COLOR_BG_ROW_SELECTED_R: f32 = 0.14;
const COLOR_BG_ROW_SELECTED_G: f32 = 0.38;
const COLOR_BG_ROW_SELECTED_B: f32 = 0.72;

const COLOR_BORDER_R: f32 = 0.20;
const COLOR_BORDER_G: f32 = 0.20;
const COLOR_BORDER_B: f32 = 0.24;

const COLOR_ACCENT_GOLD_R: f32 = 0.83;
const COLOR_ACCENT_GOLD_G: f32 = 0.68;
const COLOR_ACCENT_GOLD_B: f32 = 0.21;

const COLOR_TEXT_PRIMARY_R: f32 = 0.88;
const COLOR_TEXT_PRIMARY_G: f32 = 0.88;
const COLOR_TEXT_PRIMARY_B: f32 = 0.90;

const COLOR_TEXT_MUTED_R: f32 = 0.52;
const COLOR_TEXT_MUTED_G: f32 = 0.52;
const COLOR_TEXT_MUTED_B: f32 = 0.56;

const SUPPORTED_IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "gif"];

// =============================================================================
// Enum'lar ve Veri Yapıları
// =============================================================================
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum SortColumn {
    Name,
    Size,
    Modified,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone)]
pub enum FileSelectorMessage {
    PathInputChanged(String),
    NavigateToPath(PathBuf),
    NavigateUpDirectory,
    EntryClicked(FileEntryItem),
    SortColumnClicked(SortColumn),
    SplitterDragStarted,
    SplitterDragged(f32, f32),
    SplitterDragEnded,
    KeyPressed(iced::keyboard::Key),
    ConfirmSelection,
    CancelSelection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEntryKind {
    Directory,
    SupportedImage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntryItem {
    pub name: String,
    pub path: PathBuf,
    pub kind: FileEntryKind,
    pub size_bytes: u64,
    pub modified_time: SystemTime,
}

pub struct FileSelectorState {
    pub current_directory: PathBuf,
    pub path_input_text: String,
    pub directory_entries: Vec<FileEntryItem>,
    pub selected_entry: Option<FileEntryItem>,
    pub preview_image_handle: Option<image::Handle>,
    
    // RAM & GPU Optimizasyonu İçin Ön Bellek (Cache)
    pub image_preview_cache: HashMap<PathBuf, iced::widget::image::Handle>,

    pub last_click_timestamp: Option<Instant>,
    pub last_clicked_path: Option<PathBuf>,
    pub active_sort_column: SortColumn,
    pub active_sort_direction: SortDirection,
    pub splitter_ratio: f32,
    pub is_dragging_splitter: bool,

    // Otomatik Tamamlama (Autocomplete) State'leri
    pub autocomplete_suggestions: Vec<PathBuf>,
    pub autocomplete_selected_index: Option<usize>,
}

impl FileSelectorState {
    pub fn new(initial_path: Option<PathBuf>) -> Self {
        let starting_directory = determine_starting_directory(initial_path);
        let path_text = starting_directory.to_string_lossy().to_string();
        let mut entries = read_filtered_directory_entries(&starting_directory);

        let default_sort_col = SortColumn::Name;
        let default_sort_dir = SortDirection::Ascending;
        sort_entries(&mut entries, default_sort_col, default_sort_dir);

        Self {
            current_directory: starting_directory,
            path_input_text: path_text,
            directory_entries: entries,
            selected_entry: None,
            preview_image_handle: None,
            image_preview_cache: HashMap::new(),
            last_click_timestamp: None,
            last_clicked_path: None,
            active_sort_column: default_sort_col,
            active_sort_direction: default_sort_dir,
            splitter_ratio: INITIAL_SPLITTER_RATIO,
            is_dragging_splitter: false,
            autocomplete_suggestions: Vec::new(),
            autocomplete_selected_index: None,
        }
    }

    pub fn update(&mut self, message: FileSelectorMessage) -> Option<PathBuf> {
        match message {
            FileSelectorMessage::PathInputChanged(new_path_str) => {
                self.path_input_text = new_path_str.clone();
                self.update_autocomplete_suggestions(&new_path_str);
                None
            }
            FileSelectorMessage::NavigateToPath(target_path) => {
                self.autocomplete_suggestions.clear();
                self.autocomplete_selected_index = None;
                self.change_directory(target_path);
                None
            }
            FileSelectorMessage::NavigateUpDirectory => {
                self.autocomplete_suggestions.clear();
                self.autocomplete_selected_index = None;
                if let Some(parent_path) = self.current_directory.parent().map(|p| p.to_path_buf()) {
                    self.change_directory(parent_path);
                }
                None
            }
            FileSelectorMessage::SortColumnClicked(clicked_column) => {
                if self.active_sort_column == clicked_column {
                    self.active_sort_direction = match self.active_sort_direction {
                        SortDirection::Ascending => SortDirection::Descending,
                        SortDirection::Descending => SortDirection::Ascending,
                    };
                } else {
                    self.active_sort_column = clicked_column;
                    self.active_sort_direction = SortDirection::Ascending;
                }
                sort_entries(&mut self.directory_entries, self.active_sort_column, self.active_sort_direction);
                None
            }
            FileSelectorMessage::SplitterDragStarted => {
                self.is_dragging_splitter = true;
                None
            }
            FileSelectorMessage::SplitterDragged(cursor_x, total_width) => {
                if self.is_dragging_splitter && total_width > 0.0 {
                    let new_ratio = (cursor_x / total_width).clamp(MIN_PANEL_RATIO, MAX_PANEL_RATIO);
                    self.splitter_ratio = new_ratio;
                }
                None
            }
            FileSelectorMessage::SplitterDragEnded => {
                self.is_dragging_splitter = false;
                None
            }
            FileSelectorMessage::KeyPressed(key) => {
                self.handle_key_navigation(key)
            }
            FileSelectorMessage::EntryClicked(clicked_entry) => {
                self.autocomplete_suggestions.clear();
                self.autocomplete_selected_index = None;

                let now = Instant::now();
                let is_double_click = check_if_double_click(
                    &self.last_clicked_path,
                    &clicked_entry.path,
                    self.last_click_timestamp,
                    now,
                );

                self.last_click_timestamp = Some(now);
                self.last_clicked_path = Some(clicked_entry.path.clone());

                if clicked_entry.kind == FileEntryKind::Directory {
                    if is_double_click {
                        self.change_directory(clicked_entry.path);
                    } else {
                        self.selected_entry = Some(clicked_entry);
                        self.preview_image_handle = None;
                    }
                } else if clicked_entry.kind == FileEntryKind::SupportedImage {
                    self.selected_entry = Some(clicked_entry.clone());
                    self.preview_image_handle = self.load_optimized_preview(&clicked_entry.path);
                }
                None
            }
            FileSelectorMessage::ConfirmSelection => {
                if let Some(ref entry) = self.selected_entry {
                    if entry.kind == FileEntryKind::SupportedImage {
                        return Some(entry.path.clone());
                    }
                }
                None
            }
            FileSelectorMessage::CancelSelection => None,
        }
    }

    fn update_autocomplete_suggestions(&mut self, input: &str) {
        if input.is_empty() {
            self.autocomplete_suggestions.clear();
            self.autocomplete_selected_index = None;
            return;
        }

        let input_path = PathBuf::from(input);
        let (search_dir, prefix) = if input.ends_with('/') || input.ends_with('\\') {
            (input_path.clone(), "")
        } else {
            let parent = input_path.parent().unwrap_or_else(|| StdPath::new("/")).to_path_buf();
            let file_name = input_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            (parent, file_name)
        };

        let mut matches = Vec::new();
        if search_dir.is_dir() {
            if let Ok(read_dir) = fs::read_dir(&search_dir) {
                for entry in read_dir.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(folder_name) = path.file_name().and_then(|s| s.to_str()) {
                            if prefix.is_empty() || folder_name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                                matches.push(path);
                            }
                        }
                    }
                }
            }
        }

        matches.sort();
        self.autocomplete_suggestions = matches;
        self.autocomplete_selected_index = if self.autocomplete_suggestions.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    fn handle_key_navigation(&mut self, key: iced::keyboard::Key) -> Option<PathBuf> {
        use iced::keyboard::key::Named;

        // 1. Autocomplete Menüsü Açıksa Klavye Kontrolü
        if !self.autocomplete_suggestions.is_empty() {
            match key {
                iced::keyboard::Key::Named(Named::ArrowDown) => {
                    let current = self.autocomplete_selected_index.unwrap_or(0);
                    let next = (current + 1).min(self.autocomplete_suggestions.len() - 1);
                    self.autocomplete_selected_index = Some(next);
                    return None;
                }
                iced::keyboard::Key::Named(Named::ArrowUp) => {
                    let current = self.autocomplete_selected_index.unwrap_or(0);
                    let prev = current.saturating_sub(1);
                    self.autocomplete_selected_index = Some(prev);
                    return None;
                }
                iced::keyboard::Key::Named(Named::Tab) | iced::keyboard::Key::Named(Named::Enter) => {
                    if let Some(idx) = self.autocomplete_selected_index {
                        if let Some(target_dir) = self.autocomplete_suggestions.get(idx).cloned() {
                            self.autocomplete_suggestions.clear();
                            self.autocomplete_selected_index = None;
                            self.change_directory(target_dir);
                            return None;
                        }
                    }
                }
                iced::keyboard::Key::Named(Named::Escape) => {
                    self.autocomplete_suggestions.clear();
                    self.autocomplete_selected_index = None;
                    return None;
                }
                _ => {}
            }
        }

        // 2. Dosya Listesi İçinde Klavye Kontrolü
        match key {
            iced::keyboard::Key::Named(Named::ArrowUp) => {
                self.select_previous_entry();
            }
            iced::keyboard::Key::Named(Named::ArrowDown) => {
                self.select_next_entry();
            }
            iced::keyboard::Key::Named(Named::ArrowRight) | iced::keyboard::Key::Named(Named::Enter) => {
                if let Some(entry) = self.selected_entry.clone() {
                    match entry.kind {
                        FileEntryKind::Directory => {
                            self.change_directory(entry.path);
                        }
                        FileEntryKind::SupportedImage => {
                            return Some(entry.path);
                        }
                    }
                }
            }
            iced::keyboard::Key::Named(Named::ArrowLeft) | iced::keyboard::Key::Named(Named::Backspace) => {
                if let Some(parent_path) = self.current_directory.parent().map(|p| p.to_path_buf()) {
                    self.change_directory(parent_path);
                }
            }
            _ => {}
        }

        None
    }

    fn select_previous_entry(&mut self) {
        if self.directory_entries.is_empty() {
            return;
        }
        let current_index = self.get_current_selected_index();
        let new_index = match current_index {
            Some(idx) if idx > 0 => idx - 1,
            Some(_) => 0,
            None => 0,
        };
        self.apply_selection_by_index(new_index);
    }

    fn select_next_entry(&mut self) {
        if self.directory_entries.is_empty() {
            return;
        }
        let max_index = self.directory_entries.len().saturating_sub(1);
        let current_index = self.get_current_selected_index();
        let new_index = match current_index {
            Some(idx) if idx < max_index => idx + 1,
            Some(_) => max_index,
            None => 0,
        };
        self.apply_selection_by_index(new_index);
    }

    fn get_current_selected_index(&self) -> Option<usize> {
        let selected = self.selected_entry.as_ref()?;
        self.directory_entries.iter().position(|e| e == selected)
    }

    fn apply_selection_by_index(&mut self, index: usize) {
        if let Some(entry) = self.directory_entries.get(index).cloned() {
            if entry.kind == FileEntryKind::SupportedImage {
                self.preview_image_handle = self.load_optimized_preview(&entry.path);
            } else {
                self.preview_image_handle = None;
            }
            self.selected_entry = Some(entry);
        }
    }

    fn load_optimized_preview(&mut self, path: &PathBuf) -> Option<iced::widget::image::Handle> {
        if let Some(cached_handle) = self.image_preview_cache.get(path) {
            return Some(cached_handle.clone());
        }

        if let Some(handle) = load_resized_image_handle_from_disk(path) {
            self.image_preview_cache.insert(path.clone(), handle.clone());
            Some(handle)
        } else {
            None
        }
    }

    fn change_directory(&mut self, new_directory: PathBuf) {
        if new_directory.is_dir() {
            self.current_directory = new_directory.clone();
            self.path_input_text = new_directory.to_string_lossy().to_string();
            let mut entries = read_filtered_directory_entries(&new_directory);
            sort_entries(&mut entries, self.active_sort_column, self.active_sort_direction);
            self.directory_entries = entries;
            self.selected_entry = None;
            self.preview_image_handle = None;
        }
    }
}

pub struct FileSelectorView;

impl FileSelectorView {
    pub fn render<'a>(state: &'a FileSelectorState) -> Element<'a, FileSelectorMessage> {
        let address_bar_row = render_address_bar_row(state);
        let file_tree_list_widget = render_file_list_panel(
            &state.directory_entries,
            &state.selected_entry,
            state.active_sort_column,
            state.active_sort_direction,
        );
        let preview_panel_widget = render_image_preview_panel(&state.preview_image_handle, &state.selected_entry);

        let left_portion = (state.splitter_ratio * 1000.0) as u16;
        let right_portion = ((1.0 - state.splitter_ratio) * 1000.0) as u16;

        let splitter_bar = button(text(""))
            .width(Length::Fixed(SPLITTER_BAR_WIDTH))
            .height(Length::Fill)
            .style(|_, _| button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(
                    COLOR_BORDER_R,
                    COLOR_BORDER_G,
                    COLOR_BORDER_B,
                ))),
                ..Default::default()
            })
            .on_press(FileSelectorMessage::SplitterDragStarted);

        let main_content_split = row![
            container(file_tree_list_widget)
                .width(Length::FillPortion(left_portion))
                .height(Length::Fill)
                .style(build_panel_container_style),
            splitter_bar,
            container(preview_panel_widget)
                .width(Length::FillPortion(right_portion))
                .height(Length::Fill)
                .style(build_panel_container_style)
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(0);

        let action_buttons_row = render_action_buttons_row(&state.selected_entry);

        let root_layout = column![
            address_bar_row,
            main_content_split,
            action_buttons_row
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(UI_SPACING_MEDIUM)
        .padding(UI_PADDING_MEDIUM);

        container(root_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(build_root_container_style)
            .into()
    }
}

// =============================================================================
// Yardımcı Mantık Fonksiyonları
// =============================================================================
fn determine_starting_directory(provided_path: Option<PathBuf>) -> PathBuf {
    if let Some(path) = provided_path {
        if path.is_dir() {
            return path;
        } else if let Some(parent) = path.parent() {
            return parent.to_path_buf();
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
}

fn read_filtered_directory_entries(directory: &StdPath) -> Vec<FileEntryItem> {
    let mut entries = Vec::new();

    if let Ok(read_dir) = fs::read_dir(directory) {
        for entry_result in read_dir.flatten() {
            let path = entry_result.path();
            let file_name = entry_result.file_name().to_string_lossy().to_string();

            if is_hidden_file(&file_name) {
                continue;
            }

            let metadata = entry_result.metadata().ok();
            let size_bytes = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified_time = metadata.and_then(|m| m.modified().ok()).unwrap_or(SystemTime::UNIX_EPOCH);

            if path.is_dir() {
                entries.push(FileEntryItem {
                    name: file_name,
                    path,
                    kind: FileEntryKind::Directory,
                    size_bytes: 0,
                    modified_time,
                });
            } else if is_supported_image_file(&path) {
                entries.push(FileEntryItem {
                    name: file_name,
                    path,
                    kind: FileEntryKind::SupportedImage,
                    size_bytes,
                    modified_time,
                });
            }
        }
    }

    entries
}

fn sort_entries(entries: &mut [FileEntryItem], col: SortColumn, dir: SortDirection) {
    entries.sort_by(|a, b| {
        let cmp = match (&a.kind, &b.kind) {
            (FileEntryKind::Directory, FileEntryKind::SupportedImage) => std::cmp::Ordering::Less,
            (FileEntryKind::SupportedImage, FileEntryKind::Directory) => std::cmp::Ordering::Greater,
            _ => match col {
                SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortColumn::Size => a.size_bytes.cmp(&b.size_bytes),
                SortColumn::Modified => a.modified_time.cmp(&b.modified_time),
            },
        };

        if dir == SortDirection::Descending {
            cmp.reverse()
        } else {
            cmp
        }
    });
}

fn is_hidden_file(name: &str) -> bool {
    name.starts_with('.')
}

fn is_supported_image_file(path: &StdPath) -> bool {
    if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
        let lower_extension = extension.to_lowercase();
        return SUPPORTED_IMAGE_EXTENSIONS.contains(&lower_extension.as_str());
    }
    false
}

fn check_if_double_click(
    last_path: &Option<PathBuf>,
    current_path: &StdPath,
    last_time: Option<Instant>,
    current_time: Instant,
) -> bool {
    if let (Some(prev_path), Some(prev_time)) = (last_path, last_time) {
        if prev_path == current_path {
            let elapsed = current_time.duration_since(prev_time);
            return elapsed <= Duration::from_millis(DOUBLE_CLICK_TIME_THRESHOLD_MILLISECONDS);
        }
    }
    false
}

fn load_resized_image_handle_from_disk(image_path: &StdPath) -> Option<image::Handle> {
    let bytes = fs::read(image_path).ok()?;
    let dynamic_img = load_from_memory(&bytes).ok()?;

    let (w, h) = (dynamic_img.width(), dynamic_img.height());
    let resized_img = if w > MAX_PREVIEW_DIMENSION || h > MAX_PREVIEW_DIMENSION {
        dynamic_img.thumbnail(MAX_PREVIEW_DIMENSION, MAX_PREVIEW_DIMENSION)
    } else {
        dynamic_img
    };

    let mut png_bytes = Vec::new();
    let mut cursor = Cursor::new(&mut png_bytes);

    resized_img
        .write_to(&mut cursor, ImageFormat::Png)
        .ok()?;

    Some(image::Handle::from_bytes(png_bytes))
}

fn format_file_size(size_bytes: u64, kind: &FileEntryKind) -> String {
    if *kind == FileEntryKind::Directory {
        return "--".to_string();
    }
    if size_bytes < 1024 {
        format!("{} B", size_bytes)
    } else if size_bytes < 1024 * 1024 {
        format!("{:.1} KB", size_bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", size_bytes as f64 / (1024.0 * 1024.0))
    }
}

fn format_system_time(time: SystemTime) -> String {
    if let Ok(duration) = time.duration_since(SystemTime::UNIX_EPOCH) {
        let secs = duration.as_secs();
        let days = secs / 86400;
        let hours = (secs % 86400) / 3600;
        let minutes = (secs % 3600) / 60;
        format!("Gün {} {:02}:{:02}", days, hours, minutes)
    } else {
        "--".to_string()
    }
}

// =============================================================================
// UI Render Yardımcı Fonksiyonları
// =============================================================================
fn render_address_bar_row<'a>(state: &'a FileSelectorState) -> Element<'a, FileSelectorMessage> {
    let icon_up = text(ICON_ARROW_UP).font(MATERIAL_FONT).size(14);
    let label_up = text(TEXT_UP_DIRECTORY).size(12);

    let up_button = button(row![icon_up, label_up].spacing(UI_SPACING_SMALL))
        .on_press(FileSelectorMessage::NavigateUpDirectory)
        .padding(UI_PADDING_SMALL);

    let address_input = text_input("", &state.path_input_text)
        .on_input(FileSelectorMessage::PathInputChanged)
        .on_submit(FileSelectorMessage::NavigateToPath(PathBuf::from(&state.path_input_text)))
        .padding(UI_PADDING_SMALL);

    let mut address_column = column![address_input].width(Length::Fill);

    if !state.autocomplete_suggestions.is_empty() {
        let mut dropdown_column = column![].spacing(2);

        for (idx, path_suggestion) in state.autocomplete_suggestions.iter().take(6).enumerate() {
            let is_highlighted = state.autocomplete_selected_index == Some(idx);
            let path_str = path_suggestion.to_string_lossy().to_string();

            let row_bg = if is_highlighted {
                Color::from_rgb(COLOR_BG_ROW_SELECTED_R, COLOR_BG_ROW_SELECTED_G, COLOR_BG_ROW_SELECTED_B)
            } else {
                Color::from_rgb(COLOR_BG_PANEL_R, COLOR_BG_PANEL_G, COLOR_BG_PANEL_B)
            };

            let item_button = button(text(path_str).size(12))
                .width(Length::Fill)
                .padding(4)
                .style(move |_, _| button::Style {
                    background: Some(iced::Background::Color(row_bg)),
                    ..Default::default()
                })
                .on_press(FileSelectorMessage::NavigateToPath(path_suggestion.clone()));

            dropdown_column = dropdown_column.push(item_button);
        }

        let dropdown_container = container(dropdown_column)
            .width(Length::Fill)
            .style(build_panel_container_style);

        address_column = address_column.push(dropdown_container);
    }

    row![up_button, address_column]
        .width(Length::Fill)
        .spacing(UI_SPACING_MEDIUM)
        .into()
}

fn render_file_list_panel<'a>(
    entries: &'a [FileEntryItem],
    selected_entry: &Option<FileEntryItem>,
    active_col: SortColumn,
    active_dir: SortDirection,
) -> Element<'a, FileSelectorMessage> {
    let header_row = render_table_header_row(active_col, active_dir);

    let mut list_column = column![header_row].spacing(UI_SPACING_SMALL);

    for (index, entry) in entries.iter().enumerate() {
        let is_selected = selected_entry.as_ref() == Some(entry);
        let item_button = render_single_entry_row(entry, index, is_selected);
        list_column = list_column.push(item_button);
    }

    let vertical_scrollbar = scrollable::Scrollbar::new()
        .width(UI_SCROLLBAR_WIDTH)
        .scroller_width(UI_SCROLLBAR_WIDTH);

    scrollable(list_column.padding(UI_PADDING_SMALL))
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(vertical_scrollbar))
        .into()
}

fn render_table_header_row<'a>(
    active_col: SortColumn,
    active_dir: SortDirection,
) -> Element<'a, FileSelectorMessage> {
    let create_header = |col: SortColumn, title: &'static str| -> Element<'a, FileSelectorMessage> {
        let mut header_row = row![text(title).size(12)];
        if active_col == col {
            let sort_icon = if active_dir == SortDirection::Ascending {
                ICON_SORT_ASC
            } else {
                ICON_SORT_DESC
            };
            header_row = header_row.push(text(sort_icon).font(MATERIAL_FONT).size(12));
        }

        button(header_row.spacing(2))
            .width(Length::Fill)
            .padding(UI_PADDING_SMALL)
            .on_press(FileSelectorMessage::SortColumnClicked(col))
            .into()
    };

    row![
        container(create_header(SortColumn::Name, "Ad")).width(Length::FillPortion(10)),
        container(create_header(SortColumn::Size, "Boyut")).width(Length::FillPortion(4)),
        container(create_header(SortColumn::Modified, "Tarih")).width(Length::FillPortion(6)),
    ]
    .width(Length::Fill)
    .spacing(UI_SPACING_SMALL)
    .into()
}

fn render_single_entry_row<'a>(
    entry: &'a FileEntryItem,
    row_index: usize,
    is_selected: bool,
) -> Element<'a, FileSelectorMessage> {
    let icon_str = match entry.kind {
        FileEntryKind::Directory => ICON_DIRECTORY,
        FileEntryKind::SupportedImage => ICON_FILE_IMAGE,
    };

    let text_color = if is_selected {
        Color::WHITE
    } else {
        Color::from_rgb(COLOR_TEXT_PRIMARY_R, COLOR_TEXT_PRIMARY_G, COLOR_TEXT_PRIMARY_B)
    };

    let icon_widget = text(icon_str).font(MATERIAL_FONT).size(16).color(if is_selected {
        Color::WHITE
    } else {
        Color::from_rgb(COLOR_ACCENT_GOLD_R, COLOR_ACCENT_GOLD_G, COLOR_ACCENT_GOLD_B)
    });

    let name_widget = text(&entry.name).size(12).color(text_color);
    let size_widget = text(format_file_size(entry.size_bytes, &entry.kind)).size(12).color(if is_selected { text_color } else { Color::from_rgb(COLOR_TEXT_MUTED_R, COLOR_TEXT_MUTED_G, COLOR_TEXT_MUTED_B) });
    let time_widget = text(format_system_time(entry.modified_time)).size(12).color(if is_selected { text_color } else { Color::from_rgb(COLOR_TEXT_MUTED_R, COLOR_TEXT_MUTED_G, COLOR_TEXT_MUTED_B) });

    let name_cell = row![icon_widget, name_widget].spacing(UI_SPACING_SMALL);

    let row_content = row![
        container(name_cell).width(Length::FillPortion(10)),
        container(size_widget).width(Length::FillPortion(4)),
        container(time_widget).width(Length::FillPortion(6)),
    ]
    .width(Length::Fill)
    .padding(UI_PADDING_SMALL);

    let bg_color = if is_selected {
        Color::from_rgb(COLOR_BG_ROW_SELECTED_R, COLOR_BG_ROW_SELECTED_G, COLOR_BG_ROW_SELECTED_B)
    } else if row_index % 2 == 0 {
        Color::from_rgb(COLOR_BG_ROW_EVEN_R, COLOR_BG_ROW_EVEN_G, COLOR_BG_ROW_EVEN_B)
    } else {
        Color::from_rgb(COLOR_BG_ROW_ODD_R, COLOR_BG_ROW_ODD_G, COLOR_BG_ROW_ODD_B)
    };

    let cloned_entry = entry.clone();
    button(row_content)
        .width(Length::Fill)
        .padding(0)
        .style(move |_, _| button::Style {
            background: Some(iced::Background::Color(bg_color)),
            border: iced::Border::default(),
            ..Default::default()
        })
        .on_press(FileSelectorMessage::EntryClicked(cloned_entry))
        .into()
}

fn render_image_preview_panel<'a>(
    image_handle_opt: &'a Option<image::Handle>,
    selected_entry: &Option<FileEntryItem>,
) -> Element<'a, FileSelectorMessage> {
    if let Some(handle) = image_handle_opt {
        let image_widget = image(handle.clone())
            .width(Length::Fill)
            .height(Length::Fill);

        container(image_widget)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(UI_PADDING_MEDIUM)
            .into()
    } else {
        let text_message = if let Some(entry) = selected_entry {
            if entry.kind == FileEntryKind::Directory {
                "Klasör Seçildi"
            } else {
                TEXT_NO_PREVIEW
            }
        } else {
            TEXT_NO_PREVIEW
        };

        let placeholder_text = text(text_message)
            .size(12)
            .color(Color::from_rgb(COLOR_TEXT_MUTED_R, COLOR_TEXT_MUTED_G, COLOR_TEXT_MUTED_B));

        container(placeholder_text)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}

fn render_action_buttons_row<'a>(
    selected_entry: &Option<FileEntryItem>,
) -> Element<'a, FileSelectorMessage> {
    let cancel_btn = button(text(TEXT_CANCEL_BUTTON).size(12))
        .on_press(FileSelectorMessage::CancelSelection)
        .padding(UI_PADDING_SMALL);

    let is_image_selected = selected_entry
        .as_ref()
        .map(|e| e.kind == FileEntryKind::SupportedImage)
        .unwrap_or(false);

    let mut select_btn = button(text(TEXT_SELECT_BUTTON).size(12)).padding(UI_PADDING_SMALL);

    if is_image_selected {
        select_btn = select_btn.on_press(FileSelectorMessage::ConfirmSelection);
    }

    row![cancel_btn, select_btn]
        .width(Length::Fill)
        .spacing(UI_SPACING_MEDIUM)
        .into()
}

// =============================================================================
// Stil Fonksiyonları
// =============================================================================
fn build_root_container_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(
            COLOR_BG_ROOT_R,
            COLOR_BG_ROOT_G,
            COLOR_BG_ROOT_B,
        ))),
        border: iced::Border {
            color: Color::from_rgb(COLOR_BORDER_R, COLOR_BORDER_G, COLOR_BORDER_B),
            width: UI_BORDER_WIDTH,
            radius: UI_BORDER_RADIUS.into(),
        },
        ..Default::default()
    }
}

fn build_panel_container_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(
            COLOR_BG_PANEL_R,
            COLOR_BG_PANEL_G,
            COLOR_BG_PANEL_B,
        ))),
        border: iced::Border {
            color: Color::from_rgb(COLOR_BORDER_R, COLOR_BORDER_G, COLOR_BORDER_B),
            width: UI_BORDER_WIDTH,
            radius: UI_BORDER_RADIUS.into(),
        },
        ..Default::default()
    }
}
