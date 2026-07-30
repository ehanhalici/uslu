// src/sidebar.rs
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Image};
use iced::widget::{
    button, column, container, row, scrollable, slider, text, text_editor, text_input,
};
use iced::{Color, Element, Font, Length, Point, Rectangle, Renderer, Size, Theme};
use crate::image::{ImageCropperState, BASE_IMAGE_SIZE, CROPPER_CANVAS_SIZE};
use crate::models::{FocusNode, NodeStatus};
use uuid::Uuid;
use crate::notebox::{NoteBoxMessage, NoteBoxState, NoteBoxView};

pub const MATERIAL_FONT: Font = Font {
    family: iced::font::Family::Name("Material Symbols Outlined"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

const ICON_SETTINGS: &str = "\u{e8b8}";       // ⚙
const ICON_DESCRIPTION: &str = "\u{e873}";    // 📝
const ICON_PROGRESS_NOTES: &str = "\u{e85d}"; // 📌
const ICON_INFO: &str = "\u{e88e}";           // 📊
const ICON_RESET: &str = "\u{e5d5}";          // 🔄
const ICON_LOCK: &str = "\u{e897}";           // 🔒
const ICON_EDIT: &str = "\u{e3c9}";           // ✏️

const ICON_SIZE_NAV: u16 = 16;
const ICON_SIZE_BUTTON: u16 = 12;

const TEXT_SIZE_TITLE: u16 = 13;
const TEXT_SIZE_SUBTITLE: u16 = 10;
const TEXT_SIZE_NORMAL: u16 = 12;
const TEXT_SIZE_LABEL: u16 = 11;
const TEXT_SIZE_BUTTON: u16 = 11;
const TEXT_SIZE_HINT: u16 = 9;

const SIDEBAR_PANEL_WIDTH: u16 = 360;
const TAB_STRIP_WIDTH: u16 = 48;
const TAB_CONTAINER_HEIGHT: f32 = 360.0;

const EDITOR_PADDING: f32 = 4.0;

const SCROLLBAR_WIDTH: f32 = 1.0;

const COLOR_GOLD: Color = Color::from_rgb(0.83, 0.68, 0.21);
const COLOR_GOLD_DARK: Color = Color::from_rgb(0.15, 0.15, 0.18);
const COLOR_BG_DARK: Color = Color::from_rgb(0.04, 0.04, 0.05);
const COLOR_BG_PANEL: Color = Color::from_rgb(0.11, 0.11, 0.13);
const COLOR_BG_CARD: Color = Color::from_rgb(0.13, 0.13, 0.16);
const COLOR_BG_CONTAINER: Color = Color::from_rgb(0.09, 0.09, 0.12);
const COLOR_BG_BUTTON_INACTIVE: Color = Color::from_rgb(0.16, 0.16, 0.19);
const COLOR_BG_RESET_BTN: Color = Color::from_rgb(0.22, 0.22, 0.26);

const COLOR_BORDER: Color = Color::from_rgb(0.16, 0.16, 0.20);
const COLOR_BORDER_CROPPER: Color = Color::from_rgb(0.23, 0.23, 0.27);
const COLOR_DELETE_BTN: Color = Color::from_rgb(0.85, 0.22, 0.22);

const COLOR_TEXT_MUTED: Color = Color::from_rgb(0.47, 0.47, 0.50);
const COLOR_TEXT_LABEL: Color = Color::from_rgb(0.56, 0.56, 0.60);
const COLOR_TEXT_ACTIVE_LEVEL: Color = Color::from_rgb(0.48, 0.78, 0.48);

const BUTTON_LABEL_LOCK: &str = "Tamam (Görünüme Geç)";
const BUTTON_LABEL_EDIT: &str = "Metni Düzenle";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabType {
    General,
    Description,
    ProgressNotes,
    Info,
}

impl TabType {
    pub fn title(&self) -> &'static str {
        match self {
            Self::General => "Genel",
            Self::Description => "Açıklama",
            Self::ProgressNotes => "İlerleme Notları",
            Self::Info => "Bilgi & Seviye",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::General => ICON_SETTINGS,
            Self::Description => ICON_DESCRIPTION,
            Self::ProgressNotes => ICON_PROGRESS_NOTES,
            Self::Info => ICON_INFO,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SidebarMessage {
    TitleChanged(String),
    ProgressChanged(f32),

    DescriptionAction(text_editor::Action),
    ProgressNotesAction(text_editor::Action),
    
    OpenImagePicker,
    CropZoomChanged(f32),
    CropPanMoved { delta_x: f32, delta_y: f32 },
    ApplyCropAndSave,
    CancelCrop,

    ToggleEditing,
    MaxLevelSliderChanged(usize),

    DescriptionNoteBox(NoteBoxMessage),
    ProgressNotesNoteBox(NoteBoxMessage),

    ToggleTab(TabType),
    AddNode,
    DeleteSelected,
    ResetView,
}

#[derive(Debug)]
pub struct NodeForm {
    pub title: String,
    pub image_id: Option<Uuid>,
    pub progress: f32,
    pub description_editor: text_editor::Content,
    pub progress_notes_editor: text_editor::Content,
    pub description_notebox_state: NoteBoxState,
    pub progress_notes_notebox_state: NoteBoxState,
}

impl Clone for NodeForm {
    fn clone(&self) -> Self {
        Self {
            title: self.title.clone(),
            image_id: self.image_id,
            progress: self.progress,
            description_editor: text_editor::Content::with_text(&self.description_editor.text()),
            progress_notes_editor: text_editor::Content::with_text(
                &self.progress_notes_editor.text(),
            ),
            description_notebox_state: self.description_notebox_state.clone(),
            progress_notes_notebox_state: self.progress_notes_notebox_state.clone(),
        }
    }
}

impl Default for NodeForm {
    fn default() -> Self {
        Self {
            title: String::new(),
            image_id: None,
            progress: 0.0,
            description_editor: text_editor::Content::new(),
            progress_notes_editor: text_editor::Content::new(),
            description_notebox_state: NoteBoxState::default(),
            progress_notes_notebox_state: NoteBoxState::default(),

        }
    }
}

impl NodeForm {
    pub fn from_node(node: &FocusNode) -> Self {
        Self {
            title: node.title.clone(),
            image_id: node.image_id,
            progress: node.status.progress,
            description_editor: text_editor::Content::with_text(&node.description),
            progress_notes_editor: text_editor::Content::with_text(&node.progress_notes),
            description_notebox_state: NoteBoxState::default(),
            progress_notes_notebox_state: NoteBoxState::default(),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.title.trim().is_empty()
    }

    pub fn to_status(&self) -> NodeStatus {
        NodeStatus {
            progress: self.progress.clamp(0.0, 100.0),
        }
    }

    pub fn get_description_text(&self) -> String {
        self.description_editor.text()
    }

    pub fn get_progress_notes_text(&self) -> String {
        self.progress_notes_editor.text()
    }
    pub fn description_text(&self) -> String {
        self.description_editor.text()
    }

    pub fn progress_notes_text(&self) -> String {
        self.progress_notes_editor.text()
    }
}

pub fn sidebar_view<'a>(
    form: &'a NodeForm,
    selected_node: Option<&'a FocusNode>,
    node_count: usize,
    edge_count: usize,
    open_tabs: &'a [TabType],
    cropper_state: Option<&'a ImageCropperState>,
    is_editing_enabled: bool,
    level_counts: &std::collections::HashMap<usize, usize>,
    max_visible_level: usize,
) -> Element<'a, SidebarMessage> {
    if let Some(cropper) = cropper_state {
        return build_cropper_view(cropper);
    }

    let tab_strip = build_tab_strip(open_tabs);

    if open_tabs.is_empty() {
        return row![tab_strip].into();
    }

    let main_stack = build_tabs_content_stack(
        form,
        selected_node,
        node_count,
        edge_count,
        open_tabs,
        is_editing_enabled,
        level_counts,
        max_visible_level,
    );

    let scrollable_panel = container(scrollable(main_stack).height(Length::Fill))
        .width(SIDEBAR_PANEL_WIDTH)
        .height(Length::Fill)
        .padding(10)
        .style(|_t| container::Style {
            background: Some(iced::Background::Color(COLOR_BG_PANEL)),
            ..Default::default()
        });

    row![tab_strip, scrollable_panel].into()
}

fn build_tab_strip<'a>(open_tabs: &'a [TabType]) -> Element<'a, SidebarMessage> {
    let mut top_strip = column![].spacing(8);

    for tab in &[
        TabType::General,
        TabType::Description,
        TabType::ProgressNotes,
        TabType::Info,
    ] {
        let is_open = open_tabs.contains(tab);
        let btn = button(text(tab.icon()).font(MATERIAL_FONT).size(ICON_SIZE_NAV))
            .on_press(SidebarMessage::ToggleTab(*tab))
            .padding(8)
            .style(move |_t, _s| button::Style {
                background: Some(iced::Background::Color(if is_open {
                    COLOR_GOLD
                } else {
                    COLOR_BG_BUTTON_INACTIVE
                })),
                text_color: if is_open { Color::BLACK } else { Color::WHITE },
                border: iced::border::rounded(6),
                ..Default::default()
            });

        top_strip = top_strip.push(btn);
    }

    let reset_btn = button(text(ICON_RESET).font(MATERIAL_FONT).size(ICON_SIZE_NAV))
        .on_press(SidebarMessage::ResetView)
        .padding(8)
        .style(|_t, _s| button::Style {
            background: Some(iced::Background::Color(COLOR_BG_RESET_BTN)),
            text_color: Color::WHITE,
            border: iced::border::rounded(6),
            ..Default::default()
        });

    container(
        column![
            top_strip,
            container(reset_btn)
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Bottom)
        ]
        .padding(6)
        .height(Length::Fill),
    )
    .width(TAB_STRIP_WIDTH)
    .height(Length::Fill)
    .style(|_t| container::Style {
        background: Some(iced::Background::Color(COLOR_GOLD_DARK)),
        ..Default::default()
    })
    .into()
}

fn build_tabs_content_stack<'a>(
    form: &'a NodeForm,
    selected_node: Option<&'a FocusNode>,
    node_count: usize,
    edge_count: usize,
    open_tabs: &'a [TabType],
    is_editing_enabled: bool,
    level_counts: &std::collections::HashMap<usize, usize>,
    max_visible_level: usize,
) -> iced::widget::Column<'a, SidebarMessage> {
    let mut stack = column![].spacing(16).width(Length::Fill);

    for tab in open_tabs {
        let tab_card = build_single_tab_card(
            *tab,
            form,
            selected_node,
            node_count,
            edge_count,
            is_editing_enabled,
            level_counts,
            max_visible_level,
        );

        let card_container = container(tab_card)
            .width(Length::Fill)
            .padding(10)
            .style(|_t| container::Style {
                background: Some(iced::Background::Color(COLOR_BG_CARD)),
                ..Default::default()
            });

        stack = stack.push(card_container);
    }

    stack
}

fn build_single_tab_card<'a>(
    tab: TabType,
    form: &'a NodeForm,
    selected_node: Option<&'a FocusNode>,
    node_count: usize,
    edge_count: usize,
    is_editing_enabled: bool,
    level_counts: &std::collections::HashMap<usize, usize>,
    max_visible_level: usize,
) -> Element<'a, SidebarMessage> {
    let header = row![
        text(tab.icon()).font(MATERIAL_FONT).size(TEXT_SIZE_TITLE).color(COLOR_GOLD),
        text(tab.title()).size(TEXT_SIZE_TITLE).color(COLOR_GOLD),
        text(format!("({} düğüm, {} bağ)", node_count, edge_count))
            .size(TEXT_SIZE_SUBTITLE)
            .color(COLOR_TEXT_MUTED),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let content: Element<'a, SidebarMessage> = match tab {
        TabType::General => build_general_tab_content(form, selected_node, is_editing_enabled),
        TabType::Description => build_description_tab_content(form, is_editing_enabled),
        TabType::ProgressNotes => build_progress_notes_tab_content(form, is_editing_enabled),
        TabType::Info => build_info_tab_content(node_count, edge_count, level_counts, max_visible_level),
    };

    column![header, content].spacing(8).into()
}

fn build_general_tab_content<'a>(
    form: &'a NodeForm,
    selected_node: Option<&'a FocusNode>,
    is_editing_enabled: bool,
) -> Element<'a, SidebarMessage> {
    let mut gen_box = column![].spacing(10);

    if selected_node.is_some() {
        let (icon, label_text) = if is_editing_enabled {
            (ICON_LOCK, "Düzenlemeyi Kilitle")
        } else {
            (ICON_EDIT, "Metin/Resim Düzenle")
        };

        let lock_btn_content = row![
            text(icon).font(MATERIAL_FONT).size(ICON_SIZE_BUTTON),
            text(label_text).size(TEXT_SIZE_BUTTON),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);

        gen_box = gen_box.push(
            button(lock_btn_content)
                .on_press(SidebarMessage::ToggleEditing)
                .padding(8),
        );
    }

    if is_editing_enabled || selected_node.is_none() {
        gen_box = gen_box.push(build_title_input_field(&form.title));
        gen_box = gen_box.push(build_image_picker_field());
    }

    gen_box = gen_box.push(build_progress_slider_field(form.progress));

    if selected_node.is_none() {
        gen_box = gen_box.push(
            button(text("Yeni Düğüm Ekle").size(TEXT_SIZE_BUTTON))
                .on_press_maybe(if form.is_valid() {
                    Some(SidebarMessage::AddNode)
                } else {
                    None
                })
                .padding(8),
        );
    } else {
        gen_box = gen_box.push(
            button(text("Seçili Düğümü Sil").size(TEXT_SIZE_BUTTON))
                .on_press(SidebarMessage::DeleteSelected)
                .padding(8)
                .style(|_t, _s| button::Style {
                    background: Some(iced::Background::Color(COLOR_DELETE_BTN)),
                    text_color: Color::WHITE,
                    border: iced::border::rounded(4),
                    ..Default::default()
                }),
        );
    }

    gen_box.into()
}

fn build_title_input_field<'a>(title: &str) -> Element<'a, SidebarMessage> {
    column![
        text("Başlık").size(TEXT_SIZE_LABEL).color(COLOR_TEXT_LABEL),
        text_input("Başlık...", title)
            .on_input(SidebarMessage::TitleChanged)
            .padding(6),
    ]
    .spacing(2)
    .into()
}

fn build_image_picker_field<'a>() -> Element<'a, SidebarMessage> {
    column![
        text("Resim Seç")
            .size(TEXT_SIZE_LABEL)
            .color(COLOR_TEXT_LABEL),
        button(text("Görsel Değiştir...").size(TEXT_SIZE_LABEL))
            .on_press(SidebarMessage::OpenImagePicker)
            .padding(6),
    ]
    .spacing(2)
    .into()
}

fn build_progress_slider_field<'a>(progress: f32) -> Element<'a, SidebarMessage> {
    column![
        row![
            text("İlerleme")
                .size(TEXT_SIZE_LABEL)
                .color(COLOR_TEXT_LABEL),
            text(format!("%{:.0}", progress))
                .size(TEXT_SIZE_LABEL)
                .color(COLOR_GOLD),
        ]
        .spacing(8),
        slider(0.0..=100.0, progress, SidebarMessage::ProgressChanged).step(1.0_f32),
    ]
    .spacing(2)
    .into()
}

fn build_description_tab_content<'a>(
    form: &'a NodeForm,
    is_editing_enabled: bool,
) -> Element<'a, SidebarMessage> {
    let header_button = build_tab_toggle_button(is_editing_enabled);

    let content_view = if is_editing_enabled {
        build_text_editor_container(
            &form.description_editor,
            SidebarMessage::DescriptionAction,
        )
    } else {
        NoteBoxView::render_viewer(
            &form.get_description_text(),
            &form.description_notebox_state,
        )
        .map(SidebarMessage::DescriptionNoteBox)
    };

    column![row![header_button].padding(2), content_view]
        .spacing(6)
        .into()
}

fn build_progress_notes_tab_content<'a>(
    form: &'a NodeForm,
    is_editing_enabled: bool,
) -> Element<'a, SidebarMessage> {
    let header_button = build_tab_toggle_button(is_editing_enabled);

    let content_view = if is_editing_enabled {
        build_text_editor_container(
            &form.progress_notes_editor,
            SidebarMessage::ProgressNotesAction,
        )
    } else {
        NoteBoxView::render_viewer(
            &form.get_progress_notes_text(),
            &form.progress_notes_notebox_state,
        )
        .map(SidebarMessage::ProgressNotesNoteBox)
    };

    column![row![header_button].padding(2), content_view]
        .spacing(6)
        .into()
}

fn build_info_tab_content<'a>(
    node_count: usize,
    edge_count: usize,
    level_counts: &std::collections::HashMap<usize, usize>,
    max_visible_level: usize,
) -> Element<'a, SidebarMessage> {
    let mut info_box = column![].spacing(12);

    info_box = info_box.push(
        text(format!("Toplam Düğüm: {}", node_count))
            .size(TEXT_SIZE_TITLE)
            .color(Color::WHITE),
    );
    info_box = info_box.push(
        text(format!("Toplam Bağlantı: {}", edge_count))
            .size(TEXT_SIZE_NORMAL)
            .color(COLOR_TEXT_MUTED),
    );

    let total_levels = level_counts.keys().max().map(|m| m + 1).unwrap_or(0);
    info_box = info_box.push(
        text(format!("Toplam Seviye Sayısı: {}", total_levels))
            .size(TEXT_SIZE_NORMAL)
            .color(COLOR_GOLD),
    );

    let mut lvl_list = column![].spacing(4);
    for lvl in 0..total_levels {
        let count = level_counts.get(&lvl).copied().unwrap_or(0);
        lvl_list = lvl_list.push(
            text(format!(" • Seviye {}: {} düğüm", lvl + 1, count))
                .size(TEXT_SIZE_LABEL)
                .color(if lvl < max_visible_level {
                    COLOR_TEXT_ACTIVE_LEVEL
                } else {
                    COLOR_TEXT_MUTED
                }),
        );
    }
    info_box = info_box.push(lvl_list);

    if total_levels > 0 {
        info_box = info_box.push(
            column![
                text(format!("Açık Seviye Sınırı: Seviye 1 - {}", max_visible_level))
                    .size(TEXT_SIZE_LABEL)
                    .color(COLOR_GOLD),
                slider(1.0..=(total_levels as f32), max_visible_level as f32, |val| {
                    SidebarMessage::MaxLevelSliderChanged(val as usize)
                })
                .step(1.0_f32),
                text("Not: Slider ile toplu açıp kapatabilir, tuvalde Ctrl+Tık ile esnek müdahale edebilirsiniz.")
                    .size(TEXT_SIZE_HINT)
                    .color(COLOR_TEXT_MUTED)
            ]
            .spacing(6),
        );
    }

    info_box.into()
}

fn custom_thin_scrollbar_style(_theme: &Theme, _status: scrollable::Status) -> scrollable::Style {
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: scrollable::Rail {
            background: Some(iced::Background::Color(Color::from_rgba8(0, 0, 0, 0.15))),
            border: iced::Border::default(),
            scroller: scrollable::Scroller {
                color: COLOR_GOLD,
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

fn build_tab_toggle_button<'a>(is_editing_enabled: bool) -> Element<'a, SidebarMessage> {
    let (icon, label_text) = if is_editing_enabled {
        (ICON_LOCK, BUTTON_LABEL_LOCK)
    } else {
        (ICON_EDIT, BUTTON_LABEL_EDIT)
    };

    let btn_content = row![
        text(icon).font(MATERIAL_FONT).size(TEXT_SIZE_BUTTON),
        text(label_text).size(TEXT_SIZE_BUTTON),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    button(btn_content)
        .on_press(SidebarMessage::ToggleEditing)
        .padding(6)
        .into()
}

fn build_text_editor_container<'a>(
    editor_content: &'a text_editor::Content,
    on_action_msg: fn(text_editor::Action) -> SidebarMessage,
) -> Element<'a, SidebarMessage> {
    let editor_widget = text_editor(editor_content)
        .on_action(on_action_msg)
        .padding(EDITOR_PADDING)
        .height(Length::Shrink);

    let thin_scrollbar = scrollable::Scrollbar::new()
        .width(SCROLLBAR_WIDTH)
        .margin(0.0)
        .scroller_width(SCROLLBAR_WIDTH);

    let scrollable_editor = scrollable(editor_widget)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(thin_scrollbar))
        .style(custom_thin_scrollbar_style);

    container(scrollable_editor)
        .height(TAB_CONTAINER_HEIGHT)
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(COLOR_BG_CONTAINER)),
            border: iced::Border {
                color: COLOR_BORDER,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
}


fn build_cropper_view<'a>(cropper: &'a ImageCropperState) -> Element<'a, SidebarMessage> {
    let cropper_canvas = canvas::Canvas::new(CropperProgram { state: cropper })
        .width(CROPPER_CANVAS_SIZE)
        .height(CROPPER_CANVAS_SIZE);

    let canvas_container = container(cropper_canvas)
        .width(CROPPER_CANVAS_SIZE)
        .height(CROPPER_CANVAS_SIZE)
        .clip(true)
        .style(|_t| container::Style {
            background: Some(iced::Background::Color(COLOR_BG_DARK)),
            border: iced::Border {
                color: COLOR_BORDER_CROPPER,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        });

    let cropper_view = column![
        text("Kadrajı Ayarlayın")
            .size(14)
            .color(COLOR_GOLD),
        text("Fare ile sürükleyin, tekerlek ile zoom yapın.")
            .size(10)
            .color(COLOR_TEXT_MUTED),
        canvas_container,
        row![
            button(text("Kırp ve Kaydet").size(TEXT_SIZE_NORMAL))
                .on_press(SidebarMessage::ApplyCropAndSave)
                .padding(8),
            button(text("İptal").size(TEXT_SIZE_NORMAL))
                .on_press(SidebarMessage::CancelCrop)
                .padding(8),
        ]
        .spacing(8)
    ]
    .spacing(12)
    .padding(12);

    container(cropper_view)
        .width(SIDEBAR_PANEL_WIDTH)
        .height(Length::Fill)
        .style(|_t| container::Style {
            background: Some(iced::Background::Color(COLOR_BG_PANEL)),
            ..Default::default()
        })
        .into()
}

#[derive(Default)]
pub enum CropperInteraction {
    #[default]
    Idle,
    Dragging {
        last_pos: Point,
    },
}

pub struct CropperProgram<'a> {
    pub state: &'a ImageCropperState,
}

impl<'a> canvas::Program<SidebarMessage> for CropperProgram<'a> {
    type State = CropperInteraction;

    fn update(
        &self,
        interaction: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (iced::event::Status, Option<SidebarMessage>) {
        let cursor_pos = match cursor.position_in(bounds) {
            Some(p) => p,
            None => return (iced::event::Status::Ignored, None),
        };

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                *interaction = CropperInteraction::Dragging {
                    last_pos: cursor_pos,
                };
                (iced::event::Status::Captured, None)
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                *interaction = CropperInteraction::Idle;
                (iced::event::Status::Captured, None)
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let CropperInteraction::Dragging { last_pos } = interaction {
                    let delta_x = cursor_pos.x - last_pos.x;
                    let delta_y = cursor_pos.y - last_pos.y;
                    *last_pos = cursor_pos;
                    return (
                        iced::event::Status::Captured,
                        Some(SidebarMessage::CropPanMoved { delta_x, delta_y }),
                    );
                }
                (iced::event::Status::Ignored, None)
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let zoom_delta = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * 0.1,
                    mouse::ScrollDelta::Pixels { y, .. } => y * 0.002,
                };
                const MIN_ZOOM: f32 = CROPPER_CANVAS_SIZE / BASE_IMAGE_SIZE;
                let new_zoom = (self.state.zoom + zoom_delta).clamp(MIN_ZOOM, 5.0);
                (
                    iced::event::Status::Captured,
                    Some(SidebarMessage::CropZoomChanged(new_zoom)),
                )
            }
            _ => (iced::event::Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        _interaction: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);

        let (orig_w, orig_h) = (
            self.state.original_image.width() as f32,
            self.state.original_image.height() as f32,
        );
        let aspect_ratio = orig_w / orig_h;

        let base_size = BASE_IMAGE_SIZE;
        let (base_img_w, base_img_h) = if aspect_ratio >= 1.0 {
            (base_size * aspect_ratio, base_size)
        } else {
            (base_size, base_size / aspect_ratio)
        };

        let img_w = base_img_w * self.state.zoom;
        let img_h = base_img_h * self.state.zoom;

        let img_pos = Point::new(
            center.x - img_w / 2.0 + self.state.offset_x,
            center.y - img_h / 2.0 + self.state.offset_y,
        );

        frame.with_clip(Rectangle::with_size(bounds.size()), |frame| {
            frame.draw_image(
                Rectangle::new(img_pos, Size::new(img_w, img_h)),
                Image::new(self.state.image_handle.clone()),
            );
        });

        frame.stroke_rectangle(
            Point::ORIGIN,
            bounds.size(),
            canvas::Stroke {
                style: canvas::Style::Solid(COLOR_BORDER_CROPPER),
                width: 1.0,
                ..Default::default()
            },
        );

        vec![frame.into_geometry()]
    }
}

// sidebar.rs dosyasının en altına ekleyiniz:

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_task_on_line() {
        let input = "- [ ] Görev 1\n- [x] Görev 2\nNormal satır";
    }
}
