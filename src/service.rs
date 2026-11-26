use anyhow::{anyhow, Context, Result};
use std::{
    fs,
    process::Command,
    thread,
    time::{Duration, Instant},
};

use crate::{install, log, paths::InstallPaths, state};

#[cfg(windows)]
use std::sync::mpsc::Sender;
#[cfg(windows)]
use tray_icon::{
    menu::{Menu, MenuItem},
    Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

enum Message {
    Stop,
    Restart,
}

fn supervisor(paths: InstallPaths, rx: std::sync::mpsc::Receiver<Message>) {
    use std::process::Child;

    fn spawn(paths: &InstallPaths) -> Result<Child> {
        let mut command = Command::new(&paths.exe);
        command.arg("--worker");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        command.spawn().context("启动保活工作进程失败")
    }

    install::mark_service_started(&paths);
    let mut child: Option<Child> = None;
    loop {
        if child.is_none() {
            match spawn(&paths) {
                Ok(spawned) => child = Some(spawned),
                Err(error) => {
                    log(format!("{error:#}"));
                    thread::sleep(Duration::from_secs(3));
                    continue;
                }
            }
        }
        log("保活工作进程已启动");

        let mut stop = false;
        let mut restart = false;
        loop {
            match rx.recv_timeout(Duration::from_millis(250)) {
                Ok(Message::Stop) => {
                    stop = true;
                    break;
                }
                Ok(Message::Restart) => {
                    restart = true;
                    break;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    stop = true;
                    break;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            }
            let exited = child
                .as_mut()
                .and_then(|process| process.try_wait().ok())
                .flatten()
                .is_some();
            if exited {
                log("保活工作进程已退出，准备重启");
                break;
            }
        }
        if let Some(mut process) = child.take() {
            let _ = process.kill();
            let _ = process.wait();
        }
        if stop {
            install::mark_service_stopped(&paths);
            log("后台服务已停止");
            return;
        }
        if !restart {
            thread::sleep(Duration::from_millis(800));
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn run(paths: &InstallPaths) -> Result<()> {
    let (_tx, rx) = std::sync::mpsc::channel();
    supervisor(paths.clone(), rx);
    Ok(())
}

#[cfg(windows)]
pub(crate) fn run(paths: &InstallPaths) -> Result<()> {
    use eframe::egui;
    use std::sync::mpsc;

    install::hide_console();

    let (tx, rx) = mpsc::channel();
    let supervisor_paths = paths.clone();
    let handle = std::thread::spawn(move || supervisor(supervisor_paths, rx));

    let menu = Menu::new();
    menu.append(&MenuItem::with_id("open", "打开界面", true, None))?;
    menu.append(&tray_icon::menu::PredefinedMenuItem::separator())?;
    menu.append(&MenuItem::with_id("quit", "退出", true, None))?;
    let tray = TrayIconBuilder::new()
        .with_icon(make_icon())
        .with_menu(Box::new(menu))
        .build()
        .context("创建系统托盘图标失败")?;

    let app = App {
        tx,
        paths: paths.clone(),
        _tray: tray,
        ctx: None,
        status: state::read(&paths.status),
        logs: crate::logging::tail(40),
        last_poll: Instant::now() - Duration::from_secs(2),
        notice: String::new(),
        quitting: false,
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TYUT 校园网保活")
            .with_visible(false)
            .with_inner_size([520.0, 420.0]),
        renderer: eframe::Renderer::Glow,
        run_and_return: false,
        ..Default::default()
    };
    let result = eframe::run_native(
        "TYUT 校园网保活",
        options,
        Box::new(move |_cc| Ok(Box::new(app))),
    );

    match handle.join() {
        Ok(()) => {}
        Err(_) => log("管理线程异常退出"),
    }
    result.map_err(|error| anyhow!("界面运行失败：{error}"))
}

#[cfg(windows)]
fn make_icon() -> Icon {
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32u32 {
        for x in 0..32u32 {
            let border = x < 2 || y < 2 || x > 29 || y > 29;
            let signal = (8..24).contains(&x) && (6..12).contains(&y)
                || (11..21).contains(&x) && (14..20).contains(&y)
                || (14..18).contains(&x) && (22..28).contains(&y);
            let (r, g, b) = if border {
                (90, 120, 160)
            } else if signal {
                (80, 200, 120)
            } else {
                (30, 45, 60)
            };
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    Icon::from_rgba(rgba, 32, 32).expect("内置托盘图标数据无效")
}

#[cfg(windows)]
struct App {
    tx: Sender<Message>,
    paths: InstallPaths,
    _tray: TrayIcon,
    ctx: Option<eframe::egui::Context>,
    status: String,
    logs: String,
    last_poll: Instant,
    notice: String,
    quitting: bool,
}

#[cfg(windows)]
impl eframe::App for App {
    fn logic(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        use eframe::egui::ViewportCommand;
        use tray_icon::menu::MenuEvent;

        self.ctx = Some(ctx.clone());
        if !self.quitting && ctx.input(|input| input.viewport().close_requested()) {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        }
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    ..
                }
            ) {
                ctx.send_viewport_cmd(ViewportCommand::Visible(true));
            }
        }
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id().as_ref() {
                "open" => {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                }
                "quit" => {
                    self.quitting = true;
                    let _ = self.tx.send(Message::Stop);
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                _ => {}
            }
        }
        if self.last_poll.elapsed() >= Duration::from_secs(1) {
            self.status = state::read(&self.paths.status);
            self.logs = crate::logging::tail(40);
            self.last_poll = Instant::now();
            ctx.request_repaint();
        }
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        use eframe::egui::{self, ViewportCommand};

        ui.heading("TYUT 校园网保活");
        ui.separator();
        ui.label(&self.status);
        if !self.notice.is_empty() {
            ui.colored_label(egui::Color32::from_rgb(60, 160, 100), &self.notice);
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("立即重连").clicked() {
                match fs::write(self.paths.control_file("reconnect"), b"1") {
                    Ok(()) => self.notice = "已发送重连请求".to_string(),
                    Err(error) => self.notice = format!("发送失败：{error}"),
                }
            }
            if ui.button("重置凭据").clicked() {
                let removed = fs::remove_file(&self.paths.config).is_ok();
                let _ = fs::write(self.paths.control_file("reset"), b"1");
                let _ = self.tx.send(Message::Restart);
                let mut command = Command::new(&self.paths.exe);
                command.arg("--setup");
                if let Err(error) = command.spawn() {
                    self.notice = format!("请手动运行 --setup：{error}");
                } else {
                    self.notice = if removed {
                        "凭据已删除，请在弹出的窗口重新输入".to_string()
                    } else {
                        "请在弹出的窗口重新输入凭据".to_string()
                    };
                }
            }
        });
        ui.horizontal(|ui| {
            let enabled = install::autostart_enabled(&self.paths);
            let label = if enabled {
                "禁用开机自启"
            } else {
                "启用开机自启"
            };
            if ui.button(label).clicked() {
                let result = if enabled {
                    install::unregister_autostart(&self.paths)
                } else {
                    install::register_autostart(&self.paths)
                };
                self.notice = match result {
                    Ok(()) => "自启设置已更新".to_string(),
                    Err(error) => format!("操作失败：{error:#}"),
                };
            }
            if ui.button("退出").clicked() {
                self.quitting = true;
                let _ = self.tx.send(Message::Stop);
                if let Some(ctx) = &self.ctx {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        });
        ui.separator();
        ui.label("最近日志");
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.monospace(&self.logs);
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.quitting {
            let _ = self.tx.send(Message::Stop);
        }
    }
}
