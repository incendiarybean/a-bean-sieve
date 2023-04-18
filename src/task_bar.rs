use eframe::{
    egui::{self, Id, Sense},
    emath::Align2,
    epaint::{vec2, Color32, FontId, Pos2},
};

use crate::default_window::MainWindow;

pub fn task_bar(properties: &mut MainWindow, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
    let title_bar_height = 26.0;
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
        ui.style().visuals.text_color(),
    );

    painter.line_segment(
        [
            title_bar_rect.left_bottom() + vec2(1.0, 0.0),
            title_bar_rect.right_bottom() + vec2(-1.0, 0.0),
        ],
        ui.visuals().widgets.noninteractive.bg_stroke,
    );

    if title_bar_response.double_clicked() {
        frame.set_maximized(!frame.info().window_info.maximized);
    } else if title_bar_response.is_pointer_button_down_on() {
        frame.drag_window();
    }

    ui.allocate_ui_at_rect(title_bar_rect, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.visuals_mut().button_frame = false;
            ui.add_space(8.0);

            let _close_button_response = {
                let button_svg = egui_extras::RetainedImage::from_svg_bytes_with_size(
                    "close_button",
                    include_bytes!("./svg/close_button.svg"),
                    egui_extras::image::FitTo::Original,
                )
                .unwrap();

                let image =
                    egui::Image::new(button_svg.texture_id(ui.ctx()), button_svg.size_vec2())
                        .tint(properties.close_button_tint);

                let button_image = image
                    .sense(egui::Sense::hover())
                    .sense(egui::Sense::click());

                let response = ui.add(button_image).on_hover_text("Close the Window.");

                if response.hovered() {
                    if ui.ctx().style().visuals.dark_mode == true {
                        properties.close_button_tint = Color32::GRAY;
                    } else {
                        properties.close_button_tint = Color32::DARK_GRAY
                    }
                } else {
                    if ui.ctx().style().visuals.dark_mode == true {
                        properties.close_button_tint = Color32::WHITE;
                    } else {
                        properties.close_button_tint = Color32::BLACK
                    }
                }

                if response.clicked() {
                    frame.close();
                }
            };

            let _maximise_button_response = {
                let maximise_button_svg = egui_extras::RetainedImage::from_svg_bytes_with_size(
                    "maximise_button",
                    include_bytes!("./svg/maximise_button.svg"),
                    egui_extras::image::FitTo::Original,
                )
                .unwrap();

                let maximise_button_alt_svg = egui_extras::RetainedImage::from_svg_bytes_with_size(
                    "maximise_alt_button",
                    include_bytes!("./svg/maximise_alt_button.svg"),
                    egui_extras::image::FitTo::Original,
                )
                .unwrap();

                let maximise_button_img = egui::Image::new(
                    maximise_button_svg.texture_id(ui.ctx()),
                    maximise_button_svg.size_vec2(),
                )
                .tint(properties.maximise_button_tint);

                let maximise_button_alt_img = egui::Image::new(
                    maximise_button_alt_svg.texture_id(ui.ctx()),
                    maximise_button_alt_svg.size_vec2(),
                )
                .tint(properties.maximise_button_tint);

                let maximise_button_image = maximise_button_img
                    .sense(egui::Sense::hover())
                    .sense(egui::Sense::click());

                let maximise_button_alt_image = maximise_button_alt_img
                    .sense(egui::Sense::hover())
                    .sense(egui::Sense::click());

                let response = if frame.info().window_info.maximized {
                    ui.add(maximise_button_alt_image)
                } else {
                    ui.add(maximise_button_image)
                };

                if response.hovered() {
                    if ui.ctx().style().visuals.dark_mode == true {
                        properties.maximise_button_tint = Color32::GRAY;
                    } else {
                        properties.maximise_button_tint = Color32::DARK_GRAY
                    }
                } else {
                    if ui.ctx().style().visuals.dark_mode == true {
                        properties.maximise_button_tint = Color32::WHITE;
                    } else {
                        properties.maximise_button_tint = Color32::BLACK
                    }
                }

                if frame.info().window_info.maximized {
                    let restore_response = response.on_hover_text("Restore window");

                    if restore_response.clicked() {
                        frame.set_maximized(false);
                    }
                } else {
                    let maximise_response = response.on_hover_text("Maximize window");

                    if maximise_response.clicked() {
                        frame.set_maximized(true);
                    }
                }
            };

            let _minimise_button_response = {
                let button_svg = egui_extras::RetainedImage::from_svg_bytes_with_size(
                    "minimise_button",
                    include_bytes!("./svg/minimise_button.svg"),
                    egui_extras::image::FitTo::Original,
                )
                .unwrap();

                let image =
                    egui::Image::new(button_svg.texture_id(ui.ctx()), button_svg.size_vec2())
                        .tint(properties.minimise_button_tint);

                let button_image = image
                    .sense(egui::Sense::hover())
                    .sense(egui::Sense::click());

                let response = ui.add(button_image).on_hover_text("Minimize the window");

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
                    frame.set_minimized(true);
                }
            };
        });
    });
}
