use std::collections::HashSet;
use winit::keyboard::Key;

pub struct InputManager {
    pressed_keys: HashSet<Key>,
    just_pressed_keys: HashSet<Key>,
    pub mouse_dx: f64,
    pub mouse_dy: f64,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            just_pressed_keys: HashSet::new(),
            mouse_dx: 0.0,
            mouse_dy: 0.0,
        }
    }

    pub fn is_key_pressed(&self, key: &Key) -> bool {
        self.pressed_keys.contains(key)
    }

    pub fn is_key_just_pressed(&self, key: &Key) -> bool {
        self.just_pressed_keys.contains(key)
    }

    pub fn set_key(&mut self, key: Key, pressed: bool) {
        if pressed {
            if !self.pressed_keys.contains(&key) {
                self.just_pressed_keys.insert(key.clone());
            }
            self.pressed_keys.insert(key);
        } else {
            self.pressed_keys.remove(&key);
        }
    }

    pub fn clear_frame_state(&mut self) {
        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;
        self.just_pressed_keys.clear();
    }

    pub fn add_mouse_delta(&mut self, dx: f64, dy: f64) {
        self.mouse_dx += dx;
        self.mouse_dy += dy;
    }
}
