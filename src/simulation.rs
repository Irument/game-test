use crate::rendering;
use crate::user_interface;
use either::Either;
use std::fs;
use std::path;
use std::sync;
use std::thread;
use winit::window;

pub struct Simulation<'window> {
    pub gpu_handle: rendering::GpuHandle<'window>,
    pub user_interface: user_interface::UserInterface<'window>,
    sprite_sheet: Vec<sync::Arc<image::RgbaImage>>,
    state: State,
    window: sync::Arc<window::Window>,
}
impl<'window> Simulation<'window> {
    pub fn new(
        gpu_handle: rendering::GpuHandle<'window>,
        window: sync::Arc<window::Window>,
    ) -> Self {
        let state = State::Debug(Debuger {});
        let sprite_sheet = Vec::new();
        Self {
            gpu_handle: gpu_handle.clone(),
            user_interface: user_interface::UserInterface::new(gpu_handle.clone()),
            sprite_sheet,
            state,
            window,
        }
    }

    pub fn update(&mut self) {
        let window = self.window.clone();
        self.process_user_interface();
        // self.process_user_interface();
    }
    fn process_user_interface(&mut self) {
        match self.state {
            State::Debug(ref debuger) => self.user_interface.update(debuger.user_interface()),
            State::InitStartup => {
                self.state = State::InitLoading(InitLoading::new());
                return;
            }
            State::InitLoading(ref init_loading) => {
                self.user_interface.update(init_loading.user_interface())
            }
            State::InitError(ref error) => todo!(),
        };
    }
}

pub enum State {
    Debug(Debuger),
    InitStartup,
    InitLoading(InitLoading),
    InitError(anyhow::Error),
}

pub struct Debuger {}

impl Debuger {
    fn user_interface(&self) -> impl FnMut(&egui::Context) {
        |context| {
            egui::CentralPanel::default().show(context, |user_interface: &mut egui::Ui| {
                user_interface.add(egui::Label::new("testing"))
            });
        }
    }
}
pub struct InitLoading {
    loading_thread: thread::JoinHandle<anyhow::Result<Vec<sync::Arc<image::RgbaImage>>>>,
    progress: sync::Arc<sync::atomic::AtomicU32>,
    total_work: u32,
}

impl InitLoading {
    pub fn new() -> Self {
        let mut source = std::env::current_exe().unwrap();
        source.pop();

        let models_to_load = fs::read_dir(source)
            .unwrap()
            .map(|entry| match entry {
                Ok(entry) => entry.path(),
                Err(error) => todo!(),
            })
            .filter(|entry| {
                entry
                    .extension()
                    .is_some_and(|extension| extension == "fbx")
            })
            .collect::<Vec<_>>();

        let progress = sync::Arc::new(sync::atomic::AtomicU32::new(0));
        let progress_clone = sync::Arc::clone(&progress);
        let total_work = models_to_load.len() as u32;

        let loading_thread =
            thread::spawn(move || Self::generate_sprite_sheet(progress_clone, models_to_load));

        Self {
            loading_thread,
            progress,
            total_work,
        }
    }
    fn generate_sprite_sheet(
        progress: sync::Arc<sync::atomic::AtomicU32>,
        model_paths: impl IntoIterator<Item = impl AsRef<path::Path>>,
    ) -> anyhow::Result<Vec<sync::Arc<image::RgbaImage>>> {
        let sprite_sheet = Vec::new();
        let importer = asset_importer::Importer::new();

        for model_path in model_paths {
            let model = importer.import_file(model_path)?;

            todo!();
            progress.fetch_add(1, sync::atomic::Ordering::AcqRel);
        }

        Ok(sprite_sheet)
    }
    fn user_interface(&self) -> impl FnMut(&egui::Context) {
        |context| {
            egui::CentralPanel::default().show(context, |user_interface: &mut egui::Ui| {
                user_interface.add(egui::ProgressBar::new(
                    self.progress.load(sync::atomic::Ordering::Acquire) as f32
                        / self.total_work as f32,
                ))
            });
        }
    }
}
