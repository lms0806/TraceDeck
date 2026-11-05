use std::num::NonZeroU16;
use std::sync::Arc;

use async_channel::Sender;
use gpui::{
    App, Application, Bounds, Context, Div, FocusHandle, IntoElement, Render,
    StatefulInteractiveElement, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb,
    size,
};
use trace_deck_core::{
    AppPaths, ConnectionProfile, ProfileStore, RemoteEntry, RemoteSystemStatus, SystemStatusChunk,
    TailChunk, TerminalChunk, logs, sftp, system, terminal,
};

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 800.0;
const APP_BACKGROUND: u32 = 0x111820;
const PANEL_BACKGROUND: u32 = 0x151e28;
const BORDER: u32 = 0x293846;
const MUTED_TEXT: u32 = 0x8ba1b5;
const ACCENT: u32 = 0x36c79a;

struct TraceDeckGpui {
    core_status: String,
    profiles: Vec<ConnectionProfile>,
    selected_profile_id: Option<String>,
    remote_path: String,
    remote_entries: Vec<RemoteEntry>,
    known_hosts_file: std::path::PathBuf,
    notice: Option<String>,
    active_log_path: Option<String>,
    log_content: String,
    tail_stream_id: Option<u64>,
    system_stream_id: Option<u64>,
    remote_status: Option<RemoteSystemStatus>,
    tail_streams: Arc<logs::TailStreams>,
    system_streams: Arc<system::SystemStreams>,
    stream_sender: Sender<StreamEvent>,
    terminal_sessions: terminal::TerminalSessions,
    terminal_tabs: Vec<TerminalTab>,
    active_terminal_id: Option<u64>,
    terminal_focus: FocusHandle,
    show_new_session_dialog: bool,
}

enum StreamEvent {
    Tail(TailChunk),
    System(SystemStatusChunk),
    Terminal(TerminalChunk),
}

struct TerminalTab {
    terminal_id: u64,
    title: String,
    parser: vt100::Parser,
}

impl TraceDeckGpui {
    fn open_new_session(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.show_new_session_dialog = true;
        cx.notify();
    }

    fn close_new_session(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.show_new_session_dialog = false;
        cx.notify();
    }

    fn select_profile(&mut self, profile_id: String, cx: &mut Context<Self>) {
        self.stop_streams();
        self.selected_profile_id = Some(profile_id);
        self.active_log_path = None;
        self.log_content.clear();
        self.remote_status = None;
        self.load_directory("/".to_owned(), cx);
        self.start_system_status_stream();
    }

    fn open_directory(&mut self, path: String, cx: &mut Context<Self>) {
        self.load_directory(path, cx);
    }

    fn load_directory(&mut self, path: String, cx: &mut Context<Self>) {
        let Some(profile) = self
            .selected_profile_id
            .as_ref()
            .and_then(|id| self.profiles.iter().find(|profile| profile.id == *id))
            .cloned()
        else {
            return;
        };

        self.notice = Some("원격 폴더를 불러오는 중입니다…".to_owned());
        match sftp::list_directory(&profile, &path, &self.known_hosts_file) {
            Ok(entries) => {
                self.remote_path = path;
                self.remote_entries = entries;
                self.notice = None;
            }
            Err(error) => {
                self.remote_entries.clear();
                self.notice = Some(error);
            }
        }
        cx.notify();
    }

    fn open_log(&mut self, path: String, cx: &mut Context<Self>) {
        let Some(profile) = self
            .selected_profile_id
            .as_ref()
            .and_then(|id| self.profiles.iter().find(|profile| profile.id == *id))
            .cloned()
        else {
            return;
        };

        self.stop_tail_stream();
        match logs::read_recent_file(&profile, path.clone(), &self.known_hosts_file) {
            Ok(file) => {
                self.active_log_path = Some(file.path.clone());
                self.log_content = file.content;
                let sender = self.stream_sender.clone();
                match logs::start_tail_stream(
                    &profile,
                    &file.path,
                    &self.known_hosts_file,
                    &self.tail_streams,
                    move |chunk| {
                        let _ = sender.try_send(StreamEvent::Tail(chunk));
                    },
                ) {
                    Ok(stream_id) => self.tail_stream_id = Some(stream_id),
                    Err(error) => self.notice = Some(error),
                }
            }
            Err(error) => self.notice = Some(error),
        }
        cx.notify();
    }

    fn start_system_status_stream(&mut self) {
        let Some(profile) = self
            .selected_profile_id
            .as_ref()
            .and_then(|id| self.profiles.iter().find(|profile| profile.id == *id))
            .cloned()
        else {
            return;
        };
        let sender = self.stream_sender.clone();
        match system::start_remote_system_stream(
            &profile,
            &self.known_hosts_file,
            &self.system_streams,
            move |chunk| {
                let _ = sender.try_send(StreamEvent::System(chunk));
            },
        ) {
            Ok(stream_id) => self.system_stream_id = Some(stream_id),
            Err(error) => self.notice = Some(error),
        }
    }

    fn stop_tail_stream(&mut self) {
        if let Some(stream_id) = self.tail_stream_id.take() {
            let _ = logs::stop_tail_stream(stream_id, &self.tail_streams);
        }
    }

    fn stop_streams(&mut self) {
        self.stop_tail_stream();
        if let Some(stream_id) = self.system_stream_id.take() {
            let _ = system::stop_remote_system_stream(stream_id, &self.system_streams);
        }
    }

    fn open_terminal(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(profile) = self
            .selected_profile_id
            .as_ref()
            .and_then(|id| self.profiles.iter().find(|profile| profile.id == *id))
            .cloned()
        else {
            self.notice = Some("먼저 저장 세션을 선택하세요.".to_owned());
            cx.notify();
            return;
        };

        let sender = self.stream_sender.clone();
        match terminal::start_terminal(
            &profile,
            &self.known_hosts_file,
            &self.terminal_sessions,
            move |chunk| {
                let _ = sender.try_send(StreamEvent::Terminal(chunk));
            },
        ) {
            Ok(terminal_id) => {
                self.terminal_tabs.push(TerminalTab {
                    terminal_id,
                    title: format!("{} 터미널", profile.name),
                    parser: vt100::Parser::new(
                        NonZeroU16::new(30).expect("30 is non-zero"),
                        NonZeroU16::new(120).expect("120 is non-zero"),
                        5_000,
                    ),
                });
                self.active_terminal_id = Some(terminal_id);
            }
            Err(error) => self.notice = Some(error),
        }
        cx.notify();
    }

    fn send_terminal_key(
        &mut self,
        event: &gpui::KeyDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(terminal_id) = self.active_terminal_id else {
            return;
        };
        let key = event.keystroke.key.as_str();
        let input = match key {
            "enter" => "\r".to_owned(),
            "backspace" => "\u{7f}".to_owned(),
            "tab" => "\t".to_owned(),
            "escape" => "\u{1b}".to_owned(),
            "up" => "\u{1b}[A".to_owned(),
            "down" => "\u{1b}[B".to_owned(),
            "right" => "\u{1b}[C".to_owned(),
            "left" => "\u{1b}[D".to_owned(),
            value if value.chars().count() == 1 => value.to_owned(),
            _ => return,
        };
        if let Err(error) = terminal::write_terminal(terminal_id, input, &self.terminal_sessions) {
            self.notice = Some(error);
            cx.notify();
        }
    }

    fn focus_terminal(&mut self, _: &gpui::ClickEvent, window: &mut Window, _: &mut Context<Self>) {
        window.focus(&self.terminal_focus);
    }

    fn select_terminal(&mut self, terminal_id: u64, _: &mut Window, cx: &mut Context<Self>) {
        self.active_terminal_id = Some(terminal_id);
        cx.notify();
    }

    fn close_terminal(&mut self, terminal_id: u64, _: &mut Window, cx: &mut Context<Self>) {
        let _ = terminal::stop_terminal(terminal_id, &self.terminal_sessions);
        self.terminal_tabs
            .retain(|tab| tab.terminal_id != terminal_id);
        if self.active_terminal_id == Some(terminal_id) {
            self.active_terminal_id = self.terminal_tabs.last().map(|tab| tab.terminal_id);
        }
        cx.notify();
    }

    fn receive_stream_event(&mut self, event: StreamEvent, cx: &mut Context<Self>) {
        match event {
            StreamEvent::Tail(chunk) if Some(chunk.stream_id) == self.tail_stream_id => {
                self.log_content.push_str(&chunk.chunk);
                const MAX_LOG_CHARACTERS: usize = 100_000;
                if self.log_content.chars().count() > MAX_LOG_CHARACTERS {
                    self.log_content = self
                        .log_content
                        .chars()
                        .skip(self.log_content.chars().count() - MAX_LOG_CHARACTERS)
                        .collect();
                }
                cx.notify();
            }
            StreamEvent::System(chunk) if Some(chunk.stream_id) == self.system_stream_id => {
                self.remote_status = Some(chunk.status);
                cx.notify();
            }
            StreamEvent::Terminal(chunk) => {
                if let Some(tab) = self
                    .terminal_tabs
                    .iter_mut()
                    .find(|tab| tab.terminal_id == chunk.terminal_id)
                {
                    tab.parser.process(chunk.chunk.as_bytes());
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    fn parent_directory(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        let parent = self
            .remote_path
            .trim_end_matches('/')
            .rsplit_once('/')
            .map_or(
                "/",
                |(parent, _)| if parent.is_empty() { "/" } else { parent },
            )
            .to_owned();
        self.load_directory(parent, cx);
    }

    fn header(&self, cx: &mut Context<Self>) -> Div {
        div()
            .h(px(52.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .px(px(16.0))
            .bg(rgb(PANEL_BACKGROUND))
            .border_b_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("TraceDeck"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x7fa4c3))
                            .child("SFTP FILE BROWSER"),
                    ),
            )
            .children(self.terminal_tabs.iter().enumerate().map(|(index, tab)| {
                let terminal_id = tab.terminal_id;
                let active = self.active_terminal_id == Some(terminal_id);
                div()
                    .id(("terminal-tab", index))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .bg(if active { rgb(0x1c2935) } else { rgb(0x101820) })
                    .px(px(12.0))
                    .py(px(6.0))
                    .text_xs()
                    .text_color(if active {
                        rgb(0xffffff)
                    } else {
                        rgb(MUTED_TEXT)
                    })
                    .child(tab.title.clone())
                    .child(
                        div()
                            .id(("close-terminal", index))
                            .cursor_pointer()
                            .px(px(6.0))
                            .child("×")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.close_terminal(terminal_id, window, cx);
                            })),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.select_terminal(terminal_id, window, cx);
                    }))
            }))
            .child(
                div()
                    .id("new-session")
                    .cursor_pointer()
                    .rounded_md()
                    .bg(rgb(0x267f67))
                    .px(px(14.0))
                    .py(px(8.0))
                    .text_sm()
                    .text_color(gpui::white())
                    .hover(|style| style.bg(rgb(ACCENT)))
                    .active(|style| style.opacity(0.8))
                    .child("+  새 세션")
                    .on_click(cx.listener(Self::open_new_session)),
            )
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> Div {
        div()
            .w(px(264.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(rgb(PANEL_BACKGROUND))
            .border_r_1()
            .border_color(rgb(BORDER))
            .child(
                div().p(px(10.0)).child(
                    div()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(0x40566b))
                        .px(px(12.0))
                        .py(px(9.0))
                        .text_sm()
                        .text_color(rgb(MUTED_TEXT))
                        .child("⌕  서버 검색"),
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(16.0))
                    .py(px(10.0))
                    .text_xs()
                    .text_color(rgb(0x7fa4c3))
                    .child("SAVED SESSIONS")
                    .child(
                        div()
                            .id("sidebar-new-session")
                            .cursor_pointer()
                            .text_lg()
                            .text_color(rgb(MUTED_TEXT))
                            .hover(|style| style.text_color(gpui::white()))
                            .child("+")
                            .on_click(cx.listener(Self::open_new_session)),
                    ),
            )
            .children(self.profiles.iter().enumerate().map(|(index, profile)| {
                let profile_id = profile.id.clone();
                let active = self.selected_profile_id.as_deref() == Some(profile.id.as_str());
                div()
                    .id(("profile", index))
                    .mx(px(10.0))
                    .mb(px(6.0))
                    .cursor_pointer()
                    .rounded_md()
                    .border_1()
                    .border_color(if active { rgb(ACCENT) } else { rgb(BORDER) })
                    .bg(if active { rgb(0x1d4b43) } else { rgb(PANEL_BACKGROUND) })
                    .p(px(12.0))
                    .hover(|style| style.bg(rgb(0x263849)))
                    .child(div().text_sm().text_color(gpui::white()).child(profile.name.clone()))
                    .child(
                        div()
                            .mt(px(4.0))
                            .text_xs()
                            .text_color(rgb(MUTED_TEXT))
                            .child(format!("{}@{}:{}", profile.username, profile.host, profile.port)),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_profile(profile_id.clone(), cx);
                    }))
            }))
            .when(self.profiles.is_empty(), |sidebar| {
                sidebar.child(
                    div()
                        .mx(px(10.0))
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(BORDER))
                        .p(px(14.0))
                        .text_sm()
                        .text_color(rgb(MUTED_TEXT))
                        .child("저장된 세션이 없습니다.\n기존 Tauri 앱에서 세션을 추가하면 여기에도 표시됩니다."),
                )
            })
            .child(
                div()
                    .mt(px(18.0))
                    .px(px(16.0))
                    .py(px(10.0))
                    .text_xs()
                    .text_color(rgb(0x7fa4c3))
                    .child("OPEN SESSIONS"),
            )
            .children(
                self.selected_profile_id
                    .as_ref()
                    .and_then(|id| self.profiles.iter().find(|profile| profile.id == *id))
                    .map(|profile| {
                        div()
                            .mx(px(10.0))
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(ACCENT))
                            .p(px(12.0))
                            .text_sm()
                            .text_color(gpui::white())
                            .child(profile.name.clone())
                    }),
            )
            .child(div().flex_1())
            .child(
                div()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .px(px(16.0))
                    .py(px(14.0))
                    .text_sm()
                    .text_color(rgb(MUTED_TEXT))
                    .child("⌂  홈"),
            )
    }

    fn workspace(&self, cx: &mut Context<Self>) -> Div {
        let selected_profile = self
            .selected_profile_id
            .as_ref()
            .and_then(|id| self.profiles.iter().find(|profile| profile.id == *id));
        let content = match selected_profile {
            Some(profile) => self.directory_browser(profile, cx),
            None => div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(10.0))
                .child(div().text_xl().child("TraceDeck"))
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(MUTED_TEXT))
                        .child("저장된 세션을 선택하거나 새 세션을 만들어 시작하세요."),
                )
                .child(
                    div()
                        .mt(px(12.0))
                        .text_xs()
                        .text_color(rgb(0x6e8497))
                        .child(self.core_status.clone()),
                ),
        };
        let (cpu_label, disk_label) = self.remote_status.as_ref().map_or_else(
            || ("CPU  —".to_owned(), "DISK  —".to_owned()),
            |status| {
                (
                    format!("CPU  {:.0}%", status.cpu_percent),
                    format!(
                        "DISK  {:.1} / {:.1} GiB ({:.0}%)",
                        status.disk_used_kb as f64 / 1_048_576.0,
                        status.disk_total_kb as f64 / 1_048_576.0,
                        status.disk_percent
                    ),
                )
            },
        );

        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(rgb(APP_BACKGROUND))
            .child(
                div()
                    .h(px(52.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .h_full()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .border_t_2()
                            .border_color(rgb(ACCENT))
                            .px(px(16.0))
                            .text_sm()
                            .text_color(gpui::white())
                            .child("▣  Home"),
                    ),
            )
            .child(content)
            .child(
                div()
                    .h(px(28.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(px(18.0))
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .px(px(14.0))
                    .text_xs()
                    .text_color(rgb(MUTED_TEXT))
                    .child(cpu_label)
                    .child(disk_label),
            )
    }

    fn directory_browser(&self, profile: &ConnectionProfile, cx: &mut Context<Self>) -> Div {
        let mut entries = div()
            .id("remote-file-list")
            .flex_1()
            .flex()
            .flex_col()
            .overflow_y_scroll();
        if self.remote_path != "/" {
            entries = entries.child(
                div()
                    .id("parent-directory")
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .px(px(18.0))
                    .py(px(10.0))
                    .text_sm()
                    .text_color(rgb(MUTED_TEXT))
                    .hover(|style| style.bg(rgb(0x1c2935)))
                    .child("⌃  상위 폴더")
                    .on_click(cx.listener(Self::parent_directory)),
            );
        }
        for (index, entry) in self.remote_entries.iter().enumerate() {
            let entry_path = entry.path.clone();
            let entry_name = entry.name.clone();
            let details = entry.details.clone();
            let is_directory = entry.is_dir;
            let row = div()
                .id(("remote-entry", index))
                .flex()
                .items_center()
                .gap(px(10.0))
                .border_b_1()
                .border_color(rgb(0x20303e))
                .px(px(18.0))
                .py(px(10.0))
                .text_sm()
                .hover(|style| style.bg(rgb(0x1c2935)))
                .child(if is_directory { "▸" } else { "□" })
                .child(div().text_color(gpui::white()).child(entry_name))
                .child(
                    div()
                        .ml_auto()
                        .text_xs()
                        .text_color(rgb(MUTED_TEXT))
                        .child(details),
                );
            entries =
                if is_directory {
                    entries.child(row.cursor_pointer().on_click(cx.listener(
                        move |this, _, _, cx| {
                            this.open_directory(entry_path.clone(), cx);
                        },
                    )))
                } else {
                    entries.child(row.cursor_pointer().on_click(cx.listener(
                        move |this, _, _, cx| {
                            this.open_log(entry_path.clone(), cx);
                        },
                    )))
                };
        }

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.0))
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .px(px(18.0))
                    .py(px(12.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(gpui::white())
                                    .child(profile.name.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(MUTED_TEXT))
                                    .child(self.remote_path.clone()),
                            ),
                    )
                    .child(
                        div()
                            .id("refresh-directory")
                            .cursor_pointer()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x40566b))
                            .px(px(10.0))
                            .py(px(6.0))
                            .text_sm()
                            .hover(|style| style.bg(rgb(0x263849)))
                            .child("새로고침")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let path = this.remote_path.clone();
                                this.open_directory(path, cx);
                            })),
                    ),
            )
            .children(self.notice.clone().map(|notice| {
                div()
                    .mx(px(18.0))
                    .mt(px(12.0))
                    .rounded_md()
                    .bg(rgb(0x442d2d))
                    .p(px(10.0))
                    .text_sm()
                    .text_color(rgb(0xffb4b4))
                    .child(notice)
            }))
            .child(entries)
            .child(self.log_panel())
            .child(self.terminal_panel(cx))
    }

    fn log_panel(&self) -> Div {
        let title = self.active_log_path.as_deref().map_or(
            "파일을 선택하면 최근 로그와 실시간 tail을 표시합니다.",
            |path| path,
        );
        let content = if self.log_content.is_empty() {
            "로그 내용이 없습니다.".to_owned()
        } else {
            self.log_content.clone()
        };
        div()
            .h(px(220.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(rgb(BORDER))
            .bg(rgb(0x0d141b))
            .child(
                div()
                    .flex_shrink_0()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .px(px(14.0))
                    .py(px(8.0))
                    .text_sm()
                    .text_color(rgb(MUTED_TEXT))
                    .child(title.to_owned()),
            )
            .child(
                div()
                    .id("log-content")
                    .flex_1()
                    .overflow_y_scroll()
                    .p(px(12.0))
                    .text_xs()
                    .text_color(rgb(0xc8d7e5))
                    .child(content),
            )
    }

    fn terminal_panel(&self, cx: &mut Context<Self>) -> Div {
        let active_tab = self
            .active_terminal_id
            .and_then(|id| self.terminal_tabs.iter().find(|tab| tab.terminal_id == id));
        let title = active_tab.map_or_else(
            || "터미널을 열어 SSH 명령을 입력하세요.".to_owned(),
            |tab| tab.title.clone(),
        );
        let content = active_tab.map_or_else(
            || "세션을 선택한 뒤 터미널 열기를 누르세요.".to_owned(),
            |tab| tab.parser.screen().contents(),
        );
        div()
            .h(px(220.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(rgb(BORDER))
            .bg(rgb(0x090d12))
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .px(px(14.0))
                    .py(px(8.0))
                    .child(div().text_sm().text_color(rgb(MUTED_TEXT)).child(title))
                    .child(
                        div()
                            .id("open-terminal")
                            .cursor_pointer()
                            .rounded_md()
                            .bg(rgb(0x267f67))
                            .px(px(10.0))
                            .py(px(6.0))
                            .text_sm()
                            .text_color(gpui::white())
                            .hover(|style| style.bg(rgb(ACCENT)))
                            .child("+ 터미널")
                            .on_click(cx.listener(Self::open_terminal)),
                    ),
            )
            .child(
                div()
                    .id("terminal-content")
                    .track_focus(&self.terminal_focus)
                    .flex_1()
                    .overflow_y_scroll()
                    .cursor_text()
                    .p(px(12.0))
                    .text_xs()
                    .text_color(rgb(0xd6e1ea))
                    .on_click(cx.listener(Self::focus_terminal))
                    .on_key_down(cx.listener(Self::send_terminal_key))
                    .child(content),
            )
    }

    fn new_session_dialog(&self, cx: &mut Context<Self>) -> Div {
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::black().opacity(0.7))
            .child(
                div()
                    .w(px(520.0))
                    .rounded_lg()
                    .bg(rgb(PANEL_BACKGROUND))
                    .border_1()
                    .border_color(rgb(0x40566b))
                    .p(px(22.0))
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("새 세션"),
                    )
                    .child(
                        div().text_sm().text_color(rgb(MUTED_TEXT)).child(
                            "세션 이름, 호스트, 포트, 계정과 인증 방식을 입력하는 화면입니다.",
                        ),
                    )
                    .child(self.form_field("세션 이름", "예: 운영 로그 서버"))
                    .child(self.form_field("호스트", "예: server.example.com"))
                    .child(
                        div()
                            .flex()
                            .gap(px(12.0))
                            .child(self.form_field("포트", "22").flex_1())
                            .child(self.form_field("계정", "사용자 이름").flex_1()),
                    )
                    .child(
                        div()
                            .rounded_md()
                            .bg(rgb(0x1c2935))
                            .px(px(12.0))
                            .py(px(10.0))
                            .text_xs()
                            .text_color(rgb(MUTED_TEXT))
                            .child("입력 및 ProfileStore 저장은 3단계에서 연결됩니다."),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .id("cancel-new-session")
                                    .cursor_pointer()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(0x40566b))
                                    .px(px(14.0))
                                    .py(px(8.0))
                                    .text_sm()
                                    .hover(|style| style.bg(rgb(0x263849)))
                                    .child("취소")
                                    .on_click(cx.listener(Self::close_new_session)),
                            )
                            .child(
                                div()
                                    .rounded_md()
                                    .bg(rgb(0x31564e))
                                    .px(px(14.0))
                                    .py(px(8.0))
                                    .text_sm()
                                    .text_color(rgb(0xa2c8bf))
                                    .child("저장하고 열기"),
                            ),
                    ),
            )
    }

    fn form_field(&self, label: &str, placeholder: &str) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(MUTED_TEXT))
                    .child(label.to_owned()),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x40566b))
                    .px(px(10.0))
                    .py(px(9.0))
                    .text_sm()
                    .text_color(rgb(0x6e8497))
                    .child(placeholder.to_owned()),
            )
    }
}

impl Render for TraceDeckGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(rgb(APP_BACKGROUND))
            .text_color(rgb(0xe5edf5))
            .child(self.header(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.sidebar(cx))
                    .child(self.workspace(cx)),
            );

        if self.show_new_session_dialog {
            root.child(self.new_session_dialog(cx))
        } else {
            root
        }
    }
}

struct InitialState {
    core_status: String,
    profiles: Vec<ConnectionProfile>,
    known_hosts_file: std::path::PathBuf,
}

fn initialize_core() -> InitialState {
    match AppPaths::for_current_user().and_then(|paths| {
        paths.ensure_exists()?;
        Ok(paths)
    }) {
        Ok(paths) => {
            let store = ProfileStore::new(paths.profile_file());
            match store.list() {
                Ok(profiles) => InitialState {
                    core_status: format!("저장 세션 {}개를 불러왔습니다.", profiles.len()),
                    profiles,
                    known_hosts_file: paths.known_hosts_file(),
                },
                Err(error) => InitialState {
                    core_status: format!("저장 세션을 읽지 못했습니다: {error}"),
                    profiles: Vec::new(),
                    known_hosts_file: paths.known_hosts_file(),
                },
            }
        }
        Err(error) => InitialState {
            core_status: format!("trace-deck-core 초기화 실패: {error}"),
            profiles: Vec::new(),
            known_hosts_file: std::path::PathBuf::new(),
        },
    }
}

fn main() {
    let initial_state = initialize_core();
    Application::new().run(move |cx: &mut App| {
        let (stream_sender, stream_receiver) = async_channel::unbounded();
        let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let terminal_focus = cx.focus_handle();
                let view = cx.new(|_| TraceDeckGpui {
                    core_status: initial_state.core_status.clone(),
                    profiles: initial_state.profiles.clone(),
                    selected_profile_id: None,
                    remote_path: "/".to_owned(),
                    remote_entries: Vec::new(),
                    known_hosts_file: initial_state.known_hosts_file.clone(),
                    notice: None,
                    active_log_path: None,
                    log_content: String::new(),
                    tail_stream_id: None,
                    system_stream_id: None,
                    remote_status: None,
                    tail_streams: Arc::new(logs::TailStreams::new()),
                    system_streams: Arc::new(system::SystemStreams::new()),
                    stream_sender: stream_sender.clone(),
                    terminal_sessions: terminal::TerminalSessions::new(),
                    terminal_tabs: Vec::new(),
                    active_terminal_id: None,
                    terminal_focus,
                    show_new_session_dialog: false,
                });
                let event_view = view.clone();
                window
                    .spawn(cx, async move |cx| {
                        while let Ok(event) = stream_receiver.recv().await {
                            let _ = event_view.update(cx, |this, cx| {
                                this.receive_stream_event(event, cx);
                            });
                        }
                    })
                    .detach();
                view
            },
        )
        .expect("TraceDeck GPUI 창을 열 수 없습니다.");
        cx.activate(true);
    });
}
