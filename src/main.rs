use std::{fmt::Write, fs, path::PathBuf};

use clipboard::{ClipboardContext, ClipboardProvider};
use eframe::egui::*;
use serde::{Deserialize, Serialize};

fn main() {
    let prompt: Prompt = fs::read(Prompt::path())
        .ok()
        .and_then(|bytes| serde_yaml::from_slice(&bytes).ok())
        .unwrap_or_else(|| Prompt {
            text: String::new(),
            suffixes: vec![("realistic".into(), false)],
            algorithm: Algorithm::V3,
            aspect_w: 1,
            aspect_h: 1,
            stylize: DEFAULT_STYLIZE,
            use_seed: false,
            seed: 0,
            video: false,
            copy_on_change: true,
            copied_command: String::new(),
        });
    let options = eframe::NativeOptions {
        min_window_size: Some([600.0, 400.0].into()),
        initial_window_size: Some([600.0, 600.0].into()),
        ..Default::default()
    };
    eframe::run_native(
        "Midjourney Prompt",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(2.0);
            Box::new(prompt)
        }),
    );
}

#[derive(Serialize, Deserialize)]
struct Prompt {
    #[serde(skip)]
    text: String,
    suffixes: Vec<(String, bool)>,
    algorithm: Algorithm,
    aspect_w: u16,
    aspect_h: u16,
    stylize: u32,
    video: bool,
    copy_on_change: bool,
    use_seed: bool,
    seed: u32,
    #[serde(skip)]
    copied_command: String,
}

const DEFAULT_STYLIZE: u32 = 2500;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Algorithm {
    V3,
    Test,
    TestPhoto,
}

impl Algorithm {
    fn str(&self) -> &'static str {
        match self {
            Algorithm::V3 => "v3",
            Algorithm::Test => "test",
            Algorithm::TestPhoto => "testp",
        }
    }
}

impl Prompt {
    fn dir() -> PathBuf {
        dirs::data_local_dir().unwrap().join("midjourney_prompt")
    }
    fn path() -> PathBuf {
        Self::dir().join("promt.yaml")
    }
    #[allow(unused_must_use)]
    fn command(&self) -> String {
        let mut s = format!("/imagine prompt: {}", self.text.trim());
        for (suffix, enabled) in &self.suffixes {
            if *enabled && !suffix.trim().is_empty() {
                write!(&mut s, ", {}", suffix.trim());
            }
        }
        if self.stylize != DEFAULT_STYLIZE {
            write!(&mut s, " --stylize {}", self.stylize);
        }
        if [self.aspect_w, self.aspect_h] != [1, 1] {
            write!(&mut s, " --ar {}:{}", self.aspect_w, self.aspect_h);
        }
        if self.video {
            s.push_str(" --video");
        }
        if self.use_seed {
            write!(&mut s, " --sameseed {}", self.seed);
        }
        if self.algorithm != Algorithm::V3 {
            write!(&mut s, " --{}", self.algorithm.str());
        }
        s
    }
}

impl eframe::App for Prompt {
    fn on_close_event(&mut self) -> bool {
        let _ = fs::create_dir_all(Self::dir());
        let _ = fs::write(Self::path(), serde_yaml::to_string(self).unwrap());
        true
    }
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let old_command = self.command();
        CentralPanel::default().show(ctx, |ui| {
            // Settings
            CollapsingHeader::new("settings").show(ui, |ui| {
                Grid::new("settings").show(ui, |ui| {
                    let cot_hover_text = "copy command to clipboard when changed";
                    ui.label("copy on change").on_hover_text(cot_hover_text);
                    ui.checkbox(&mut self.copy_on_change, "")
                        .on_hover_text(cot_hover_text);
                    ui.end_row();
                });
            });
            ui.separator();
            ScrollArea::both()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    Grid::new(0).show(ui, |ui| {
                        // Prompt
                        ui.label("prompt");
                        TextEdit::multiline(&mut self.text)
                            .show(ui)
                            .response
                            .changed();
                        ui.end_row();

                        // Algorithm
                        ui.label("algorithm");
                        ui.horizontal(|ui| {
                            for algo in [Algorithm::V3, Algorithm::Test, Algorithm::TestPhoto] {
                                ui.selectable_value(&mut self.algorithm, algo, algo.str())
                                    .clicked();
                            }
                            ui.add_space(100.0);
                        });
                        ui.end_row();

                        // Aspect
                        ui.label("aspect");
                        ui.horizontal(|ui| {
                            DragValue::new(&mut self.aspect_w)
                                .clamp_range(1..=21)
                                .speed(0.1)
                                .ui(ui)
                                .changed();
                            ui.label(":");
                            DragValue::new(&mut self.aspect_h)
                                .clamp_range(1..=10)
                                .speed(0.1)
                                .ui(ui)
                                .changed();
                            ComboBox::from_id_source("aspect")
                                .selected_text("preset")
                                .width(60.0)
                                .show_ui(ui, |ui| {
                                    for [w, h] in [
                                        [1, 1],
                                        [1, 2],
                                        [1, 3],
                                        [2, 3],
                                        [3, 2],
                                        [3, 4],
                                        [4, 3],
                                        [16, 9],
                                        [21, 9],
                                    ] {
                                        if ui
                                            .selectable_label(
                                                [self.aspect_w, self.aspect_h] == [w, h],
                                                format!("{w}:{h}"),
                                            )
                                            .clicked()
                                        {
                                            self.aspect_w = w;
                                            self.aspect_h = h;
                                        }
                                    }
                                });
                        });
                        ui.end_row();

                        // Stylize
                        ui.label("stylize");
                        ui.horizontal(|ui| {
                            Slider::new(&mut self.stylize, 625..=60000)
                                .logarithmic(true)
                                .ui(ui);
                            if self.stylize != DEFAULT_STYLIZE && ui.button("reset").clicked() {
                                self.stylize = DEFAULT_STYLIZE;
                            }
                        });
                        ui.end_row();

                        // Seed
                        ui.label("seed");
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.use_seed, "");
                            if self.use_seed {
                                DragValue::new(&mut self.seed).ui(ui);
                            }
                        });
                        ui.end_row();

                        // Video
                        ui.label("video");
                        ui.checkbox(&mut self.video, "");
                        ui.end_row();

                        // Suffixes
                        ui.label("suffixes");
                        ui.vertical(|ui| {
                            let mut to_remove = None;
                            for i in 0..self.suffixes.len() {
                                let (suffix, enabled) = &mut self.suffixes[i];
                                ui.horizontal(|ui| {
                                    TextEdit::singleline(suffix)
                                        .desired_width(120.0)
                                        .show(ui)
                                        .response
                                        .changed();
                                    ui.checkbox(enabled, "");
                                    if ui.button("-").clicked() {
                                        to_remove = Some(i);
                                    }
                                });
                            }
                            if ui.button("+").clicked() {
                                self.suffixes.push(("".into(), true));
                            }
                            if let Some(i) = to_remove {
                                self.suffixes.remove(i);
                            }
                        });
                        ui.end_row();
                    });
                    // Command
                    ui.label("");
                    ui.horizontal_wrapped(|ui| {
                        ui.label(&self.copied_command);
                    });
                    let copy_to_clipboard = self.copy_on_change && self.command() != old_command
                        || !self.copy_on_change
                            && ui
                                .add_enabled(!self.text.trim().is_empty(), Button::new("copy"))
                                .clicked();
                    if copy_to_clipboard && !self.text.trim().is_empty() {
                        self.copied_command = match ClipboardContext::new()
                            .unwrap()
                            .set_contents(self.command())
                        {
                            Ok(()) => {
                                format!("copied command:\n{}", self.command())
                            }
                            Err(e) => format!("error copying command: {e}"),
                        };
                    }
                });
        });
    }
}
