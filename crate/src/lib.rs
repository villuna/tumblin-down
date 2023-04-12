// For now...
#![allow(unused)]

use std::sync::{Arc, Mutex};

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
mod input;

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

    cfg_if! {
        if #[cfg(target_arch="wasm32")] {
            let width = web_sys::window()
                .and_then(|win| win.inner_width().ok())
                .and_then(|wid| wid.as_f64())
                .unwrap() as u32;

            let height = web_sys::window()
                .and_then(|win| win.inner_height().ok())
                .and_then(|hei| hei.as_f64())
                .unwrap() as u32;

        } else {
            let width = WIDTH;
            let height = HEIGHT;
        }
    }

    // Instantiate the window
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(width, height))
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
                canvas.set_id("render-canvas");
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document.");
    }

    let mut app = App::new(window).await.unwrap();
    app.play_music();

    #[cfg(target_arch = "wasm32")]
    let app = Arc::new(Mutex::new(app));

    #[cfg(target_arch = "wasm32")]
    {
        let app = app.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |_event: web_sys::UiEvent| {
            let width = web_sys::window()
                .and_then(|win| win.inner_width().ok())
                .and_then(|wid| wid.as_f64())
                .unwrap() as u32;

            let height = web_sys::window()
                .and_then(|win| win.inner_height().ok())
                .and_then(|hei| hei.as_f64())
                .unwrap() as u32;

            app.lock().unwrap().resize(PhysicalSize::new(width, height));

            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|document| {
                    let canvas: web_sys::HtmlCanvasElement = document.get_element_by_id("render-canvas")?.unchecked_into();
                    log::info!("set canvas size to ({width}, {height})");
                    canvas.set_width(width);
                    canvas.set_height(height);
                    canvas.style().set_property("width", &format!("{width}px")).ok()?;
                    canvas.style().set_property("height", &format!("{height}px")).ok()?;
                    Some(())
                }).unwrap();
        });
        web_sys::window().unwrap()
            .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref()).expect("couldn't add event listener");

        closure.forget();
    }

    event_loop.run(move |event, _, control_flow| {
        #[cfg(target_arch = "wasm32")]
        let mut app = app.lock().unwrap();

        match event {
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

                    Err(wgpu::SurfaceError::Lost) => {
                        let size = *app.size();
                        app.resize(size);
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => control_flow.set_exit(),
                    Err(e) => log::error!("{e:?}"),
                }
            }

            Event::MainEventsCleared => app.window().request_redraw(),

            _ => {}
        }
    });
}
