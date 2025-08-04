#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use discord_sdk::{self as ds, Subscriptions};
use ds::Discord;

use eframe::{egui, App, CreationContext, NativeOptions};
use egui::{Rounding, ScrollArea, Visuals};

use once_cell::sync::Lazy;
use tokio::sync::Mutex as AsyncMutex;
use tracing::{error, info};
use tracing_subscriber::fmt::time::OffsetTime;
use tracing_subscriber::{layer::SubscriberExt, Layer, Registry};

const APP_ID: ds::AppId = ;

static LOGS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[derive(Default)]
struct StringVisitor {
    output: String,
}

impl tracing::field::Visit for StringVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        // フィールド名=値 の形式で追記する
        use std::fmt::Write as _;
        let _ = write!(self.output, "{}={:?} ", field.name(), value);
    }
}

struct LogCaptureLayer;

impl<S> Layer<S> for LogCaptureLayer
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // イベントフィールドを visitor で文字列化
        let mut visitor = StringVisitor::default();
        event.record(&mut visitor);

        // LOGS へ同期的に push
        let mut logs = LOGS.lock().unwrap();
        logs.push(visitor.output);
    }
}

fn setup_logging() {
    let timer = OffsetTime::local_rfc_3339().expect("cannot get local offset");
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_timer(timer)
        .with_target(false)
        .with_thread_ids(false);

    let subscriber = Registry::default().with(fmt_layer).with(LogCaptureLayer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set global default subscriber");
}

#[derive(Default)]
struct AppState {
    discord: Option<Discord>,
}

struct MyEguiApp {
    shared: Arc<AsyncMutex<AppState>>,

    details: String,
    state: String,
    large_img: String,
    large_txt: String,
    small_img: String,
    small_txt: String,
    button_label: String,
    button_url: String,
}

impl MyEguiApp {
    fn new(cc: &CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);

        Self {
            shared: Arc::new(AsyncMutex::new(AppState::default())),

            details: "".into(),
            state: "".into(),
            large_img: "85248977".into(),
            large_txt: "Numbani".into(),
            small_img: "00236-2915952634".into(),
            small_txt: "Rogue - Level 100".into(),
            button_label: "Visit Website".into(),
            button_url: "https://www.google.com".into(),
        }
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "meiryo".to_owned(),
        egui::FontData::from_static(include_bytes!("./meiryo.ttc")).into(),
    );

    if let Some(proportional) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        proportional.insert(0, "meiryo".to_owned());
    }
    if let Some(monospace) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        monospace.insert(0, "meiryo".to_owned());
    }

    // 設定を適用
    ctx.set_fonts(fonts);
}

impl App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut style = (*ctx.style()).clone();
        style.visuals = Visuals::dark();
        style.visuals.widgets.inactive.rounding = Rounding::same(8.0);
        ctx.set_style(style.clone());

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Discord Connect / Disconnect Example");

            ui.separator();

            ui.collapsing("Rich Presence Basic Info", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Details:");
                    ui.text_edit_singleline(&mut self.details);
                });
                ui.horizontal(|ui| {
                    ui.label("State:");
                    ui.text_edit_singleline(&mut self.state);
                });
            });

            ui.collapsing("Images", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Large Key:");
                    ui.text_edit_singleline(&mut self.large_img);
                });
                ui.horizontal(|ui| {
                    ui.label("Large Tooltip:");
                    ui.text_edit_singleline(&mut self.large_txt);
                });
                ui.horizontal(|ui| {
                    ui.label("Small Key:");
                    ui.text_edit_singleline(&mut self.small_img);
                });
                ui.horizontal(|ui| {
                    ui.label("Small Tooltip:");
                    ui.text_edit_singleline(&mut self.small_txt);
                });
            });

            ui.collapsing("Button", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Label:");
                    ui.text_edit_singleline(&mut self.button_label);
                });
                ui.horizontal(|ui| {
                    ui.label("URL:");
                    ui.text_edit_singleline(&mut self.button_url);
                });
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Start").clicked() {
                    let shared_clone = self.shared.clone();
                    tokio::spawn(async move {
                        if let Err(e) = connect_discord(shared_clone).await {
                            error!("Failed to connect Discord: {:?}", e);
                        }
                    });
                }

                if ui.button("Stop").clicked() {
                    let shared_clone = self.shared.clone();
                    tokio::spawn(async move {
                        if let Err(e) = disconnect_discord(shared_clone).await {
                            error!("Failed to stop Discord: {:?}", e);
                        }
                    });
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Update Activity").clicked() {
                    let details = self.details.clone();
                    let state_str = self.state.clone();
                    let l_img = self.large_img.clone();
                    let l_txt = self.large_txt.clone();
                    let s_img = self.small_img.clone();
                    let s_txt = self.small_txt.clone();
                    let b_label = self.button_label.clone();
                    let b_url = self.button_url.clone();

                    let shared_clone = self.shared.clone();
                    tokio::spawn(async move {
                        if let Err(e) = update_activity(
                            shared_clone,
                            details,
                            state_str,
                            l_img,
                            l_txt,
                            s_img,
                            s_txt,
                            b_label,
                            b_url,
                        )
                        .await
                        {
                            error!("Failed to update activity: {:?}", e);
                        }
                    });
                }

                if ui.button("Clear Activity").clicked() {
                    let shared_clone = self.shared.clone();
                    tokio::spawn(async move {
                        if let Err(e) = clear_activity(shared_clone).await {
                            error!("Failed to clear activity: {:?}", e);
                        }
                    });
                }
            });
        });

        egui::SidePanel::right("LogPanel")
            .resizable(true)
            .min_width(400.0)
            .max_width(400.0)
            .show(ctx, |ui| {
                ui.heading("Logs");
                ui.separator();

                ScrollArea::vertical().show(ui, |ui| {
                    // グローバルログバッファを参照 (同期Mutexなので try_lock でも block でもOK)
                    if let Ok(logs) = LOGS.try_lock() {
                        // 新しい順に表示したいなら logs.iter().rev() などにする
                        for line in logs.iter() {
                            ui.label(line);
                        }
                    } else {
                        ui.label("Cannot lock logs right now...");
                    }
                });
            });
    }
}

// --------------------------
// 非同期関数群 (Discordとのやりとり)
// --------------------------

async fn connect_discord(shared: Arc<AsyncMutex<AppState>>) -> anyhow::Result<()> {
    let mut state = shared.lock().await;
    if state.discord.is_some() {
        info!("Already connected.");
        return Ok(());
    }

    let (wheel, handler) = ds::wheel::Wheel::new(Box::new(|err| {
        error!(error=?err, "encountered an error");
    }));

    let mut user_state = wheel.user();
    let discord = ds::Discord::new(
        ds::DiscordApp::PlainId(APP_ID),
        Subscriptions::ACTIVITY,
        Box::new(handler),
    )?;
    info!("waiting for handshake...");
    user_state.0.changed().await?;

    let _user = match &*user_state.0.borrow() {
        ds::wheel::UserState::Connected(u) => {
            info!("Connected to Discord: user={:?}", u);
            u.clone()
        }
        ds::wheel::UserState::Disconnected(err) => {
            error!("failed to connect to Discord: {}", err);
            return Ok(()); // あるいは bail!(err)
        }
    };

    let mut activity_events = wheel.activity();
    tokio::spawn(async move {
        while let Ok(evt) = activity_events.0.recv().await {
            info!("received activity event: {:?}", evt);
        }
    });

    state.discord = Some(discord);

    Ok(())
}

async fn disconnect_discord(shared: Arc<AsyncMutex<AppState>>) -> anyhow::Result<()> {
    let mut state = shared.lock().await;
    if let Some(discord) = state.discord.take() {
        discord.disconnect().await;
        info!("Discord disconnected.");
    } else {
        info!("No Discord instance to stop.");
    }
    Ok(())
}

async fn update_activity(
    shared: Arc<AsyncMutex<AppState>>,
    details: String,
    state_str: String,
    large_img: String,
    large_txt: String,
    small_img: String,
    small_txt: String,
    button_label: String,
    button_url: String,
) -> anyhow::Result<()> {
    let state_lock = shared.lock().await;
    let Some(discord) = &state_lock.discord else {
        info!("update_activity: Not connected yet.");
        return Ok(());
    };

    let rp = ds::activity::ActivityBuilder::default()
        .details(details)
        .state(state_str)
        .assets(
            ds::activity::Assets::default()
                .large(large_img, Some(large_txt))
                .small(small_img, Some(small_txt)),
        )
        .button(ds::activity::Button {
            label: button_label,
            url: button_url,
        })
        .start_timestamp(SystemTime::now());

    discord.update_activity(rp).await?;
    info!("Activity updated.");
    Ok(())
}

async fn clear_activity(shared: Arc<AsyncMutex<AppState>>) -> anyhow::Result<()> {
    let state_lock = shared.lock().await;
    let Some(discord) = &state_lock.discord else {
        info!("clear_activity: Not connected yet.");
        return Ok(());
    };
    discord.clear_activity().await?;
    info!("Activity cleared.");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1) tracing初期化
    setup_logging();
    info!("Application started.");

    // 2) eframe + eguiでGUI起動
    let native_options = NativeOptions {
        vsync: true,
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        hardware_acceleration: eframe::HardwareAcceleration::Preferred,
        renderer: eframe::Renderer::default(),
        run_and_return: false,
        event_loop_builder: None,
        window_builder: None,
        shader_version: None,
        centered: true,
        persist_window: false,
        persistence_path: None,
        dithering: false,
        viewport: egui::ViewportBuilder::default().with_inner_size([800f32, 600f32]),
    };

    let _ = eframe::run_native(
        "Discord Connect/Disconnect Example",
        native_options,
        Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc)))),
    );

    info!("Application closed.");
    Ok(())
}
