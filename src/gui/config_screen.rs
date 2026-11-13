//! 1:1 port of ConfigScreen.

use super::widget::separator::Separator;
use super::widget::tooltipped_checkbox::TooltippedCheckbox;
use super::widget::tooltipped_slider_button::TooltippedSliderButton;
use crate::config::{Config, ConfigData};
use crate::core::hashed_seed_manager;
use crate::seams;
use crate::NicerSkies;

pub struct ConfigScreen {
    title: seams::Component,
    last_screen: seams::ScreenHandle,
    pub config: Config,
    pub new_config: ConfigData,

    invalidated: bool,

    pub reset_btn: Option<seams::Button>,
    pub apply_btn: Option<seams::Button>,

    pub width: i32,
    pub height: i32,

    widgets: Vec<seams::Widget>,
}

impl ConfigScreen {
    /// java: protected ConfigScreen(Screen lastScreen).
    pub fn new(last_screen: seams::ScreenHandle) -> Self {
        let mut screen = ConfigScreen {
            title: seams::Component::translatable("nicer_skies.menu.settings"),
            last_screen,
            config: Config::from_config_data(ConfigData::default(), None),
            new_config: ConfigData::default(),
            invalidated: false,
            reset_btn: None,
            apply_btn: None,
            width: 0,
            height: 0,
            widgets: Vec::new(),
        };

        let inst_config = {
            let instance = crate::get_instance().lock().unwrap();
            match instance.as_ref() {
                Some(inst) => inst.config.to_builder_build(),
                None => ConfigData::default(),
            }
        };
        screen.config = Config::from_config_data(inst_config, None);

        screen.wipe_config();
        screen
    }

    /// java: private wipeConfig().
    pub fn wipe_config(&mut self) {
        let config_data = self.config.to_builder_build();
        self.new_config = config_data.clone();
    }

    /// java: init().
    pub fn init(&mut self) {
        let btn_dst = 24;
        let nebula_opt_margin = self.width / 2 + (self.width / 2 - 150) / 2;
        let f = seams::font();

        // initial values
        let render_nebulas = self.new_config.render_nebulas;
        let twinkle_stars = self.new_config.twinkling_stars;

        let render_during_day = self.new_config.nebula_config.render_during_day;
        let nebula_strength = self.new_config.nebula_config.nebula_strength;
        let nebula_noise_amount = self.new_config.nebula_config.nebula_noise_amount;
        let nebula_base_colour_amount = self.new_config.nebula_config.base_colour_amount;
        let nebula_noise_scale = self.new_config.nebula_config.nebula_noise_scale;

        let title_string = seams::StringWidget::new(self.title.clone(), &f);
        let tw = title_string.get_width();
        title_string.set_position(self.width / 2 - tw / 2, 10);
        self.add_renderable_widget(seams::Widget::StringWidget(title_string));

        let features_string = seams::StringWidget::new(seams::Component::translatable("nicer_skies.menu.subtitle.feature_toggles"), &f);
        let fw = features_string.get_width();
        features_string.set_position(self.width / 4 - fw / 2, 36);
        self.add_renderable_widget(seams::Widget::StringWidget(features_string));

        let nebula_string = seams::StringWidget::new(seams::Component::translatable("nicer_skies.menu.subtitle.nebula_settings"), &f);
        let nw = nebula_string.get_width();
        nebula_string.set_position(3 * self.width / 4 - nw / 2, 36);
        self.add_renderable_widget(seams::Widget::StringWidget(nebula_string));

        let warning_string = seams::MultiLineTextWidget::new(20, 160, seams::Component::translatable("nicer_skies.menu.compatibility_warning"), &f).set_max_width(self.width / 2 - 40);
        self.add_renderable_widget(seams::Widget::MultiLineTextWidget(warning_string));

        let mut y = 60;
        let checkbox1 = TooltippedCheckbox::new(
            20,
            y,
            seams::Component::translatable("nicer_skies.option.render_nebulas"),
            &f,
            render_nebulas,
            seams::CheckboxOnValueChange(|_cb: *const (), selected: bool| {
                seams::screen_new_config_set_render_nebulas(selected);
                seams::screen_set_invalidated(true);
            }),
            None,
        );
        self.add_renderable_widget(seams::Widget::Checkbox(checkbox1.checkbox));

        let checkbox2 = TooltippedCheckbox::new(
            20,
            {
                y += btn_dst;
                y
            },
            seams::Component::translatable("nicer_skies.option.twinkle_stars"),
            &f,
            twinkle_stars,
            seams::CheckboxOnValueChange(|_cb: *const (), selected: bool| {
                seams::screen_new_config_set_twinkling_stars(selected);
                seams::screen_set_invalidated(true);
            }),
            None,
        );
        self.add_renderable_widget(seams::Widget::Checkbox(checkbox2.checkbox));

        self.add_renderable_only(seams::Widget::Separator(Separator::new(self.width / 2, 30, self.height - 70)));
        y = 60;

        // Render During Day
        let checkbox3 = TooltippedCheckbox::new(
            nebula_opt_margin,
            y,
            seams::Component::translatable("nicer_skies.option.render_during_day"),
            &f,
            render_during_day,
            seams::CheckboxOnValueChange(|_cb: *const (), selected: bool| {
                seams::screen_nebula_config_set_render_during_day(selected);
                seams::screen_set_invalidated(true);
            }),
            Some(seams::Tooltip::create(seams::Component::translatable("nicer_skies.option.render_during_day.tooltip"))),
        );
        self.add_renderable_widget(seams::Widget::Checkbox(checkbox3.checkbox));

        // Nebula Strength
        let slider1 = TooltippedSliderButton::new(
            nebula_opt_margin,
            {
                y += btn_dst;
                y
            },
            150,
            20,
            seams::Component::translatable_with(
                "nicer_skies.option.nebula_transparency",
                &[(format!("{}%", (nebula_strength * 100.0) as i32))],
            ),
            nebula_strength as f64,
            seams::SliderMovedFn(Box::new(|value| {
                seams::Component::translatable_with(
                    "nicer_skies.option.nebula_transparency",
                    &[(format!("{}%", (value * 100.0) as i32))],
                )
            })),
            seams::SliderReleasedFn(Box::new(|value| {
                seams::screen_nebula_config_set_nebula_strength(value as f32);
                seams::screen_set_invalidated(true);
            })),
            Some(seams::Tooltip::create(seams::Component::translatable("nicer_skies.option.nebula_transparency.tooltip"))),
        );
        self.add_renderable_widget(seams::Widget::SliderButton(slider1.button));

        // Nebula Amount
        let slider2 = TooltippedSliderButton::new(
            nebula_opt_margin,
            {
                y += btn_dst;
                y
            },
            150,
            20,
            seams::Component::translatable_with(
                "nicer_skies.option.nebula_amount",
                &[(format!("{}%", (nebula_noise_amount * 100.0) as i32))],
            ),
            nebula_noise_amount as f64,
            seams::SliderMovedFn(Box::new(|value| {
                seams::Component::translatable_with(
                    "nicer_skies.option.nebula_amount",
                    &[(format!("{}%", (value * 100.0) as i32))],
                )
            })),
            seams::SliderReleasedFn(Box::new(|value| {
                seams::screen_nebula_config_set_nebula_noise_amount(value as f32);
                seams::screen_set_invalidated(true);
            })),
            Some(seams::Tooltip::create(seams::Component::translatable("nicer_skies.option.nebula_amount.tooltip"))),
        );
        self.add_renderable_widget(seams::Widget::SliderButton(slider2.button));

        // Background Strength
        let slider3 = TooltippedSliderButton::new(
            nebula_opt_margin,
            {
                y += btn_dst;
                y
            },
            150,
            20,
            seams::Component::translatable_with(
                "nicer_skies.option.background_strength",
                &[(format!("{}", nebula_base_colour_amount))],
            ),
            nebula_base_colour_amount as f64 / 255.0,
            seams::SliderMovedFn(Box::new(|value| {
                seams::Component::translatable_with(
                    "nicer_skies.option.background_strength",
                    &[(format!("{}", (value * 255.0) as i32))],
                )
            })),
            seams::SliderReleasedFn(Box::new(|value| {
                seams::screen_nebula_config_set_base_colour_amount((value * 255.0) as i32);
                seams::screen_set_invalidated(true);
            })),
            Some(seams::Tooltip::create(seams::Component::translatable("nicer_skies.option.background_strength.tooltip"))),
        );
        self.add_renderable_widget(seams::Widget::SliderButton(slider3.button));

        // Nebula Scale
        let slider4 = TooltippedSliderButton::new(
            nebula_opt_margin,
            {
                y += btn_dst;
                y
            },
            150,
            20,
            seams::Component::translatable_with(
                "nicer_skies.option.nebula_scale",
                &[(format!("{}x", Self::format_scale(nebula_noise_scale as f64)))],
            ),
            Self::map_value_to_scale(nebula_noise_scale as f64),
            seams::SliderMovedFn(Box::new(|value| {
                seams::Component::translatable_with(
                    "nicer_skies.option.nebula_scale",
                    &[(format!("{}x", Self::format_scale(Self::map_scale_to_value(value))))],
                )
            })),
            seams::SliderReleasedFn(Box::new(|value| {
                seams::screen_nebula_config_set_nebula_noise_scale(Self::map_scale_to_value(value) as f32);
                seams::screen_set_invalidated(true);
            })),
            Some(seams::Tooltip::create(seams::Component::translatable("nicer_skies.option.nebula_scale.tooltip"))),
        );
        self.add_renderable_widget(seams::Widget::SliderButton(slider4.button));

        // Reset
        let reset_btn = seams::Button::builder(
            seams::Component::translatable("nicer_skies.menu.reset"),
            Box::new(|| {
                seams::screen_reset_nebula_config_to_default();
                seams::screen_rebuild_widgets();
                seams::screen_set_invalidated(true);
            }),
        )
        .bounds(nebula_opt_margin, {
            y += btn_dst;
            y
        }, 150, 20)
        .build();
        self.reset_btn = Some(reset_btn.clone_handle());
        self.add_renderable_widget(seams::Widget::Button(reset_btn));

        // Apply
        let apply_btn = seams::Button::builder(
            seams::Component::translatable("nicer_skies.menu.apply"),
            Box::new(|| {
                seams::screen_apply_config();
                seams::screen_regenerate_sky();
                seams::screen_set_invalidated(false);
            }),
        )
        .bounds(self.width / 2 + 4, self.height - 28, 150, 20)
        .build();
        self.apply_btn = Some(apply_btn.clone_handle());
        self.add_renderable_widget(seams::Widget::Button(apply_btn));

        // Back
        let back_btn = seams::Button::builder(
            seams::Component::translatable("nicer_skies.menu.back"),
            Box::new(|| {
                seams::screen_on_close();
            }),
        )
        .pos(self.width / 2 - 154, self.height - 28)
        .size(150, 20)
        .build();
        self.add_renderable_widget(seams::Widget::Button(back_btn));
    }

    pub fn add_renderable_widget(&mut self, widget: seams::Widget) {
        self.widgets.push(widget);
    }

    pub fn add_renderable_only(&mut self, widget: seams::Widget) {
        self.widgets.push(widget);
    }

    /// java: extractRenderState.
    pub fn extract_render_state(&mut self, g: &mut seams::GuiGraphicsExtractor, mouse_x: i32, mouse_y: i32, partial_tick: f32) {
        let reset_active = !self.is_default_nebula_settings();
        if let Some(reset_btn) = self.reset_btn.as_mut() {
            reset_btn.active = reset_active;
        }
        let apply_active = self.invalidated;
        if let Some(apply_btn) = self.apply_btn.as_mut() {
            apply_btn.active = apply_active;
        }
    }

    /// java: onClose().
    pub fn on_close(&mut self) {
        self.wipe_config();
        seams::set_screen(self.last_screen);
    }

    pub fn regenerate_sky(&mut self) {
        if hashed_seed_manager::can_generate_sky() {
            let seed = hashed_seed_manager::get_seed();
            let mut instance = crate::get_instance().lock().unwrap();
            if let Some(inst) = instance.as_mut() {
                inst.renderer.generate_sky(seed);
            }
        }
        self.invalidated = false;
    }

    /// java: private isDefaultNebulaSettings().
    pub fn is_default_nebula_settings(&self) -> bool {
        self.new_config.nebula_config == Config::DEFAULT_CONFIG.nebula_config
    }

    /// java: private float mapScaleToValue(double value).
    pub fn map_scale_to_value(value: f64) -> f64 {
        ((1.0 / (value * 1.5 + 0.5)) * 100.0).round() / 100.0
    }

    /// inverse of above
    pub fn map_value_to_scale(value: f64) -> f64 {
        ((1.0 / value) - 0.5) / 1.5
    }

    /// java: nebula_scale uses the raw float value in the label ("%dx").
    fn format_scale(v: f64) -> String {
        // java: String.format uses the float's own decimal form.
        format!("{}", v)
    }
}
