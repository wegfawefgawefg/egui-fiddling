use raylib::prelude::*;
use std::collections::HashMap;

pub const FRAMES_PER_SECOND: u32 = 60;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    Square,
    Circle,
    Triangle,
}

/// Represents a single object in our scene tree.
#[derive(Debug, Clone)]
pub struct SceneObject {
    pub id: u32,
    pub text: String,
    pub shape: Shape,
    pub color: Color,
    pub rotation_speed: f32,
    pub current_rotation: f32,
    pub children: Vec<SceneObject>,
    pub text_buffer: String,
}

impl SceneObject {
    fn new(id: u32, text: &str, shape: Shape, color: Color) -> Self {
        Self {
            id,
            text: text.to_string(),
            shape,
            color,
            rotation_speed: 20.0,
            current_rotation: 0.0,
            children: Vec::new(),
            text_buffer: text.to_string(),
        }
    }
}

/// A request to modify the scene tree, processed at the end of the frame.
#[derive(Debug, Clone)]
pub enum EditorRequest {
    AddChild { parent_id: u32 },
    DeleteNode { node_id: u32 },
}

/// The main state for the application.
pub struct State {
    pub time_since_last_update: f32,
    pub scene_objects: Vec<SceneObject>,
    pub camera: Camera2D,
    next_id: u32,
    pub active_settings_id: Option<u32>,
    pub requests: Vec<EditorRequest>,
    // The ID of the text box that is currently active.
    pub active_textbox_id: Option<u32>,
}

impl State {
    pub fn new() -> Self {
        let mut state = Self {
            time_since_last_update: 0.0,
            scene_objects: Vec::new(),
            camera: Camera2D {
                target: Vector2::new(400.0, 450.0),
                offset: Vector2::new(1280.0 / 2.0, 720.0 / 2.0),
                rotation: 0.0,
                zoom: 1.0,
            },
            next_id: 0,
            active_settings_id: None,
            requests: Vec::new(),
            active_textbox_id: None,
        };

        // --- Create a sample scene tree ---
        let mut root = SceneObject::new(state.new_id(), "Root", Shape::Square, Color::RED);
        let mut child1 = SceneObject::new(state.new_id(), "Data", Shape::Circle, Color::BLUE);
        let mut child2 = SceneObject::new(state.new_id(), "Render", Shape::Triangle, Color::GREEN);
        let grandchild1a = SceneObject::new(state.new_id(), "Mesh", Shape::Square, Color::YELLOW);
        let grandchild1b =
            SceneObject::new(state.new_id(), "Texture", Shape::Triangle, Color::ORANGE);
        let grandchild2a = SceneObject::new(state.new_id(), "Shader", Shape::Circle, Color::PURPLE);

        child1.children.push(grandchild1a);
        child1.children.push(grandchild1b);
        child2.children.push(grandchild2a);
        root.children.push(child1);
        root.children.push(child2);
        state.scene_objects.push(root);

        state
    }

    pub fn new_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }
}

/// Handles user input for camera movement and object selection.
pub fn process_events_and_input(rl: &mut RaylibHandle, state: &mut State) {
    if state.active_textbox_id.is_some() {
        return;
    }

    const ZOOM_INCREMENT: f32 = 0.125;
    const MIN_ZOOM: f32 = 0.1;
    const MAX_ZOOM: f32 = 2.0;

    let wheel_move = rl.get_mouse_wheel_move();
    if wheel_move != 0.0 {
        let wheel_direction = if wheel_move > 0.0 { 1.0 } else { -1.0 };
        state.camera.zoom += wheel_direction * ZOOM_INCREMENT;
    }
    if rl.is_key_down(KeyboardKey::KEY_EQUAL) {
        state.camera.zoom += 1.0 * rl.get_frame_time();
    }
    if rl.is_key_down(KeyboardKey::KEY_MINUS) {
        state.camera.zoom -= 1.0 * rl.get_frame_time();
    }
    state.camera.zoom = state.camera.zoom.clamp(MIN_ZOOM, MAX_ZOOM);

    if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT) {
        let delta = rl.get_mouse_delta() / state.camera.zoom;
        state.camera.target -= delta;
    }

    // --- Click to Select/Edit ---
    if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
        let mouse_pos = rl.get_mouse_position();

        if state.active_settings_id.is_some() && mouse_pos.x < 420.0 {
            return;
        }

        let world_mouse_pos = screen_to_world(mouse_pos, &state.camera);

        let mut layout_positions = HashMap::new();
        let mut start_y = 100.0;
        for obj in &state.scene_objects {
            layout_recursive(obj, 200.0, start_y, &mut start_y, &mut layout_positions);
        }

        let mut clicked_id = None;
        for obj in &state.scene_objects {
            if let Some(id) = find_clicked_object(obj, world_mouse_pos, &layout_positions) {
                clicked_id = Some(id);
                break;
            }
        }

        if let Some(id) = clicked_id {
            state.active_settings_id = Some(id);
            if let Some(obj_pos) = layout_positions.get(&id) {
                let screen_width = rl.get_screen_width() as f32;
                let viewport_center_x = (screen_width / 2.0 + screen_width) / 2.0;
                let offset_x = (viewport_center_x - state.camera.offset.x) / state.camera.zoom;
                state.camera.target = Vector2::new(obj_pos.x - offset_x, obj_pos.y);
            }
        }
    }
}

/// Updates the state of all objects in the scene.
pub fn step(state: &mut State, dt: f32) {
    if state.active_textbox_id.is_some() {
        return;
    }

    for obj in &mut state.scene_objects {
        update_object_recursively(obj, dt);
    }
    process_editor_requests(state);
}

fn update_object_recursively(obj: &mut SceneObject, dt: f32) {
    obj.current_rotation += obj.rotation_speed * dt;
    for child in &mut obj.children {
        update_object_recursively(child, dt);
    }
}

/// Main drawing function.
pub fn draw(state: &mut State, d: &mut RaylibDrawHandle) {
    let mut layout_positions = HashMap::new();
    let mut start_y = 100.0;
    for root_obj in &state.scene_objects {
        layout_recursive(
            root_obj,
            200.0,
            start_y,
            &mut start_y,
            &mut layout_positions,
        );
    }

    // --- Draw all world objects ---
    {
        let mut d2d = d.begin_mode2D(state.camera);
        for (id, pos) in &layout_positions {
            if let Some(obj) = find_object_by_id_mut(&mut state.scene_objects, *id) {
                draw_world_object(&mut d2d, obj, *pos, &layout_positions);
            }
        }
    }

    // --- Draw the ONE active settings panel on top of everything else ---
    let State {
        scene_objects,
        active_settings_id,
        requests,
        active_textbox_id,
        ..
    } = state;

    if let Some(id) = *active_settings_id {
        if let Some(obj) = find_object_by_id_mut(scene_objects, id) {
            draw_settings_panel(d, active_settings_id, requests, active_textbox_id, obj);
        }
    }

    d.draw_text(
        "Click a shape to edit. Right Mouse to Pan, Scroll Wheel to Zoom.",
        10,
        10,
        20,
        Color::WHITE,
    );
}

/// The layout algorithm. Calculates positions for a left-to-right tree.
fn layout_recursive(
    obj: &SceneObject,
    x: f32,
    y_start: f32,
    y_cursor: &mut f32,
    positions: &mut HashMap<u32, Vector2>,
) -> f32 {
    const X_SPACING: f32 = 250.0;
    const Y_SPACING: f32 = 120.0;
    let mut children_height = 0.0;
    let mut child_y_cursor = y_start;
    for child in &obj.children {
        children_height += layout_recursive(
            child,
            x + X_SPACING,
            child_y_cursor,
            &mut child_y_cursor,
            positions,
        );
    }
    let my_pos = if !obj.children.is_empty() {
        Vector2::new(x, y_start + (children_height / 2.0) - (Y_SPACING / 2.0))
    } else {
        Vector2::new(x, *y_cursor)
    };
    positions.insert(obj.id, my_pos);
    let my_height = if children_height > 0.0 {
        children_height
    } else {
        Y_SPACING
    };
    *y_cursor = y_start + my_height;
    my_height
}

/// Draws just the world representation of an object (shape, lines, details).
fn draw_world_object(
    d_world: &mut RaylibMode2D<RaylibDrawHandle>,
    obj: &SceneObject,
    world_pos: Vector2,
    positions: &HashMap<u32, Vector2>,
) {
    for child in &obj.children {
        if let Some(child_pos) = positions.get(&child.id) {
            d_world.draw_line_v(world_pos, *child_pos, Color::GRAY);
        }
    }

    let size = 40.0;
    match obj.shape {
        Shape::Square => d_world.draw_rectangle_pro(
            Rectangle::new(world_pos.x, world_pos.y, size, size),
            Vector2::new(size / 2.0, size / 2.0),
            obj.current_rotation,
            obj.color,
        ),
        Shape::Circle => d_world.draw_circle_v(world_pos, size / 2.0, obj.color),
        Shape::Triangle => {
            let angle_rad = obj.current_rotation.to_radians();
            let cos_a = angle_rad.cos();
            let sin_a = angle_rad.sin();
            let p1 = Vector2::new(0.0, -size / 2.0);
            let p2 = Vector2::new(-size / 2.0, size / 2.0);
            let p3 = Vector2::new(size / 2.0, size / 2.0);
            let rp1 =
                Vector2::new(p1.x * cos_a - p1.y * sin_a, p1.x * sin_a + p1.y * cos_a) + world_pos;
            let rp2 =
                Vector2::new(p2.x * cos_a - p2.y * sin_a, p2.x * sin_a + p2.y * cos_a) + world_pos;
            let rp3 =
                Vector2::new(p3.x * cos_a - p3.y * sin_a, p3.x * sin_a + p3.y * cos_a) + world_pos;
            d_world.draw_triangle(rp1, rp2, rp3, obj.color);
        }
    }

    let text_size = d_world
        .get_font_default()
        .measure_text(&obj.text, 20.0, 1.0);
    d_world.draw_text(
        &obj.text,
        (world_pos.x - text_size.x / 2.0) as i32,
        (world_pos.y + 30.0) as i32,
        20,
        Color::WHITE,
    );
}

/// Draws the main, interactive settings panel for the active node.
fn draw_settings_panel(
    d: &mut RaylibDrawHandle,
    active_settings_id: &mut Option<u32>,
    requests: &mut Vec<EditorRequest>,
    active_textbox_id: &mut Option<u32>,
    obj: &mut SceneObject,
) {
    let panel_width = 400.0;
    let padding = 20.0;
    let window_rect = Rectangle::new(
        padding,
        padding,
        panel_width,
        d.get_screen_height() as f32 - padding * 2.0,
    );

    if d.gui_window_box(window_rect, &format!("Settings: {}", obj.text)) {
        *active_settings_id = None;
    }

    let base_x = window_rect.x + 10.0;
    let mut current_y = window_rect.y + 40.0;

    d.gui_label(Rectangle::new(base_x, current_y, 100.0, 20.0), "Name:");
    current_y += 25.0;

    let textbox_bounds = Rectangle::new(base_x, current_y, window_rect.width - 20.0, 30.0);
    if gui_text_box_safe(d, textbox_bounds, &mut obj.text_buffer) {
        obj.text = obj.text_buffer.clone();
        *active_textbox_id = None;
    } else if check_collision_point_rec(d.get_mouse_position(), textbox_bounds)
        && d.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT)
    {
        *active_textbox_id = Some(obj.id);
    } else if *active_textbox_id == Some(obj.id)
        && d.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT)
    {
        obj.text = obj.text_buffer.clone();
        *active_textbox_id = None;
    }
    current_y += 40.0;

    d.gui_label(Rectangle::new(base_x, current_y, 100.0, 20.0), "Shape:");
    current_y += 25.0;
    let mut active_shape = obj.shape as i32;
    d.gui_toggle_group(
        Rectangle::new(base_x, current_y, 70.0, 25.0),
        "Square;Circle;Triangle",
        &mut active_shape,
    );
    obj.shape = match active_shape {
        0 => Shape::Square,
        1 => Shape::Circle,
        _ => Shape::Triangle,
    };
    current_y += 35.0;

    d.gui_label(
        Rectangle::new(base_x, current_y, 100.0, 20.0),
        "Rotation Speed:",
    );
    current_y += 25.0;
    d.gui_slider_bar(
        Rectangle::new(base_x, current_y, window_rect.width - 20.0, 20.0),
        "",
        &format!("{:.0}", obj.rotation_speed),
        &mut obj.rotation_speed,
        -180.0,
        180.0,
    );
    current_y += 30.0;

    d.gui_label(Rectangle::new(base_x, current_y, 100.0, 20.0), "Color:");
    current_y += 25.0;
    obj.color = d.gui_color_picker(
        Rectangle::new(base_x, current_y, window_rect.width - 20.0, 150.0),
        "",
        obj.color,
    );
    current_y += 160.0;

    if d.gui_button(Rectangle::new(base_x, current_y, 150.0, 30.0), "Add Child") {
        requests.push(EditorRequest::AddChild { parent_id: obj.id });
    }
    if d.gui_button(
        Rectangle::new(
            window_rect.x + window_rect.width - 160.0,
            current_y,
            150.0,
            30.0,
        ),
        "Delete Node",
    ) {
        requests.push(EditorRequest::DeleteNode { node_id: obj.id });
        *active_settings_id = None;
    }
}

/// Processes the request queue to add or delete nodes.
fn process_editor_requests(state: &mut State) {
    let requests = state.requests.drain(..).collect::<Vec<_>>();
    for req in requests {
        match req {
            EditorRequest::AddChild { parent_id } => {
                let new_id = state.new_id();
                if let Some(parent) = find_object_by_id_mut(&mut state.scene_objects, parent_id) {
                    let new_node =
                        SceneObject::new(new_id, "New Node", Shape::Square, Color::WHITE);
                    parent.children.push(new_node);
                }
            }
            EditorRequest::DeleteNode { node_id } => {
                find_and_delete_node(&mut state.scene_objects, node_id);
            }
        }
    }
}

// --- Helper Functions ---

/// A 100% safe wrapper around `gui_text_box` that handles buffer management correctly.
/// Returns `true` if the user pressed Enter.
fn gui_text_box_safe(d: &mut RaylibDrawHandle, bounds: Rectangle, text: &mut String) -> bool {
    let mut result = false;

    // Check if the text box should be active.
    if check_collision_point_rec(d.get_mouse_position(), bounds) {
        // Process character input
        while let Some(key) = d.get_char_pressed() {
            text.push(key);
        }

        // Process backspace
        if d.is_key_pressed_repeat(KeyboardKey::KEY_BACKSPACE)
            || d.is_key_down(KeyboardKey::KEY_BACKSPACE)
        {
            text.pop();
        }

        // Check for commit
        if d.is_key_pressed(KeyboardKey::KEY_ENTER) {
            result = true;
        }
    }

    // We still call the original function to draw the box, but we handle the input ourselves.
    d.gui_text_box(bounds, text, true);
    result
}

fn check_collision_point_rec(point: Vector2, rec: Rectangle) -> bool {
    point.x >= rec.x
        && point.x < (rec.x + rec.width)
        && point.y >= rec.y
        && point.y < (rec.y + rec.height)
}

fn find_and_delete_node(objects: &mut Vec<SceneObject>, id_to_delete: u32) -> bool {
    if let Some(index) = objects.iter().position(|o| o.id == id_to_delete) {
        objects.remove(index);
        return true;
    }
    for obj in objects {
        if find_and_delete_node(&mut obj.children, id_to_delete) {
            return true;
        }
    }
    false
}

fn find_object_by_id_mut<'a>(
    objects: &'a mut [SceneObject],
    id: u32,
) -> Option<&'a mut SceneObject> {
    for obj in objects {
        if obj.id == id {
            return Some(obj);
        }
        if let Some(found) = find_object_by_id_mut(&mut obj.children, id) {
            return Some(found);
        }
    }
    None
}

fn find_clicked_object(
    obj: &SceneObject,
    world_pos: Vector2,
    positions: &HashMap<u32, Vector2>,
) -> Option<u32> {
    if let Some(obj_pos) = positions.get(&obj.id) {
        let dist = world_pos.distance_to(*obj_pos);
        if dist < 20.0 {
            // 20.0 is half the shape size
            return Some(obj.id);
        }
    }
    for child in &obj.children {
        if let Some(id) = find_clicked_object(child, world_pos, positions) {
            return Some(id);
        }
    }
    None
}

fn screen_to_world(screen_pos: Vector2, camera: &Camera2D) -> Vector2 {
    (screen_pos - camera.offset) / camera.zoom + camera.target
}
