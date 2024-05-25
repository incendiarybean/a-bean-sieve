use eframe::{
    egui::{self, include_image, Id, Sense},
    emath::Align2,
    epaint::{vec2, Color32, FontId, Pos2, Stroke},
};

use crate::default_window::MainWindow;

pub fn task_bar(properties: &mut MainWindow, ui: &mut egui::Ui) {
    let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    let is_dark_mode = ui.ctx().style().visuals.dark_mode;

    let title_bar_height = 32.0;
    let title_bar_rect = {
        let mut rect = ui.max_rect();
        rect.max.y = rect.min.y + title_bar_height;
        rect
    };

    let painter = ui.painter();

    let title_bar_response = ui.interact(title_bar_rect, Id::new("title_bar"), Sense::click());

    painter.text(
        Pos2 {
            x: title_bar_rect.left() + 10.0,
            y: title_bar_rect.height() / 2.0,
        },
        Align2::LEFT_CENTER,
        "Address Blocker",
        FontId::proportional(15.0),
        Color32::WHITE,
    );

    painter.line_segment(
        [
            title_bar_rect.left_bottom() + vec2(1.0, 0.0),
            title_bar_rect.right_bottom() + vec2(-1.0, 0.0),
        ],
        Stroke::new(1.0, Color32::GRAY),
    );

    if title_bar_response.double_clicked() {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
    } else if title_bar_response.is_pointer_button_down_on() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

    ui.allocate_ui_at_rect(title_bar_rect, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.visuals_mut().button_frame = false;
            ui.add_space(8.0);

            let _close_button_response = {
                let button = egui::Image::new(include_image!("./assets/close_button.svg"))
                    .fit_to_fraction(egui::Vec2 { x: 0.8, y: 0.8 })
                    .sense(egui::Sense::hover())
                    .sense(egui::Sense::click())
                    .tint(properties.close_button_tint);

                let response = ui.add(button).on_hover_text("Close the Window.");

                if response.hovered() {
                    properties.close_button_tint = if is_dark_mode {
                        Color32::GRAY
                    } else {
                        Color32::DARK_GRAY
                    }
                } else {
                    properties.close_button_tint = if is_dark_mode {
                        Color32::WHITE
                    } else {
                        Color32::BLACK
                    }
                }

                if response.clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            };

            let _maximise_button_response = {
                let button_image = include_image!("./assets/maximise_button.svg");
                let alt_button_image = include_image!("./assets/maximise_alt_button.svg");

                let current_button_image = if is_maximized {
                    alt_button_image
                } else {
                    button_image
                };

                let button = egui::Image::new(current_button_image)
                    .fit_to_fraction(egui::Vec2 { x: 0.8, y: 0.8 })
                    .sense(egui::Sense::hover())
                    .sense(egui::Sense::click())
                    .tint(properties.maximise_button_tint);

                let response = ui.add(button);

                if response.hovered() {
                    properties.maximise_button_tint = if is_dark_mode {
                        Color32::GRAY
                    } else {
                        Color32::DARK_GRAY
                    }
                } else {
                    properties.maximise_button_tint = if is_dark_mode {
                        Color32::WHITE
                    } else {
                        Color32::BLACK
                    }
                }

                if is_maximized {
                    let restore_response = response.on_hover_text("Restore window");

                    if restore_response.clicked() {
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                    }
                } else {
                    let maximise_response = response.on_hover_text("Maximize window");

                    if maximise_response.clicked() {
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                    }
                }
            };

            let _minimise_button_response = {
                let button = egui::Image::new(include_image!("./assets/minimise_button.svg"))
                    .fit_to_fraction(egui::Vec2 { x: 0.8, y: 0.8 })
                    .sense(egui::Sense::hover())
                    .sense(egui::Sense::click())
                    .tint(properties.minimise_button_tint);

                let response = ui.add(button).on_hover_text("Minimize the window");

                if response.hovered() {
                    if ui.ctx().style().visuals.dark_mode == true {
                        properties.minimise_button_tint = Color32::GRAY;
                    } else {
                        properties.minimise_button_tint = Color32::DARK_GRAY
                    }
                } else {
                    if ui.ctx().style().visuals.dark_mode == true {
                        properties.minimise_button_tint = Color32::WHITE;
                    } else {
                        properties.minimise_button_tint = Color32::BLACK
                    }
                }
                if response.clicked() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
            };
        });
    });
}
