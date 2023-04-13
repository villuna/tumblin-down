use std::collections::HashSet;

use winit::event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};

// A very basic input system. Why did I write it myself?
// because it's more work to figure out someone else's implementation.
pub struct KeyboardWatcher {
    pressed: HashSet<VirtualKeyCode>,
}

impl KeyboardWatcher {
    pub fn new() -> Self {
        Self {
            pressed: HashSet::new(),
        }
    }

    pub fn process_input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                if *state == ElementState::Pressed {
                    self.pressed.insert(*keycode);
                } else {
                    self.pressed.remove(keycode);
                }
            }

            _ => {}
        }
    }

    pub fn pressed(&self, keycode: VirtualKeyCode) -> bool {
        self.pressed.contains(&keycode)
    }
}
