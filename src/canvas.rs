// src/canvas.rs
use iced::mouse;
use iced::widget::canvas::{self, Event, Frame, Geometry, Image, Path};
use iced::{Color, Element, Point, Rectangle, Renderer, Size, Theme, Vector};
use std::collections::HashMap;
use crate::models::{FocusGraph, FocusNode};
use uuid::Uuid;

const DEFAULT_PAN_X: f32 = 0.0;
const DEFAULT_PAN_Y: f32 = 0.0;
const DEFAULT_ZOOM: f32 = 1.0;

const MIN_ZOOM_LEVEL: f32 = 0.2;
const MAX_ZOOM_LEVEL: f32 = 3.0;

const GRID_SPACING: f32 = 50.0;
const MIN_GRID_PIXEL_SPACING: f32 = 8.0;

const DELETE_NODE_CLICK_RADIUS: f32 = 16.0;
const DELETE_EDGE_CLICK_RADIUS: f32 = 16.0;

const ICON_CONTAINER_SIZE: f32 = 72.0;
const ICON_PLACEHOLDER_SIZE: f32 = 24.0;
const BANNER_HEIGHT: f32 = 32.0;
const BANNER_WIDTH_SCALE: f32 = 0.96;
const PROGRESS_BAR_HEIGHT: f32 = 5.0;

const DELETE_BUTTON_SIZE: f32 = 16.0;
const COLLAPSE_BUTTON_SIZE: f32 = 18.0;
const ARROW_HEAD_SIZE: f32 = 6.0;

const MIN_FONT_SIZE: f32 = 3.0;
const MAX_TITLE_LINES: usize = 2;
const CHAR_WIDTH_RATIO: f32 = 0.58;
const BASE_FONT_SIZE_SCALE: f32 = 10.5;
const BANNER_PADDING: f32 = 8.0;
const MIN_VISIBLE_TITLE_ZOOM: f32 = 0.15;

const SYMBOL_DELETE: &str = "\u{00d7}";    // ×
const SYMBOL_EXPAND: &str = "\u{002b}";    // +
const SYMBOL_COLLAPSE: &str = "\u{2212}";  // −
const ELLIPSIS_CHAR: char = '…';

const COLOR_BG_CANVAS: Color = Color::from_rgb(0.10, 0.10, 0.12);
const COLOR_GRID_LINE: Color = Color::from_rgba(0.25, 0.25, 0.28, 0.33);

const COLOR_BANNER_BG: Color = Color::from_rgb(0.12, 0.13, 0.15);
const COLOR_BANNER_BORDER_SELECTED: Color = Color::from_rgb(1.0, 0.84, 0.0);
const COLOR_BANNER_BORDER_NORMAL: Color = Color::from_rgb(0.60, 0.48, 0.18);
const COLOR_BANNER_INSET_BORDER: Color = Color::from_rgba(0.83, 0.68, 0.21, 0.3);

const COLOR_PROGRESS_BG: Color = Color::from_rgb(0.07, 0.07, 0.08);
const COLOR_PROGRESS_BORDER: Color = Color::from_rgb(0.25, 0.25, 0.28);
const COLOR_PROGRESS_EMPTY: Color = Color::from_rgb(0.85, 0.15, 0.15);
const COLOR_PROGRESS_FULL: Color = Color::from_rgb(0.15, 0.85, 0.20);

const COLOR_EDGE_LINE: Color = Color::from_rgb(0.83, 0.68, 0.21);
const COLOR_DELETE_BTN: Color = Color::from_rgb(0.85, 0.22, 0.22);
const COLOR_EXPAND_BTN: Color = Color::from_rgb(0.18, 0.55, 0.34);
const COLOR_COLLAPSE_BTN: Color = Color::from_rgb(0.27, 0.51, 0.71);

const COLOR_TEXT_TITLE: Color = Color::from_rgb(0.94, 0.90, 0.82);
const COLOR_PLACEHOLDER_BORDER: Color = Color::from_rgba(0.83, 0.68, 0.21, 0.4);

#[derive(Clone)]
pub struct CanvasData<'a> {
    pub graph: &'a FocusGraph,
    pub view: Viewport,
    pub selected: Option<Uuid>,
    pub is_shift_pressed: bool,
    pub is_ctrl_pressed: bool,
    pub loaded_images: &'a HashMap<String, String>,
    pub image_cache: &'a HashMap<String, iced::widget::image::Handle>,
}

#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            pan_x: DEFAULT_PAN_X,
            pan_y: DEFAULT_PAN_Y,
            zoom: DEFAULT_ZOOM,
        }
    }
}

impl Viewport {
    pub fn world_to_screen(&self, point: Point) -> Point {
        Point::new(
            point.x * self.zoom + self.pan_x,
            point.y * self.zoom + self.pan_y,
        )
    }

    pub fn screen_to_world(&self, point: Point) -> Point {
        Point::new(
            (point.x - self.pan_x) / self.zoom,
            (point.y - self.pan_y) / self.zoom,
        )
    }
}

#[derive(Default)]
pub enum Interaction {
    #[default]
    Idle,
    Panning {
        start: Point,
        original_pan: Vector,
    },
    DraggingNode {
        id: Uuid,
        grab_offset: Vector,
    },
}

#[derive(Debug, Clone)]
pub enum CanvasMessage {
    NodeClicked { id: Uuid, shift: bool, ctrl: bool },
    NodeMoved { id: Uuid, x: f32, y: f32 },
    DeleteNodeClicked(Uuid),
    DeleteEdgeClicked { parent_id: Uuid, child_id: Uuid },
    BackgroundClicked,
    ViewChanged(Viewport),
}

pub fn canvas_view<'a>(data: CanvasData<'a>) -> Element<'a, CanvasMessage> {
    canvas::Canvas::new(CanvasProgram { data })
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}

pub struct CanvasProgram<'a> {
    pub data: CanvasData<'a>,
}

impl<'a> canvas::Program<CanvasMessage> for CanvasProgram<'a> {
    type State = Interaction;

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (iced::event::Status, Option<CanvasMessage>) {
        let cursor_pos = match cursor.position_in(bounds) {
            Some(pos) => pos,
            None => return (iced::event::Status::Ignored, None),
        };

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                self.handle_mouse_pressed(state, cursor_pos)
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                *state = Interaction::Idle;
                (iced::event::Status::Captured, None)
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                self.handle_cursor_moved(state, cursor_pos)
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                self.handle_wheel_scrolled(delta, cursor_pos)
            }
            _ => (iced::event::Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), COLOR_BG_CANVAS);
        draw_grid(&mut frame, &self.data.view, bounds);

        self.draw_visible_edges(&mut frame, bounds);
        self.draw_visible_nodes(&mut frame, bounds);

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        match state {
            Interaction::Panning { .. } | Interaction::DraggingNode { .. } => {
                mouse::Interaction::Grabbing
            }
            Interaction::Idle => {
                if let Some(pos) = cursor.position_in(bounds) {
                    let world_pos = self.data.view.screen_to_world(pos);
                    if hit_test_node(self.data.graph, world_pos).is_some() {
                        return mouse::Interaction::Pointer;
                    }
                }
                mouse::Interaction::default()
            }
        }
    }
}

impl<'a> CanvasProgram<'a> {
    fn handle_mouse_pressed(
        &self,
        state: &mut Interaction,
        cursor_pos: Point,
    ) -> (iced::event::Status, Option<CanvasMessage>) {
        let world_pos = self.data.view.screen_to_world(cursor_pos);

        if self.data.is_shift_pressed {
            if let Some(msg) = self.check_delete_target_click(cursor_pos) {
                return (iced::event::Status::Captured, Some(msg));
            }
        }

        if let Some(node) = hit_test_node(self.data.graph, world_pos) {
            if self.data.graph.is_node_visible(node.id) {
                if self.data.is_ctrl_pressed {
                    return (
                        iced::event::Status::Captured,
                        Some(CanvasMessage::NodeClicked {
                            id: node.id,
                            shift: false,
                            ctrl: true,
                        }),
                    );
                }

                if self.data.is_shift_pressed {
                    return (
                        iced::event::Status::Captured,
                        Some(CanvasMessage::NodeClicked {
                            id: node.id,
                            shift: true,
                            ctrl: false,
                        }),
                    );
                }

                *state = Interaction::DraggingNode {
                    id: node.id,
                    grab_offset: Vector::new(world_pos.x - node.x, world_pos.y - node.y),
                };
                return (
                    iced::event::Status::Captured,
                    Some(CanvasMessage::NodeClicked {
                        id: node.id,
                        shift: false,
                        ctrl: false,
                    }),
                );
            }
        }

        *state = Interaction::Panning {
            start: cursor_pos,
            original_pan: Vector::new(self.data.view.pan_x, self.data.view.pan_y),
        };
        (
            iced::event::Status::Captured,
            Some(CanvasMessage::BackgroundClicked),
        )
    }

    fn check_delete_target_click(&self, cursor_pos: Point) -> Option<CanvasMessage> {
        for node in &self.data.graph.nodes {
            if !self.data.graph.is_node_visible(node.id) {
                continue;
            }
            
            let screen_pos = self.data.view.world_to_screen(Point::new(node.x, node.y));
            let scaled_width = FocusNode::WIDTH * self.data.view.zoom;
            let cross_center = Point::new(screen_pos.x + scaled_width - 6.0, screen_pos.y + 6.0);
            if (cursor_pos.x - cross_center.x).hypot(cursor_pos.y - cross_center.y)
                < DELETE_NODE_CLICK_RADIUS
            {
                return Some(CanvasMessage::DeleteNodeClicked(node.id));
            }
        }

        for edge in &self.data.graph.edges {
            if !self.data.graph.is_node_visible(edge.parent_id)
                || !self.data.graph.is_node_visible(edge.child_id)
            {
                continue;
            }

            if let (Some(parent), Some(child)) = (
                self.data.graph.get_node(edge.parent_id),
                self.data.graph.get_node(edge.child_id),
            ) {
                let start_world = Point::new(parent.x + FocusNode::WIDTH / 2.0, parent.y + FocusNode::HEIGHT);
                let end_world = Point::new(child.x + FocusNode::WIDTH / 2.0, child.y);

                let start_screen = self.data.view.world_to_screen(start_world);
                let end_screen = self.data.view.world_to_screen(end_world);

                let mid_screen = Point::new((start_screen.x + end_screen.x) / 2.0, (start_screen.y + end_screen.y) / 2.0);

                if (cursor_pos.x - mid_screen.x).hypot(cursor_pos.y - mid_screen.y)
                    < DELETE_EDGE_CLICK_RADIUS
                {
                    return Some(CanvasMessage::DeleteEdgeClicked {
                        parent_id: edge.parent_id,
                        child_id: edge.child_id,
                    });
                }
            }
        }

        None
    }

    fn handle_cursor_moved(
        &self,
        state: &Interaction,
        cursor_pos: Point,
    ) -> (iced::event::Status, Option<CanvasMessage>) {
        let world_pos = self.data.view.screen_to_world(cursor_pos);
        match state {
            Interaction::Idle => (iced::event::Status::Ignored, None),
            Interaction::Panning {
                start,
                original_pan,
            } => {
                let new_view = Viewport {
                    pan_x: original_pan.x + (cursor_pos.x - start.x),
                    pan_y: original_pan.y + (cursor_pos.y - start.y),
                    zoom: self.data.view.zoom,
                };
                (
                    iced::event::Status::Captured,
                    Some(CanvasMessage::ViewChanged(new_view)),
                )
            }
            Interaction::DraggingNode { id, grab_offset } => {
                let new_x = world_pos.x - grab_offset.x;
                let new_y = world_pos.y - grab_offset.y;
                (
                    iced::event::Status::Captured,
                    Some(CanvasMessage::NodeMoved {
                        id: *id,
                        x: new_x,
                        y: new_y,
                    }),
                )
            }
        }
    }

    fn handle_wheel_scrolled(
        &self,
        delta: mouse::ScrollDelta,
        cursor_pos: Point,
    ) -> (iced::event::Status, Option<CanvasMessage>) {
        let zoom_delta = match delta {
            mouse::ScrollDelta::Lines { y, .. } => y * 0.1,
            mouse::ScrollDelta::Pixels { y, .. } => y * 0.002,
        };
        let new_zoom =
            (self.data.view.zoom * (1.0 + zoom_delta)).clamp(MIN_ZOOM_LEVEL, MAX_ZOOM_LEVEL);
        let world_before = self.data.view.screen_to_world(cursor_pos);
        let new_view = Viewport {
            zoom: new_zoom,
            pan_x: cursor_pos.x - world_before.x * new_zoom,
            pan_y: cursor_pos.y - world_before.y * new_zoom,
        };
        (
            iced::event::Status::Captured,
            Some(CanvasMessage::ViewChanged(new_view)),
        )
    }

    fn draw_visible_edges(&self, frame: &mut Frame, _bounds: Rectangle) {
        for edge in &self.data.graph.edges {
            if self.data.graph.is_node_visible(edge.parent_id)
                && self.data.graph.is_node_visible(edge.child_id)
            {
                if let (Some(parent), Some(child)) = (
                    self.data.graph.get_node(edge.parent_id),
                    self.data.graph.get_node(edge.child_id),
                ) {
                    draw_edge(
                        frame,
                        &self.data.view,
                        parent,
                        child,
                        self.data.is_shift_pressed,
                    );
                }
            }
        }
    }

    fn draw_visible_nodes(&self, frame: &mut Frame, bounds: Rectangle) {
        for node in &self.data.graph.nodes {
            if self.data.graph.is_node_visible(node.id) {
                let screen_pos = self.data.view.world_to_screen(Point::new(node.x, node.y));
                let scaled_w = FocusNode::WIDTH * self.data.view.zoom;
                let scaled_h = FocusNode::HEIGHT * self.data.view.zoom;

                // Viewport Culling
                if screen_pos.x + scaled_w < 0.0
                    || screen_pos.x > bounds.width
                    || screen_pos.y + scaled_h < 0.0
                    || screen_pos.y > bounds.height
                {
                    continue;
                }

                let is_selected = self.data.selected == Some(node.id);
                draw_node(
                    frame,
                    &self.data.view,
                    node,
                    is_selected,
                    self.data.is_shift_pressed,
                    self.data.is_ctrl_pressed,
                    self.data.image_cache,
                );
            }
        }
    }
}

fn hit_test_node(graph: &FocusGraph, world_pos: Point) -> Option<&FocusNode> {
    graph.nodes.iter().rev().find(|node| {
        world_pos.x >= node.x
            && world_pos.x <= node.x + FocusNode::WIDTH
            && world_pos.y >= node.y
            && world_pos.y <= node.y + FocusNode::HEIGHT
    })
}

fn draw_grid(frame: &mut Frame, view: &Viewport, bounds: Rectangle) {
    let scaled_spacing = GRID_SPACING * view.zoom;
    if scaled_spacing < MIN_GRID_PIXEL_SPACING {
        return;
    }

    let mut x = view.pan_x.rem_euclid(scaled_spacing);
    while x < bounds.width {
        frame.fill_rectangle(
            Point::new(x, 0.0),
            Size::new(1.0, bounds.height),
            COLOR_GRID_LINE,
        );
        x += scaled_spacing;
    }

    let mut y = view.pan_y.rem_euclid(scaled_spacing);
    while y < bounds.height {
        frame.fill_rectangle(
            Point::new(0.0, y),
            Size::new(bounds.width, 1.0),
            COLOR_GRID_LINE,
        );
        y += scaled_spacing;
    }
}

fn draw_node(
    frame: &mut Frame,
    view: &Viewport,
    node: &FocusNode,
    is_selected: bool,
    is_shift: bool,
    is_control: bool,
    image_cache: &HashMap<String, iced::widget::image::Handle>,
) {
    let screen_pos = view.world_to_screen(Point::new(node.x, node.y));
    let scaled_width = FocusNode::WIDTH * view.zoom;
    let icon_size = ICON_CONTAINER_SIZE * view.zoom;
    let icon_pos = Point::new(
        screen_pos.x + (scaled_width - icon_size) / 2.0,
        screen_pos.y + 4.0 * view.zoom,
    );

    draw_node_image_or_placeholder(frame, view, node, icon_pos, icon_size, image_cache);

    let banner_w = scaled_width * BANNER_WIDTH_SCALE;
    let banner_h = BANNER_HEIGHT * view.zoom;
    let banner_pos = Point::new(
        screen_pos.x + (scaled_width - banner_w) / 2.0,
        screen_pos.y + icon_size + 6.0 * view.zoom,
    );

    draw_node_banner(frame, view, banner_pos, banner_w, banner_h, is_selected);
    draw_node_title(
        frame,
        &node.title,
        banner_pos,
        banner_w,
        banner_h,
        view.zoom,
    );

    let bar_y = banner_pos.y + banner_h + 4.0 * view.zoom;
    draw_node_progress_bar(frame, view, banner_pos.x, bar_y, banner_w, node.status.progress);

    if is_shift {
        draw_delete_badge(frame, screen_pos, scaled_width);
    }

    if is_control {
        draw_collapse_badge(frame, screen_pos, node.is_collapsed);
    }
}

fn draw_node_image_or_placeholder(
    frame: &mut Frame,
    view: &Viewport,
    node: &FocusNode,
    icon_pos: Point,
    icon_size: f32,
    image_cache: &HashMap<String, iced::widget::image::Handle>,
) {
    if let Some(img_id) = &node.image_id {
        if let Some(handle) = image_cache.get(&img_id.to_string()) {
            frame.draw_image(
                Rectangle::new(icon_pos, Size::new(icon_size, icon_size)),
                Image::new(handle.clone()),
            );
            return;
        }
    }

    let center = Point::new(icon_pos.x + icon_size / 2.0, icon_pos.y + icon_size / 2.0);
    let placeholder_size = ICON_PLACEHOLDER_SIZE * view.zoom;
    frame.stroke_rectangle(
        Point::new(
            center.x - placeholder_size / 2.0,
            center.y - placeholder_size / 2.0,
        ),
        Size::new(placeholder_size, placeholder_size),
        canvas::Stroke {
            style: canvas::Style::Solid(COLOR_PLACEHOLDER_BORDER),
            width: (1.5 * view.zoom).max(0.5),
            ..Default::default()
        },
    );
}

fn draw_node_banner(
    frame: &mut Frame,
    view: &Viewport,
    banner_pos: Point,
    banner_w: f32,
    banner_h: f32,
    is_selected: bool,
) {
    frame.fill_rectangle(banner_pos, Size::new(banner_w, banner_h), COLOR_BANNER_BG);

    let border_color = if is_selected {
        COLOR_BANNER_BORDER_SELECTED
    } else {
        COLOR_BANNER_BORDER_NORMAL
    };

    frame.stroke_rectangle(
        banner_pos,
        Size::new(banner_w, banner_h),
        canvas::Stroke {
            style: canvas::Style::Solid(border_color),
            width: if is_selected {
                (2.0 * view.zoom).max(0.8)
            } else {
                (1.2 * view.zoom).max(0.5)
            },
            ..Default::default()
        },
    );

    let inset = 2.0 * view.zoom;
    frame.stroke_rectangle(
        Point::new(banner_pos.x + inset, banner_pos.y + inset),
        Size::new(banner_w - (inset * 2.0), banner_h - (inset * 2.0)),
        canvas::Stroke {
            style: canvas::Style::Solid(COLOR_BANNER_INSET_BORDER),
            width: (0.8 * view.zoom).max(0.4),
            ..Default::default()
        },
    );
}

fn draw_node_progress_bar(
    frame: &mut Frame,
    view: &Viewport,
    bar_x: f32,
    bar_y: f32,
    banner_w: f32,
    progress: f32,
) {
    let bar_h = PROGRESS_BAR_HEIGHT * view.zoom;

    frame.fill_rectangle(
        Point::new(bar_x, bar_y),
        Size::new(banner_w, bar_h),
        COLOR_PROGRESS_BG,
    );

    let progress_ratio = (progress / 100.0).clamp(0.0, 1.0);
    let fill_w = banner_w * progress_ratio;
    let bar_color = interpolate_color(COLOR_PROGRESS_EMPTY, COLOR_PROGRESS_FULL, progress_ratio);

    frame.fill_rectangle(
        Point::new(bar_x, bar_y),
        Size::new(fill_w, bar_h),
        bar_color,
    );

    frame.stroke_rectangle(
        Point::new(bar_x, bar_y),
        Size::new(banner_w, bar_h),
        canvas::Stroke {
            style: canvas::Style::Solid(COLOR_PROGRESS_BORDER),
            width: (0.8 * view.zoom).max(0.4),
            ..Default::default()
        },
    );
}

fn draw_delete_badge(frame: &mut Frame, screen_pos: Point, scaled_width: f32) {
    let cross_center = Point::new(screen_pos.x + scaled_width - 6.0, screen_pos.y + 6.0);
    let half_size = DELETE_BUTTON_SIZE / 2.0;

    frame.fill_rectangle(
        Point::new(cross_center.x - half_size, cross_center.y - half_size),
        Size::new(DELETE_BUTTON_SIZE, DELETE_BUTTON_SIZE),
        COLOR_DELETE_BTN,
    );

    frame.fill_text(canvas::Text {
        content: SYMBOL_DELETE.to_string(),
        position: cross_center,
        color: Color::WHITE,
        size: iced::Pixels(12.0),
        horizontal_alignment: iced::alignment::Horizontal::Center,
        vertical_alignment: iced::alignment::Vertical::Center,
        ..Default::default()
    });
}

fn draw_collapse_badge(
    frame: &mut Frame,
    screen_pos: Point,
    is_collapsed: bool,
) {
    let icon_center = Point::new(screen_pos.x + 10.0, screen_pos.y + 10.0);
    let half_size = COLLAPSE_BUTTON_SIZE / 2.0;
    let btn_color = if is_collapsed {
        COLOR_EXPAND_BTN
    } else {
        COLOR_COLLAPSE_BTN
    };

    frame.fill_rectangle(
        Point::new(icon_center.x - half_size, icon_center.y - half_size),
        Size::new(COLLAPSE_BUTTON_SIZE, COLLAPSE_BUTTON_SIZE),
        btn_color,
    );

    let symbol = if is_collapsed {
        SYMBOL_EXPAND
    } else {
        SYMBOL_COLLAPSE
    };

    frame.fill_text(canvas::Text {
        content: symbol.to_string(),
        position: icon_center,
        color: Color::WHITE,
        size: iced::Pixels(14.0),
        horizontal_alignment: iced::alignment::Horizontal::Center,
        vertical_alignment: iced::alignment::Vertical::Center,
        ..Default::default()
    });
}

fn draw_edge(
    frame: &mut Frame,
    view: &Viewport,
    parent: &FocusNode,
    child: &FocusNode,
    is_shift: bool,
) {
    let parent_center_x = parent.x + FocusNode::WIDTH / 2.0;
    let child_center_x = child.x + FocusNode::WIDTH / 2.0;

    let start_world = Point::new(parent_center_x, parent.y + FocusNode::HEIGHT);
    let end_world = Point::new(child_center_x, child.y);

    let start = view.world_to_screen(start_world);
    let end = view.world_to_screen(end_world);

    let stroke = canvas::Stroke {
        style: canvas::Style::Solid(COLOR_EDGE_LINE),
        width: (1.8 * view.zoom).max(0.6),
        ..Default::default()
    };

    let mid_y = (start.y + end.y) / 2.0;

    if (start.x - end.x).abs() < 2.0 {
        frame.stroke(&Path::line(start, end), stroke);
    } else {
        let mid1 = Point::new(start.x, mid_y);
        let mid2 = Point::new(end.x, mid_y);

        frame.stroke(
            &Path::new(|builder| {
                builder.move_to(start);
                builder.line_to(mid1);
                builder.line_to(mid2);
                builder.line_to(end);
            }),
            stroke,
        );
    }

    draw_arrowhead(frame, end);

    if is_shift {
        let mid_screen = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
        let badge_size = 14.0;
        let half_badge = badge_size / 2.0;

        frame.fill_rectangle(
            Point::new(mid_screen.x - half_badge, mid_screen.y - half_badge),
            Size::new(badge_size, badge_size),
            COLOR_DELETE_BTN,
        );

        frame.fill_text(canvas::Text {
            content: SYMBOL_DELETE.to_string(),
            position: mid_screen,
            color: Color::WHITE,
            size: iced::Pixels(10.0),
            horizontal_alignment: iced::alignment::Horizontal::Center,
            vertical_alignment: iced::alignment::Vertical::Center,
            ..Default::default()
        });
    }
}

fn draw_arrowhead(frame: &mut Frame, tip: Point) {
    let size = ARROW_HEAD_SIZE;
    let p1 = Point::new(tip.x - size, tip.y - size * 1.5);
    let p2 = Point::new(tip.x + size, tip.y - size * 1.5);

    frame.fill(
        &Path::new(|builder| {
            builder.move_to(tip);
            builder.line_to(p1);
            builder.line_to(p2);
            builder.close();
        }),
        COLOR_EDGE_LINE,
    );
}

fn interpolate_color(c1: Color, c2: Color, t: f32) -> Color {
    Color::from_rgb(
        c1.r + (c2.r - c1.r) * t,
        c1.g + (c2.g - c1.g) * t,
        c1.b + (c2.b - c1.b) * t,
    )
}

fn draw_node_title(
    frame: &mut Frame,
    title: &str,
    banner_pos: Point,
    banner_w: f32,
    banner_h: f32,
    zoom: f32,
) {
    if zoom <= MIN_VISIBLE_TITLE_ZOOM {
        return;
    }

    let available_width = banner_w - BANNER_PADDING;
    let base_font_size = BASE_FONT_SIZE_SCALE * zoom;

    let (font_size, content) = calculate_title_layout(title, available_width, base_font_size);
    let center_position = Point::new(banner_pos.x + banner_w / 2.0, banner_pos.y + banner_h / 2.0);

    frame.fill_text(canvas::Text {
        content,
        position: center_position,
        color: COLOR_TEXT_TITLE,
        size: iced::Pixels(font_size),
        horizontal_alignment: iced::alignment::Horizontal::Center,
        vertical_alignment: iced::alignment::Vertical::Center,
        ..Default::default()
    });
}

fn calculate_title_layout(title: &str, available_width: f32, base_font_size: f32) -> (f32, String) {
    let total_characters = title.chars().count();
    if total_characters == 0 {
        return (base_font_size.max(MIN_FONT_SIZE), String::new());
    }

    let max_chars_single_line = calculate_max_characters_per_line(available_width, base_font_size);
    if total_characters <= max_chars_single_line {
        return (base_font_size, title.to_string());
    }

    let base_wrapped = format_title_into_lines(title, available_width, base_font_size);
    if !base_wrapped.contains(ELLIPSIS_CHAR) {
        return (base_font_size, base_wrapped);
    }

    let total_available_width = available_width * (MAX_TITLE_LINES as f32);
    let required_font_size = total_available_width / (total_characters as f32 * CHAR_WIDTH_RATIO);
    let font_size = base_font_size.min(required_font_size).max(MIN_FONT_SIZE);
    let content = format_title_into_lines(title, available_width, font_size);

    (font_size, content)
}

fn calculate_max_characters_per_line(available_width: f32, font_size: f32) -> usize {
    let character_width = font_size * CHAR_WIDTH_RATIO;
    let min_characters = 3;
    ((available_width / character_width) as usize).max(min_characters)
}

fn format_title_into_lines(title: &str, available_width: f32, font_size: f32) -> String {
    let max_chars = calculate_max_characters_per_line(available_width, font_size);
    if title.chars().count() <= max_chars {
        return title.to_string();
    }

    let words: Vec<&str> = title.split_whitespace().collect();
    if words.len() > 1 {
        if let Some(wrapped) = wrap_words_into_two_lines(&words, max_chars) {
            return wrapped;
        }
    }

    split_characters_into_two_lines(title, max_chars)
}

fn wrap_words_into_two_lines(words: &[&str], max_chars_per_line: usize) -> Option<String> {
    let mut first_line = String::new();
    let mut split_index = 0;

    for (index, word) in words.iter().enumerate() {
        let proposed_length = if first_line.is_empty() {
            word.chars().count()
        } else {
            first_line.chars().count() + 1 + word.chars().count()
        };

        if proposed_length <= max_chars_per_line {
            if !first_line.is_empty() {
                first_line.push(' ');
            }
            first_line.push_str(word);
            split_index = index + 1;
        } else {
            break;
        }
    }

    if split_index > 0 && split_index < words.len() {
        let second_line_raw = words[split_index..].join(" ");
        let second_line = truncate_text_line(&second_line_raw, max_chars_per_line);
        Some(format!("{}\n{}", first_line, second_line))
    } else {
        None
    }
}

fn split_characters_into_two_lines(title: &str, max_chars_per_line: usize) -> String {
    let first_line: String = title.chars().take(max_chars_per_line).collect();
    let second_line_raw: String = title.chars().skip(max_chars_per_line).collect();
    let second_line = truncate_text_line(&second_line_raw, max_chars_per_line);
    format!("{}\n{}", first_line, second_line)
}

fn truncate_text_line(text: &str, max_characters: usize) -> String {
    if text.chars().count() <= max_characters {
        text.to_string()
    } else if max_characters <= 1 {
        text.chars().take(1).collect()
    } else {
        let mut truncated: String = text.chars().take(max_characters - 1).collect();
        truncated.push(ELLIPSIS_CHAR);
        truncated
    }
}
