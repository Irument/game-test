use std::sync;
use winit::window;

pub mod rendering;
pub mod simulation;
pub mod sprite;
pub mod user_interface;

pub fn start() -> Result<(), anyhow::Error> {
    let mut app = App::new();
    let event_loop = winit::event_loop::EventLoop::builder().build()?;

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    event_loop.run_app(&mut app)?;
    Ok(())
}

pub struct App<'window> {
    simulation: Option<crate::simulation::Simulation<'window>>,
    last_update: std::time::Instant,
}

impl App<'_> {
    pub fn new() -> Self {
        Self {
            simulation: None,
            last_update: std::time::Instant::now(),
        }
    }
}

impl winit::application::ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.simulation.is_none() {
            let window = sync::Arc::new(
                event_loop
                    .create_window(window::WindowAttributes::default())
                    .unwrap(),
            );

            let renderer = crate::rendering::Gpu::new(window.clone()).unwrap();
            let simulation = crate::simulation::Simulation::new(renderer, window);
            self.simulation = Some(simulation);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent;
        if self.simulation.is_none() {
            return;
        }
        let simulation = self.simulation.as_mut().unwrap();
        match event {
            WindowEvent::RedrawRequested => {
                let mut gpu = simulation.gpu_handle.write().unwrap();
                gpu.submit_command_buffer();
            }
            WindowEvent::ActivationTokenDone { serial, token } => todo!(),
            WindowEvent::Resized(physical_size) => {
                let mut gpu = simulation.gpu_handle.write().unwrap();
                gpu.surface_config_mut().width = physical_size.width;
                gpu.surface_config_mut().height = physical_size.height;
                simulation.user_interface.user_interface_input.screen_rect =
                    Some(egui::Rect::from_min_size(
                        egui::pos2(0.0, 0.0),
                        egui::vec2(physical_size.width as f32, physical_size.height as f32),
                    ));
            }
            WindowEvent::Moved(physical_position) => (),
            WindowEvent::CloseRequested => todo!(),
            WindowEvent::Destroyed => todo!(),
            WindowEvent::DroppedFile(path_buf) => todo!(),
            WindowEvent::HoveredFile(path_buf) => todo!(),
            WindowEvent::HoveredFileCancelled => todo!(),
            WindowEvent::Focused(focused) => {
                simulation.user_interface.user_interface_input.focused = focused
            }
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => {
                if is_synthetic {
                    return;
                }
                let key = key_from_winit_key(&event.logical_key);
                let physical_key =
                    if let winit::keyboard::PhysicalKey::Code(keycode) = event.physical_key {
                        key_from_key_code(keycode)
                    } else {
                        None
                    };

                if let Some(key) = key {
                    simulation
                        .user_interface
                        .user_interface_input
                        .events
                        .push(egui::Event::Key {
                            key,
                            physical_key,
                            pressed: event.state.is_pressed(),
                            repeat: false,
                            modifiers: simulation.user_interface.user_interface_input.modifiers,
                        })
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                simulation.user_interface.user_interface_input.modifiers = egui::Modifiers {
                    alt: modifiers.state().alt_key(),
                    ctrl: modifiers.state().control_key(),
                    shift: modifiers.state().shift_key(),
                    mac_cmd: false,
                    command: modifiers.state().control_key(),
                }
            }
            WindowEvent::Ime(ime) => todo!(),
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                let position = egui::Pos2 {
                    x: position.x as f32,
                    y: position.y as f32,
                };
                simulation.user_interface.last_mouse_pos = position;
                simulation
                    .user_interface
                    .user_interface_input
                    .events
                    .push(egui::Event::PointerMoved(position))
            }
            WindowEvent::CursorEntered {
                device_id: _device_id,
            } => (),
            WindowEvent::CursorLeft {
                device_id: _device_id,
            } => simulation
                .user_interface
                .user_interface_input
                .events
                .push(egui::Event::PointerGone),
            WindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
            } => todo!(),
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => simulation.user_interface.user_interface_input.events.push(
                egui::Event::PointerButton {
                    pos: simulation.user_interface.last_mouse_pos,
                    button: match button {
                        winit::event::MouseButton::Left => egui::PointerButton::Primary,
                        winit::event::MouseButton::Right => egui::PointerButton::Secondary,
                        winit::event::MouseButton::Middle => egui::PointerButton::Middle,
                        winit::event::MouseButton::Back => egui::PointerButton::Extra1,
                        winit::event::MouseButton::Forward => egui::PointerButton::Extra2,
                        winit::event::MouseButton::Other(_) => return,
                    },
                    pressed: state.is_pressed(),
                    modifiers: simulation.user_interface.user_interface_input.modifiers,
                },
            ),
            WindowEvent::PinchGesture {
                device_id,
                delta,
                phase,
            } => todo!(),
            WindowEvent::PanGesture {
                device_id,
                delta,
                phase,
            } => todo!(),
            WindowEvent::DoubleTapGesture { device_id } => todo!(),
            WindowEvent::RotationGesture {
                device_id,
                delta,
                phase,
            } => todo!(),
            WindowEvent::TouchpadPressure {
                device_id,
                pressure,
                stage,
            } => todo!(),
            WindowEvent::AxisMotion {
                device_id,
                axis,
                value,
            } => todo!(),
            WindowEvent::Touch(touch) => todo!(),
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer,
            } => (),
            WindowEvent::ThemeChanged(theme) => todo!(),
            WindowEvent::Occluded(_) => todo!(),
        }
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        use winit::event::StartCause;
        if std::time::Instant::now().duration_since(self.last_update)
            >= std::time::Duration::from_secs_f64(1.0 / 60.0)
        {
            self.last_update = std::time::Instant::now();
            if let Some(ref mut simulation) = self.simulation {
                simulation.update();
            }
        }
        match cause {
            StartCause::Init => (),
            StartCause::Poll => (),
            StartCause::ResumeTimeReached {
                start,
                requested_resume,
            } => todo!(),
            StartCause::WaitCancelled {
                start,
                requested_resume,
            } => todo!(),
        }
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: ()) {
        let _ = (event_loop, event);
    }

    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let _ = (event_loop, device_id, event);
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = event_loop;
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = event_loop;
    }

    fn exiting(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = event_loop;
    }

    fn memory_warning(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = event_loop;
    }
}

fn key_from_winit_key(key: &winit::keyboard::Key) -> Option<egui::Key> {
    match key {
        winit::keyboard::Key::Named(named_key) => key_from_named_key(*named_key),
        winit::keyboard::Key::Character(str) => egui::Key::from_name(str.as_str()),
        winit::keyboard::Key::Unidentified(_) | winit::keyboard::Key::Dead(_) => None,
    }
}

fn key_from_named_key(named_key: winit::keyboard::NamedKey) -> Option<egui::Key> {
    use egui::Key;
    use winit::keyboard::NamedKey;

    Some(match named_key {
        NamedKey::Enter => Key::Enter,
        NamedKey::Tab => Key::Tab,
        NamedKey::ArrowDown => Key::ArrowDown,
        NamedKey::ArrowLeft => Key::ArrowLeft,
        NamedKey::ArrowRight => Key::ArrowRight,
        NamedKey::ArrowUp => Key::ArrowUp,
        NamedKey::End => Key::End,
        NamedKey::Home => Key::Home,
        NamedKey::PageDown => Key::PageDown,
        NamedKey::PageUp => Key::PageUp,
        NamedKey::Backspace => Key::Backspace,
        NamedKey::Delete => Key::Delete,
        NamedKey::Insert => Key::Insert,
        NamedKey::Escape => Key::Escape,
        NamedKey::Cut => Key::Cut,
        NamedKey::Copy => Key::Copy,
        NamedKey::Paste => Key::Paste,

        NamedKey::Space => Key::Space,

        NamedKey::F1 => Key::F1,
        NamedKey::F2 => Key::F2,
        NamedKey::F3 => Key::F3,
        NamedKey::F4 => Key::F4,
        NamedKey::F5 => Key::F5,
        NamedKey::F6 => Key::F6,
        NamedKey::F7 => Key::F7,
        NamedKey::F8 => Key::F8,
        NamedKey::F9 => Key::F9,
        NamedKey::F10 => Key::F10,
        NamedKey::F11 => Key::F11,
        NamedKey::F12 => Key::F12,
        NamedKey::F13 => Key::F13,
        NamedKey::F14 => Key::F14,
        NamedKey::F15 => Key::F15,
        NamedKey::F16 => Key::F16,
        NamedKey::F17 => Key::F17,
        NamedKey::F18 => Key::F18,
        NamedKey::F19 => Key::F19,
        NamedKey::F20 => Key::F20,
        NamedKey::F21 => Key::F21,
        NamedKey::F22 => Key::F22,
        NamedKey::F23 => Key::F23,
        NamedKey::F24 => Key::F24,
        NamedKey::F25 => Key::F25,
        NamedKey::F26 => Key::F26,
        NamedKey::F27 => Key::F27,
        NamedKey::F28 => Key::F28,
        NamedKey::F29 => Key::F29,
        NamedKey::F30 => Key::F30,
        NamedKey::F31 => Key::F31,
        NamedKey::F32 => Key::F32,
        NamedKey::F33 => Key::F33,
        NamedKey::F34 => Key::F34,
        NamedKey::F35 => Key::F35,

        NamedKey::BrowserBack => Key::BrowserBack,
        _ => {
            return None;
        }
    })
}

fn key_from_key_code(key: winit::keyboard::KeyCode) -> Option<egui::Key> {
    use egui::Key;
    use winit::keyboard::KeyCode;

    Some(match key {
        KeyCode::ArrowDown => Key::ArrowDown,
        KeyCode::ArrowLeft => Key::ArrowLeft,
        KeyCode::ArrowRight => Key::ArrowRight,
        KeyCode::ArrowUp => Key::ArrowUp,

        KeyCode::Escape => Key::Escape,
        KeyCode::Tab => Key::Tab,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Enter | KeyCode::NumpadEnter => Key::Enter,

        KeyCode::Insert => Key::Insert,
        KeyCode::Delete => Key::Delete,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,

        // Punctuation
        KeyCode::Space => Key::Space,
        KeyCode::Comma => Key::Comma,
        KeyCode::Period => Key::Period,
        // KeyCode::Colon => Key::Colon, // NOTE: there is no physical colon key on an american keyboard
        KeyCode::Semicolon => Key::Semicolon,
        KeyCode::Backslash => Key::Backslash,
        KeyCode::Slash | KeyCode::NumpadDivide => Key::Slash,
        KeyCode::BracketLeft => Key::OpenBracket,
        KeyCode::BracketRight => Key::CloseBracket,
        KeyCode::Backquote => Key::Backtick,
        KeyCode::Quote => Key::Quote,

        KeyCode::Cut => Key::Cut,
        KeyCode::Copy => Key::Copy,
        KeyCode::Paste => Key::Paste,
        KeyCode::Minus | KeyCode::NumpadSubtract => Key::Minus,
        KeyCode::NumpadAdd => Key::Plus,
        KeyCode::Equal => Key::Equals,

        KeyCode::Digit0 | KeyCode::Numpad0 => Key::Num0,
        KeyCode::Digit1 | KeyCode::Numpad1 => Key::Num1,
        KeyCode::Digit2 | KeyCode::Numpad2 => Key::Num2,
        KeyCode::Digit3 | KeyCode::Numpad3 => Key::Num3,
        KeyCode::Digit4 | KeyCode::Numpad4 => Key::Num4,
        KeyCode::Digit5 | KeyCode::Numpad5 => Key::Num5,
        KeyCode::Digit6 | KeyCode::Numpad6 => Key::Num6,
        KeyCode::Digit7 | KeyCode::Numpad7 => Key::Num7,
        KeyCode::Digit8 | KeyCode::Numpad8 => Key::Num8,
        KeyCode::Digit9 | KeyCode::Numpad9 => Key::Num9,

        KeyCode::KeyA => Key::A,
        KeyCode::KeyB => Key::B,
        KeyCode::KeyC => Key::C,
        KeyCode::KeyD => Key::D,
        KeyCode::KeyE => Key::E,
        KeyCode::KeyF => Key::F,
        KeyCode::KeyG => Key::G,
        KeyCode::KeyH => Key::H,
        KeyCode::KeyI => Key::I,
        KeyCode::KeyJ => Key::J,
        KeyCode::KeyK => Key::K,
        KeyCode::KeyL => Key::L,
        KeyCode::KeyM => Key::M,
        KeyCode::KeyN => Key::N,
        KeyCode::KeyO => Key::O,
        KeyCode::KeyP => Key::P,
        KeyCode::KeyQ => Key::Q,
        KeyCode::KeyR => Key::R,
        KeyCode::KeyS => Key::S,
        KeyCode::KeyT => Key::T,
        KeyCode::KeyU => Key::U,
        KeyCode::KeyV => Key::V,
        KeyCode::KeyW => Key::W,
        KeyCode::KeyX => Key::X,
        KeyCode::KeyY => Key::Y,
        KeyCode::KeyZ => Key::Z,

        KeyCode::F1 => Key::F1,
        KeyCode::F2 => Key::F2,
        KeyCode::F3 => Key::F3,
        KeyCode::F4 => Key::F4,
        KeyCode::F5 => Key::F5,
        KeyCode::F6 => Key::F6,
        KeyCode::F7 => Key::F7,
        KeyCode::F8 => Key::F8,
        KeyCode::F9 => Key::F9,
        KeyCode::F10 => Key::F10,
        KeyCode::F11 => Key::F11,
        KeyCode::F12 => Key::F12,
        KeyCode::F13 => Key::F13,
        KeyCode::F14 => Key::F14,
        KeyCode::F15 => Key::F15,
        KeyCode::F16 => Key::F16,
        KeyCode::F17 => Key::F17,
        KeyCode::F18 => Key::F18,
        KeyCode::F19 => Key::F19,
        KeyCode::F20 => Key::F20,
        KeyCode::F21 => Key::F21,
        KeyCode::F22 => Key::F22,
        KeyCode::F23 => Key::F23,
        KeyCode::F24 => Key::F24,
        KeyCode::F25 => Key::F25,
        KeyCode::F26 => Key::F26,
        KeyCode::F27 => Key::F27,
        KeyCode::F28 => Key::F28,
        KeyCode::F29 => Key::F29,
        KeyCode::F30 => Key::F30,
        KeyCode::F31 => Key::F31,
        KeyCode::F32 => Key::F32,
        KeyCode::F33 => Key::F33,
        KeyCode::F34 => Key::F34,
        KeyCode::F35 => Key::F35,

        _ => {
            return None;
        }
    })
}
