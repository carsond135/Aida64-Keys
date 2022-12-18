#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashSet;
use std::ops::Sub;

use aida64_keys_lib::{KeyEdition, License};
use chrono::{Date, Duration, TimeZone, Utc};
use clipboard::{ClipboardContext, ClipboardProvider};
use eframe::egui::{self, Layout};
use eframe::emath::Align;
use eframe::epaint::Vec2;
use egui_datepicker::DatePicker;
use strum::IntoEnumIterator;

struct NotePopup {
    text: String,
}

impl NotePopup {
    fn new(text: String) -> NotePopup {
        Self { text }
    }

    fn show(&self, ctx: &egui::Context) -> bool {
        let mut wants_close = false;

        egui::Window::new("note_window")
            .default_size(egui::Vec2 { x: 250.0, y: 50.0 })
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::default())
            .show(ctx, |ui| {
                let layout = Layout::top_down(Align::Center).with_cross_justify(true);
                ui.with_layout(layout, |ui| {
                    ui.label(&self.text);
                    ui.add_space(2.5);
                    if ui.button("OK").clicked() {
                        wants_close |= true;
                    }
                });
            });

        wants_close
    }
}

struct App {
    note: Option<NotePopup>,

    licenses: HashSet<String>,
    license_count: usize,

    license_edition: KeyEdition,
    license_seats: i32,
    license_purchase: Date<Utc>,
    license_expire: Date<Utc>,
    license_expire_never: bool,
    license_maintenance: Date<Utc>,

    selected_license: Option<usize>,

    clipboard_provider: ClipboardContext,
}

impl Default for App {
    fn default() -> Self {
        Self {
            note: None,

            licenses: HashSet::new(),
            license_count: 1,

            license_edition: KeyEdition::Extreme,
            license_seats: 1,
            license_purchase: Utc::today(),
            license_expire: Utc::today() + Duration::days(3658),
            license_expire_never: true,
            license_maintenance: Utc::today() + Duration::days(3658),

            selected_license: None,

            clipboard_provider: ClipboardProvider::new().expect("Failed to get clipboard provider"),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if let Some(note) = &self.note {
            note.show(ctx).then(|| self.note = None);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.note.is_some() {
                ui.set_enabled(false);
            }

            ui.columns(2, |columns| {
                columns[0].group(|ui| {
                    let available_size = ui.available_size();

                    ui.set_max_size(available_size);
                    ui.set_min_size(available_size);

                    ui.columns(2, |columns| {
                        columns[0].vertical_centered_justified(|ui| {
                            if ui.button("Generate").clicked() {
                                self.licenses.clear();
                                self.selected_license = None;

                                while self.licenses.len() < self.license_count {
                                    let mut license = License::new(self.license_edition)
                                        .with_seats(self.license_seats)
                                        .with_purchase_date(self.license_purchase)
                                        .with_maintenance_expiry(
                                            self.license_maintenance
                                                .sub(self.license_purchase)

                                        );

                                    if !self.license_expire_never {
                                        license = license.with_license_expiry(Some(self.license_expire
                                            .sub(self.license_purchase)
                                        ));
                                    }

                                    self.licenses.insert(license.generate_string(true));
                                }
                            }
                        });
                        columns[1].vertical_centered_justified(|ui| {
                            // ? INFO: width here is the text area width of the combobox, not including the arrow button, thanks egui
                            egui::ComboBox::from_id_source("edition_combobox")
                                .width(ui.available_width() - 8.0)
                                .selected_text(self.license_edition.to_string())
                                .show_ui(ui, |ui| {
                                    KeyEdition::iter().for_each(|edition| {
                                        ui.selectable_value(
                                            &mut self.license_edition,
                                            edition,
                                            edition.to_string(),
                                        );
                                    });
                                });
                        });
                    });

                    ui.separator();
                    ui.add(
                        egui::Slider::new(&mut self.license_count, 1..=500)
                            .text("License count")
                            .show_value(true),
                    )
                    .on_hover_text("Number of licenses to generate");

                    ui.add(
                        egui::Slider::new(&mut self.license_seats, 1..=797)
                            .text("Seats")
                            .show_value(true),
                    );

                    ui.horizontal(|ui| {
                        ui.add(
                            DatePicker::new("license_purchase_date", &mut self.license_purchase)
                                .min_date(Utc.ymd(2004, 1, 1))
                                .max_date(Utc.ymd(2099, 12, 31)),
                        );
                        ui.label("Purchase Date");
                    });

                    let min_date = self.license_purchase + Duration::days(1);
                    let max_date = self.license_purchase + Duration::days(3658);

                    self.license_expire = self.license_expire.clamp(min_date, max_date);
                    self.license_maintenance = self.license_maintenance.clamp(min_date, max_date);

                    ui.horizontal(|ui| {
                        ui.add_enabled_ui(!self.license_expire_never, |ui| {
                            ui.add(
                                DatePicker::new("license_expire_date", &mut self.license_expire)
                                    .min_date(min_date)
                                    .max_date(max_date),
                            );
                        });

                        ui.label("Expire Date");
                        ui.checkbox(&mut self.license_expire_never, "No Expiry");
                    });

                    ui.horizontal(|ui| {
                        ui.add(
                            DatePicker::new(
                                "maintenance_expire_date",
                                &mut self.license_maintenance,
                            )
                            .min_date(min_date)
                            .max_date(max_date),
                        );
                        ui.label("Maintenance Expire Date");
                    });
                });

                columns[1].group(|ui| {
                    let available_size = ui.available_size();

                    ui.set_max_size(available_size);
                    ui.set_min_size(available_size);

                    egui::ScrollArea::new([false, true]).show(ui, |ui| {
                        self.licenses.iter().enumerate().for_each(|(idx, license)| {
                            if ui
                                .selectable_label(
                                    matches!(self.selected_license, Some(sel_idx) if sel_idx == idx),
                                    egui::RichText::new(license)
                                        .text_style(egui::TextStyle::Monospace),
                                )
                                .clicked()
                            {
                                self.selected_license = Some(idx);

                                if let Err(e) = self.clipboard_provider.set_contents(license.to_string()) {
                                    self.note = Some(NotePopup::new(format!("Failed to set cliboard content: {e}")));
                                }
                            }
                        });
                    });
                });
            });
        });
    }
}

fn main() {
    let options = eframe::NativeOptions {
        always_on_top: true,
        drag_and_drop_support: false,
        resizable: false,
        initial_window_size: Some(eframe::egui::Vec2::new(520.0, 300.0)),
        ..Default::default()
    };

    eframe::run_native("Key Generator", options, Box::new(|_cc| Box::<App>::default()));
}
