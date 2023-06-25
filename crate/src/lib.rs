use std::{
    sync::{Arc, Mutex},
    task::Context,
};

use cfg_if::cfg_if;
use kira::sound::static_sound::{PlaybackState, StaticSoundData, StaticSoundSettings};
use resources::load_bytes;
use std::future::Future;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

mod app;
mod camera;
mod input;
mod light;
mod model;
mod resources;
mod texture;

use app::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

// Async function to load resources in the background while the
// window is running. It was a bit of an ordeal to get that working...
async fn load_resources(app: Arc<Mutex<App>>) -> anyhow::Result<()> {
    log::info!("Loading resources...");
    let (device, queue) = {
        let app = app.lock().unwrap();
        (app.device.clone(), app.queue.clone())
    };

    let rei_model = model::Model::load(
        device.as_ref(),
        queue.as_ref(),
        "assets/rei/rei.obj",
        Some(&texture::Texture::texture_bind_group_layout(
            device.as_ref(),
        )),
    )
    .await?;

    let light_model =
        model::Model::load(device.as_ref(), queue.as_ref(), "assets/ike.obj", None).await?;

    let song = StaticSoundData::from_cursor(
        std::io::Cursor::new(load_bytes("assets/komm-susser-tod.ogg").await?),
        StaticSoundSettings::default(),
    )?;

    {
        let mut app = app.lock().unwrap();
        app.rei_model = Some(rei_model);
        app.light_model = Some(light_model);
        app.song = Some(song);

        app.state = State::Playing;
    }

    log::info!("Resources loaded!");

    Ok(())
}

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

    // Set the width and height of the window
    // on web this is going to have to be the dimensions of the page
    // so we need some web-specific code
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

    let app = App::new(window).await.unwrap();

    // On the web, we need to add an event listener to resize the window when the
    // page is resized. This isn't in sync with the regular window events, so
    // we need to wrap the app in a mutex.
    // TODO: make the mutex control less data so we dont have to interrupt so much stuff
    // every time the page is resized
    let app = Arc::new(Mutex::new(app));

    #[cfg(target_arch = "wasm32")]
    {
        let app = app.clone();
        let resize_closure = Closure::<dyn FnMut(_)>::new(move |_event: web_sys::UiEvent| {
            let width = web_sys::window()
                .and_then(|win| win.inner_width().ok())
                .and_then(|wid| wid.as_f64())
                .unwrap() as u32;

            let height = web_sys::window()
                .and_then(|win| win.inner_height().ok())
                .and_then(|hei| hei.as_f64())
                .unwrap() as u32;

            app.lock().unwrap().resize(PhysicalSize::new(width, height));
        });

        web_sys::window()
            .unwrap()
            .add_event_listener_with_callback("resize", resize_closure.as_ref().unchecked_ref())
            .expect("couldn't add event listener");

        resize_closure.forget();
    }

    let mut loaded = false;
    let mut load_result = Box::pin({
        let app = app.clone();
        load_resources(app)
    });

    event_loop.run(move |event, _, control_flow| {
        let mut app = app.lock().unwrap();

        if loaded {
            if let Some(handle) = app.song_handle_mut() {
                if handle.state() != PlaybackState::Playing {
                    log::info!("Resuming music");
                    handle.resume(Default::default()).unwrap();
                }
            } else {
                log::info!("Playing music");
                app.play_music();
                app.song_handle_mut()
                    .unwrap()
                    .pause(Default::default())
                    .unwrap();
                app.song_handle_mut()
                    .unwrap()
                    .resume(Default::default())
                    .unwrap();
            }
        }

        app.egui_platform.handle_event(&event);

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

        drop(app);

        // Perhaps I owe a bit of explanation to whoever's reading this.
        // This code is awful, and it's the fault of rust being special.
        // Rust could have a very nice async ecosystem but unfortunately, winit
        // needs to take control of the entire thread just to run its even loop.
        // This means winit can't be easily integrated with an async runtime like tokio,
        // and if you spawn a task to be completed while the window runs (for example,
        // i dunno, loading resources while a loading screen is displayed), the task
        // will never complete as winit is hogging all the resources for itself.
        // As a result, I've had to implement my own basic future executor to load
        // resources. This is awful and possibly a good sign that someone needs
        // to integrate async into winit. Apparently someone tried but they gave up
        // 4 years ago.
        //
        // Update: 1 day after i got this problem, a crate called "async-winit" was
        // announced. :shrug:
        if !loaded {
            let waker = futures::task::noop_waker();
            let mut cx = Context::from_waker(&waker);
            match (&mut load_result).as_mut().poll(&mut cx) {
                std::task::Poll::Ready(result) => {
                    result.unwrap();
                    loaded = true;
                }

                std::task::Poll::Pending => {}
            }
        }
    });
}
