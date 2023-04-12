// For now...
#![allow(unused)]

use cfg_if::cfg_if;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

mod app;
mod resources;
mod texture;
mod model;
mod camera;

use app::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    // Set up the logging system (wgpu only outputs its errors through logging)
    // The logging system will be different for web than for desktop
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            // i dont really know what this does
            // it just makes everything very very way more safer
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Couldn't initialise logger");
        } else {
            env_logger::init();
        }
    }

    // Instantiate the window
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT))
        .build(&event_loop)
        .unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        // On web we need to bind the window to the canvas
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|document| {
                let dst = document.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document.");
    }

    let mut app = App::new(window).await.unwrap();

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { window_id, event }
            if window_id == app.window().id() && !app.process_input(&event) =>
        {
            match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    control_flow.set_exit();
                }

                WindowEvent::Resized(size) => {
                    app.resize(size);
                }

                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    app.resize(*new_inner_size);
                }

                _ => {}
            }
        }

        Event::RedrawRequested(window_id) if window_id == app.window().id() => {
            app.update();

            match app.render() {
                Ok(_) => {}

                Err(wgpu::SurfaceError::Lost) => app.resize(*app.size()),
                Err(wgpu::SurfaceError::OutOfMemory) => control_flow.set_exit(),
                Err(e) => log::error!("{e:?}"),
            }
        }

        Event::MainEventsCleared => app.window().request_redraw(),

        _ => {}
    });
}
