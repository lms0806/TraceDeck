use std::num::NonZeroU16;
use std::ops::Range;
use std::sync::Arc;

use async_channel::Sender;
use gpui::{
    App, Application, Bounds, Context, Div, FocusHandle, HighlightStyle, IntoElement, MouseButton,
    Render, ScrollHandle, StatefulInteractiveElement, StyledText, Window, WindowBounds,
    WindowOptions, div, prelude::*, px, rgb, size,
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
    profile_file: std::path::PathBuf,
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
    sidebar_scroll: ScrollHandle,
    file_list_scroll: ScrollHandle,
    log_scroll: ScrollHandle,
    terminal_scroll: ScrollHandle,
    terminal_focus: FocusHandle,
    session_form_focus: FocusHandle,
    server_search_focus: FocusHandle,
    server_search: String,
    session_draft: SessionDraft,
    active_draft_field: DraftField,
    active_bottom_panel: BottomPanel,
    bottom_panel_height: f32,
    show_new_session_dialog: bool,
    file_action: Option<FileAction>,
    file_action_name: String,
    file_action_focus: FocusHandle,
    context_menu: Option<ContextMenu>,
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

#[derive(Clone)]
struct ContextMenu {
    entry: RemoteEntry,
    x: f32,
    y: f32,
}

struct ResizeDragPreview;

impl Render for ResizeDragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size(px(1.0))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DraftField {
    Name,
    Host,
    Port,
    Username,
    PrivateKey,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BottomPanel {
    Log,
    Terminal,
}

#[derive(Clone)]
enum FileAction {
    Rename(RemoteEntry),
    Delete(RemoteEntry),
}

struct SessionDraft {
    name: String,
    host: String,
    port: String,
    username: String,
    private_key_path: String,
}

impl Default for SessionDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            host: String::new(),
            port: "22".to_owned(),
            username: String::new(),
            private_key_path: String::new(),
        }
    }
}

impl TraceDeckGpui {
    fn open_new_session(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_new_session_dialog = true;
        self.session_draft = SessionDraft::default();
        self.active_draft_field = DraftField::Name;
        window.focus(&self.session_form_focus);
        cx.notify();
    }

    fn close_new_session(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.show_new_session_dialog = false;
        cx.notify();
    }

    fn select_log_panel(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.active_bottom_panel = BottomPanel::Log;
        cx.notify();
    }

    fn select_terminal_panel(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_bottom_panel = BottomPanel::Terminal;
        cx.notify();
    }

    fn resize_bottom_panel(
        &mut self,
        event: &gpui::DragMoveEvent<()>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let window_height = f32::from(window.bounds().size.height);
        let pointer_y = f32::from(event.event.position.y);
        self.bottom_panel_height = (window_height - pointer_y).clamp(160.0, 600.0);
        cx.notify();
    }

    fn refresh_scrollbar(
        &mut self,
        _: &gpui::ScrollWheelEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.notify();
    }

    fn scrollbar_thumb(&self, id: &'static str, handle: &ScrollHandle) -> gpui::Stateful<Div> {
        let viewport_height = f32::from(handle.bounds().size.height);
        let max_offset = f32::from(handle.max_offset().height);
        let offset = -f32::from(handle.offset().y);
        if viewport_height <= 0.0 || max_offset <= 0.0 {
            return div().id(id).absolute().w(px(0.0)).h(px(0.0));
        }
        let thumb_height = (viewport_height * viewport_height / (viewport_height + max_offset))
            .clamp(24.0, (viewport_height - 8.0).max(24.0));
        let travel = (viewport_height - thumb_height - 8.0).max(0.0);
        let top = 4.0 + (offset / max_offset).clamp(0.0, 1.0) * travel;

        div()
            .id(id)
            .absolute()
            .top(px(top))
            .right(px(3.0))
            .w(px(5.0))
            .h(px(thumb_height))
            .rounded_lg()
            .bg(rgb(0x61798d))
    }

    fn activate_draft_field(
        &mut self,
        field: DraftField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_draft_field = field;
        window.focus(&self.session_form_focus);
        cx.notify();
    }

    fn activate_server_search(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.server_search_focus);
        cx.notify();
    }

    fn edit_server_search(
        &mut self,
        event: &gpui::KeyDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event.keystroke.key.as_str() {
            "backspace" => {
                self.server_search.pop();
            }
            "escape" => self.server_search.clear(),
            "space" => self.server_search.push(' '),
            character if character.chars().count() == 1 => self.server_search.push_str(character),
            _ => return,
        }
        cx.notify();
    }

    fn pick_private_key(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("SSH 개인키 파일 선택")
            .add_filter("개인키", &["pem", "ppk", "key"])
            .add_filter("모든 파일", &["*"])
            .pick_file()
        {
            self.session_draft.private_key_path = path.to_string_lossy().into_owned();
            self.active_draft_field = DraftField::PrivateKey;
            window.focus(&self.session_form_focus);
            cx.notify();
        }
    }

    fn edit_draft(
        &mut self,
        event: &gpui::KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        if key == "enter" {
            self.persist_new_session(cx);
            return;
        }
        let value = match self.active_draft_field {
            DraftField::Name => &mut self.session_draft.name,
            DraftField::Host => &mut self.session_draft.host,
            DraftField::Port => &mut self.session_draft.port,
            DraftField::Username => &mut self.session_draft.username,
            DraftField::PrivateKey => &mut self.session_draft.private_key_path,
        };
        match key {
            "backspace" => {
                value.pop();
            }
            "space" => value.push(' '),
            character if character.chars().count() == 1 => value.push_str(character),
            _ => return,
        }
        cx.notify();
    }

    fn save_new_session(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.persist_new_session(cx);
    }

    fn persist_new_session(&mut self, cx: &mut Context<Self>) {
        let draft = &self.session_draft;
        let port = match draft.port.parse::<u16>().ok().filter(|port| *port > 0) {
            Some(port) => port,
            None => {
                self.notice = Some("포트는 1~65535 사이의 숫자로 입력하세요.".to_owned());
                cx.notify();
                return;
            }
        };
        if draft.name.trim().is_empty()
            || draft.host.trim().is_empty()
            || draft.username.trim().is_empty()
        {
            self.notice = Some("세션 이름, 호스트, 계정은 필수입니다.".to_owned());
            cx.notify();
            return;
        }
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis());
        let profile = ConnectionProfile {
            id: format!("gpui-{unique}"),
            name: draft.name.trim().to_owned(),
            host: draft.host.trim().to_owned(),
            port,
            username: draft.username.trim().to_owned(),
            group: "Saved Sessions".to_owned(),
            auth_kind: if draft.private_key_path.trim().is_empty() {
                "agent".to_owned()
            } else {
                "privateKey".to_owned()
            },
            private_key_path: (!draft.private_key_path.trim().is_empty())
                .then(|| draft.private_key_path.trim().to_owned()),
        };
        match ProfileStore::new(self.profile_file.clone()).save(profile.clone()) {
            Ok(()) => {
                self.profiles.push(profile.clone());
                self.show_new_session_dialog = false;
                self.session_draft = SessionDraft::default();
                self.notice = None;
                self.select_profile(profile.id, cx);
            }
            Err(error) => {
                self.notice = Some(format!("세션을 저장하지 못했습니다: {error}"));
                cx.notify();
            }
        }
    }

    fn select_profile(&mut self, profile_id: String, cx: &mut Context<Self>) {
        self.stop_streams();
        self.context_menu = None;
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

    fn close_session(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.stop_streams();
        self.selected_profile_id = None;
        self.remote_entries.clear();
        self.active_log_path = None;
        self.log_content.clear();
        self.remote_status = None;
        self.context_menu = None;
        cx.notify();
    }

    fn delete_profile(
        &mut self,
        profile_id: String,
        _: &gpui::ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match ProfileStore::new(self.profile_file.clone()).delete(&profile_id) {
            Ok(()) => {
                if self.selected_profile_id.as_deref() == Some(profile_id.as_str()) {
                    self.stop_streams();
                    self.selected_profile_id = None;
                    self.remote_entries.clear();
                }
                self.profiles.retain(|profile| profile.id != profile_id);
            }
            Err(error) => self.notice = Some(error),
        }
        cx.notify();
    }

    fn begin_file_action(
        &mut self,
        action: FileAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.file_action_name = match &action {
            FileAction::Rename(entry) => entry.name.clone(),
            _ => String::new(),
        };
        self.file_action = Some(action);
        self.context_menu = None;
        window.focus(&self.file_action_focus);
        cx.notify();
    }

    fn open_entry_context(
        &mut self,
        entry: RemoteEntry,
        event: &gpui::MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Right {
            self.context_menu = Some(ContextMenu {
                entry,
                x: f32::from(event.position.x),
                y: f32::from(event.position.y),
            });
            cx.notify();
        }
    }

    fn dismiss_context_menu(
        &mut self,
        _: &gpui::MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.context_menu.take().is_some() {
            cx.notify();
        }
    }

    fn edit_file_action(
        &mut self,
        event: &gpui::KeyDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event.keystroke.key.as_str() {
            "enter" => self.confirm_file_action(cx),
            "escape" => {
                self.file_action = None;
                cx.notify();
            }
            "backspace" => {
                self.file_action_name.pop();
                cx.notify();
            }
            "space" => {
                self.file_action_name.push(' ');
                cx.notify();
            }
            character if character.chars().count() == 1 => {
                self.file_action_name.push_str(character);
                cx.notify();
            }
            _ => {}
        }
    }

    fn confirm_file_action(&mut self, cx: &mut Context<Self>) {
        let Some(profile) = self
            .selected_profile_id
            .as_ref()
            .and_then(|id| self.profiles.iter().find(|profile| profile.id == *id))
            .cloned()
        else {
            return;
        };
        let Some(action) = self.file_action.clone() else {
            return;
        };
        let result = match action {
            FileAction::Rename(entry) => sftp::rename_entry(
                &profile,
                &entry.path,
                &self.file_action_name,
                &self.known_hosts_file,
            ),
            FileAction::Delete(entry) => {
                sftp::delete_entry(&profile, &entry.path, entry.is_dir, &self.known_hosts_file)
            }
        };
        match result {
            Ok(()) => {
                self.file_action = None;
                self.load_directory(self.remote_path.clone(), cx);
            }
            Err(error) => {
                self.notice = Some(error);
                cx.notify();
            }
        }
    }

    fn cancel_file_action(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.file_action = None;
        self.file_action_name.clear();
        cx.notify();
    }

    fn load_directory(&mut self, path: String, cx: &mut Context<Self>) {
        self.context_menu = None;
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
                self.log_content = normalize_newlines(file.content);
                self.log_scroll.scroll_to_bottom();
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

    fn open_terminal(&mut self, _: &gpui::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
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
                window.focus(&self.terminal_focus);
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
            "enter" | "return" => "\r".to_owned(),
            "backspace" => "\u{7f}".to_owned(),
            "tab" => "\t".to_owned(),
            "escape" => "\u{1b}".to_owned(),
            "up" => "\u{1b}[A".to_owned(),
            "down" => "\u{1b}[B".to_owned(),
            "right" => "\u{1b}[C".to_owned(),
            "left" => "\u{1b}[D".to_owned(),
            "space" => " ".to_owned(),
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

    fn clear_terminal(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(terminal_id) = self.active_terminal_id
            && let Some(tab) = self
                .terminal_tabs
                .iter_mut()
                .find(|tab| tab.terminal_id == terminal_id)
        {
            tab.parser = vt100::Parser::new(
                NonZeroU16::new(30).expect("30 is non-zero"),
                NonZeroU16::new(120).expect("120 is non-zero"),
                5_000,
            );
            cx.notify();
        }
    }

    fn receive_stream_event(&mut self, event: StreamEvent, cx: &mut Context<Self>) {
        match event {
            StreamEvent::Tail(chunk) if Some(chunk.stream_id) == self.tail_stream_id => {
                self.log_content.push_str(&normalize_newlines(chunk.chunk));
                self.log_scroll.scroll_to_bottom();
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

    fn sidebar(&self, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let search_query = self.server_search.trim().to_lowercase();
        let matching_profiles = self
            .profiles
            .iter()
            .filter(|profile| profile_matches_search(profile, &search_query))
            .collect::<Vec<_>>();
        let search_label = if self.server_search.is_empty() {
            "⌕  서버 검색".to_owned()
        } else {
            format!("⌕  {}", self.server_search)
        };
        div()
            .id("sidebar")
            .w(px(264.0))
            .flex_shrink_0()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&self.sidebar_scroll)
            .on_scroll_wheel(cx.listener(Self::refresh_scrollbar))
            .bg(rgb(PANEL_BACKGROUND))
            .border_r_1()
            .border_color(rgb(BORDER))
            .child(
                div().p(px(10.0)).child(
                    div()
                        .id("server-search")
                        .track_focus(&self.server_search_focus)
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(0x40566b))
                        .px(px(12.0))
                        .py(px(9.0))
                        .text_sm()
                        .text_color(if self.server_search.is_empty() {
                            rgb(MUTED_TEXT)
                        } else {
                            rgb(0xffffff)
                        })
                        .cursor_text()
                        .child(search_label)
                        .on_click(cx.listener(Self::activate_server_search))
                        .on_key_down(cx.listener(Self::edit_server_search)),
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
            .children(matching_profiles.iter().enumerate().map(|(index, profile)| {
                let profile_id = profile.id.clone();
                let delete_profile_id = profile.id.clone();
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
                    .child(
                        div().flex().items_center().justify_between()
                            .child(div().text_sm().text_color(gpui::white()).child(profile.name.clone()))
                            .child(div().id(("delete-profile", index)).cursor_pointer().px(px(4.0)).text_color(rgb(MUTED_TEXT)).hover(|style| style.text_color(rgb(0xff8f8f))).child("×").on_click(cx.listener(move |this, event, window, cx| { cx.stop_propagation(); this.delete_profile(delete_profile_id.clone(), event, window, cx) }))),
                    )
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
            .when(!self.profiles.is_empty() && !self.server_search.is_empty() && matching_profiles.is_empty(), |sidebar| {
                sidebar.child(
                    div()
                        .mx(px(10.0))
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(BORDER))
                        .p(px(14.0))
                        .text_sm()
                        .text_color(rgb(MUTED_TEXT))
                        .child("일치하는 저장 세션이 없습니다."),
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
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_sm().text_color(gpui::white()).child(profile.name.clone()))
                            .child(div().id("close-session").cursor_pointer().px(px(4.0)).text_color(rgb(MUTED_TEXT)).hover(|style| style.text_color(gpui::white())).child("×").on_click(cx.listener(Self::close_session)))
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
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_scroll()
            .track_scroll(&self.file_list_scroll)
            .on_scroll_wheel(cx.listener(Self::refresh_scrollbar));
        entries = entries.child(
            div()
                .flex()
                .items_center()
                .border_b_1()
                .border_color(rgb(0x20303e))
                .px(px(18.0))
                .py(px(9.0))
                .text_xs()
                .text_color(rgb(0x71879a))
                .child(div().flex_1().child("이름"))
                .child(metadata_columns([
                    "Access".into(),
                    "Owner".into(),
                    "Group".into(),
                    "Size".into(),
                    "Last modified".into(),
                ])),
        );
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
            let entry_for_context = entry.clone();
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
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, event, window, cx| {
                        this.open_entry_context(entry_for_context.clone(), event, window, cx)
                    }),
                )
                .hover(|style| style.bg(rgb(0x1c2935)))
                .child(entry_icon(is_directory))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_color(gpui::white())
                        .child(entry_name),
                )
                .child(metadata_columns(entry_metadata(&details).values()));
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
            .child(self.bottom_panel_resizer(cx))
            .child(self.bottom_panel(cx))
    }

    fn bottom_panel_resizer(&self, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        div()
            .id("bottom-panel-resizer")
            .h(px(6.0))
            .flex_shrink_0()
            .bg(rgb(BORDER))
            .hover(|style| style.bg(rgb(ACCENT)))
            .on_drag((), |_, _, _, cx| cx.new(|_| ResizeDragPreview))
            .on_drag_move(cx.listener(Self::resize_bottom_panel))
    }

    fn bottom_panel(&self, cx: &mut Context<Self>) -> Div {
        let log_active = self.active_bottom_panel == BottomPanel::Log;
        div()
            .h(px(self.bottom_panel_height))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(rgb(BORDER))
            .bg(rgb(0x0d141b))
            .child(
                div()
                    .h(px(34.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .id("bottom-log-tab")
                            .cursor_pointer()
                            .h_full()
                            .flex()
                            .items_center()
                            .border_t_2()
                            .border_color(if log_active {
                                rgb(ACCENT)
                            } else {
                                rgb(0x0d141b)
                            })
                            .px(px(14.0))
                            .text_sm()
                            .text_color(if log_active {
                                rgb(0xffffff)
                            } else {
                                rgb(MUTED_TEXT)
                            })
                            .child("로그")
                            .on_click(cx.listener(Self::select_log_panel)),
                    )
                    .child(
                        div()
                            .id("bottom-terminal-tab")
                            .cursor_pointer()
                            .h_full()
                            .flex()
                            .items_center()
                            .border_t_2()
                            .border_color(if log_active {
                                rgb(0x0d141b)
                            } else {
                                rgb(ACCENT)
                            })
                            .px(px(14.0))
                            .text_sm()
                            .text_color(if log_active {
                                rgb(MUTED_TEXT)
                            } else {
                                rgb(0xffffff)
                            })
                            .child("터미널")
                            .on_click(cx.listener(Self::select_terminal_panel)),
                    ),
            )
            .child(if log_active {
                self.log_panel(cx)
            } else {
                self.terminal_panel(cx)
            })
    }

    fn log_panel(&self, cx: &mut Context<Self>) -> Div {
        let title = self.active_log_path.as_deref().map_or(
            "파일을 선택하면 최근 로그와 실시간 tail을 표시합니다.",
            |path| path,
        );
        let display = if self.log_content.is_empty() {
            LogDisplay::plain("로그 내용이 없습니다.".to_owned())
        } else {
            parse_log_content(&self.log_content)
        };
        div()
            .flex_1()
            .min_h_0()
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
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .id("log-content")
                            .size_full()
                            .overflow_scroll()
                            .track_scroll(&self.log_scroll)
                            .on_scroll_wheel(cx.listener(Self::refresh_scrollbar))
                            .p(px(12.0))
                            .text_size(px(12.5))
                            .text_color(rgb(0xc8d7e5))
                            .child(StyledText::new(display.text).with_highlights(
                                display.highlights.into_iter().map(|highlight| {
                                    (
                                        highlight.range,
                                        HighlightStyle {
                                            color: Some(rgb(highlight.color).into()),
                                            ..Default::default()
                                        },
                                    )
                                }),
                            )),
                    )
                    .child(self.scrollbar_thumb("log", &self.log_scroll)),
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
            .flex_1()
            .min_h_0()
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
                            .flex()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .id("clear-terminal")
                                    .cursor_pointer()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(0x40566b))
                                    .px(px(10.0))
                                    .py(px(6.0))
                                    .text_sm()
                                    .hover(|style| style.bg(rgb(0x263849)))
                                    .child("출력 지우기")
                                    .on_click(cx.listener(Self::clear_terminal)),
                            )
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
                    ),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .id("terminal-content")
                            .size_full()
                            .track_focus(&self.terminal_focus)
                            .overflow_scroll()
                            .track_scroll(&self.terminal_scroll)
                            .on_scroll_wheel(cx.listener(Self::refresh_scrollbar))
                            .cursor_text()
                            .p(px(12.0))
                            .text_xs()
                            .text_color(rgb(0xd6e1ea))
                            .on_click(cx.listener(Self::focus_terminal))
                            .on_key_down(cx.listener(Self::send_terminal_key))
                            .child(content),
                    )
                    .child(self.scrollbar_thumb("terminal", &self.terminal_scroll)),
            )
    }

    fn file_action_dialog(&self, cx: &mut Context<Self>) -> Div {
        let action = self
            .file_action
            .as_ref()
            .expect("dialog requires an action");
        let (title, needs_name, confirm_label) = match action {
            FileAction::Rename(_) => ("이름 변경", true, "변경"),
            FileAction::Delete(_) => ("삭제 확인", false, "삭제"),
        };
        let description = match action {
            FileAction::Delete(entry) => format!(
                "‘{}’을(를) 삭제합니다. 이 작업은 되돌릴 수 없습니다.",
                entry.name
            ),
            _ => "이름에는 경로 구분자(/, \\)를 사용할 수 없습니다.".to_owned(),
        };
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::black().opacity(0.7))
            .child(
                div()
                    .w(px(420.0))
                    .rounded_lg()
                    .bg(rgb(PANEL_BACKGROUND))
                    .border_1()
                    .border_color(rgb(0x40566b))
                    .p(px(22.0))
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    .track_focus(&self.file_action_focus)
                    .on_key_down(cx.listener(Self::edit_file_action))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED_TEXT))
                            .child(description),
                    )
                    .when(needs_name, |dialog| {
                        dialog.child(
                            div()
                                .id("file-action-name")
                                .cursor_text()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(ACCENT))
                                .px(px(10.0))
                                .py(px(9.0))
                                .text_sm()
                                .child(if self.file_action_name.is_empty() {
                                    "이름 입력".to_owned()
                                } else {
                                    self.file_action_name.clone()
                                }),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .id("cancel-file-action")
                                    .cursor_pointer()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(0x40566b))
                                    .px(px(14.0))
                                    .py(px(8.0))
                                    .text_sm()
                                    .child("취소")
                                    .on_click(cx.listener(Self::cancel_file_action)),
                            )
                            .child(
                                div()
                                    .id("confirm-file-action")
                                    .cursor_pointer()
                                    .rounded_md()
                                    .bg(if matches!(action, FileAction::Delete(_)) {
                                        rgb(0xa64242)
                                    } else {
                                        rgb(0x267f67)
                                    })
                                    .px(px(14.0))
                                    .py(px(8.0))
                                    .text_sm()
                                    .text_color(gpui::white())
                                    .child(confirm_label)
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.confirm_file_action(cx)),
                                    ),
                            ),
                    ),
            )
    }

    fn context_menu(&self, cx: &mut Context<Self>) -> Div {
        let menu = self.context_menu.as_ref().expect("menu exists");
        let rename_entry = menu.entry.clone();
        let delete_entry = menu.entry.clone();
        div()
            .absolute()
            .left(px(menu.x))
            .top(px(menu.y))
            .w(px(138.0))
            .rounded_md()
            .border_1()
            .border_color(rgb(0x40566b))
            .bg(rgb(0x1a2631))
            .p(px(5.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.stop_propagation()),
            )
            .child(
                div()
                    .id("context-menu-rename")
                    .cursor_pointer()
                    .rounded_sm()
                    .px(px(10.0))
                    .py(px(8.0))
                    .text_sm()
                    .hover(|style| style.bg(rgb(0x294052)))
                    .child("이름 변경")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.begin_file_action(FileAction::Rename(rename_entry.clone()), window, cx)
                    })),
            )
            .child(
                div()
                    .id("context-menu-delete")
                    .cursor_pointer()
                    .rounded_sm()
                    .px(px(10.0))
                    .py(px(8.0))
                    .text_sm()
                    .text_color(rgb(0xff8f8f))
                    .hover(|style| style.bg(rgb(0x41272a)))
                    .child("삭제")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.begin_file_action(FileAction::Delete(delete_entry.clone()), window, cx)
                    })),
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
                    .id("new-session-form")
                    .track_focus(&self.session_form_focus)
                    .on_key_down(cx.listener(Self::edit_draft))
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
                    .child(self.form_field(DraftField::Name, "세션 이름", "예: 운영 로그 서버", cx))
                    .child(self.form_field(DraftField::Host, "호스트", "예: server.example.com", cx))
                    .child(
                        div()
                            .flex()
                            .gap(px(12.0))
                            .child(self.form_field(DraftField::Port, "포트", "22", cx).flex_1())
                            .child(self.form_field(DraftField::Username, "계정", "사용자 이름", cx).flex_1()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_end()
                            .gap(px(10.0))
                            .child(
                                self.form_field(
                                    DraftField::PrivateKey,
                                    "개인키 경로 (선택)",
                                    "비우면 SSH agent 사용",
                                    cx,
                                )
                                .flex_1(),
                            )
                            .child(
                                div()
                                    .id("pick-private-key")
                                    .cursor_pointer()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(0x40566b))
                                    .px(px(12.0))
                                    .py(px(9.0))
                                    .text_sm()
                                    .hover(|style| style.bg(rgb(0x263849)))
                                    .child("파일 선택")
                                    .on_click(cx.listener(Self::pick_private_key)),
                            ),
                    )
                    .child(
                        div()
                            .rounded_md()
                            .bg(rgb(0x1c2935))
                            .px(px(12.0))
                            .py(px(10.0))
                            .text_xs()
                            .text_color(rgb(MUTED_TEXT))
                            .child("입력 후 저장하고 열기를 누르면 ProfileStore에 저장하고 SFTP 연결을 시작합니다."),
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
                                    .id("save-new-session")
                                    .cursor_pointer()
                                    .rounded_md()
                                    .bg(rgb(0x267f67))
                                    .px(px(14.0))
                                    .py(px(8.0))
                                    .text_sm()
                                    .text_color(gpui::white())
                                    .hover(|style| style.bg(rgb(ACCENT)))
                                    .child("저장하고 열기")
                                    .on_click(cx.listener(Self::save_new_session)),
                            ),
                    ),
            )
    }

    fn form_field(
        &self,
        field: DraftField,
        label: &str,
        placeholder: &str,
        cx: &mut Context<Self>,
    ) -> Div {
        let value = match field {
            DraftField::Name => &self.session_draft.name,
            DraftField::Host => &self.session_draft.host,
            DraftField::Port => &self.session_draft.port,
            DraftField::Username => &self.session_draft.username,
            DraftField::PrivateKey => &self.session_draft.private_key_path,
        };
        let active = self.active_draft_field == field;
        let id = match field {
            DraftField::Name => "session-name",
            DraftField::Host => "session-host",
            DraftField::Port => "session-port",
            DraftField::Username => "session-username",
            DraftField::PrivateKey => "session-private-key",
        };
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
                    .id(id)
                    .cursor_text()
                    .rounded_md()
                    .border_1()
                    .border_color(if active { rgb(ACCENT) } else { rgb(0x40566b) })
                    .px(px(10.0))
                    .py(px(9.0))
                    .text_sm()
                    .text_color(if value.is_empty() {
                        rgb(0x6e8497)
                    } else {
                        rgb(0xe5edf5)
                    })
                    .child(if value.is_empty() {
                        placeholder.to_owned()
                    } else {
                        value.clone()
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.activate_draft_field(field, window, cx);
                    })),
            )
    }
}

impl Render for TraceDeckGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div()
            .id("app-root")
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(rgb(APP_BACKGROUND))
            .text_color(rgb(0xe5edf5))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::dismiss_context_menu))
            .child(self.header(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.sidebar(cx))
                    .child(self.workspace(cx)),
            );

        let root = if self.show_new_session_dialog {
            root.child(self.new_session_dialog(cx))
        } else {
            root
        };

        let root = if self.file_action.is_some() {
            root.child(self.file_action_dialog(cx))
        } else {
            root
        };
        if self.context_menu.is_some() {
            root.child(self.context_menu(cx))
        } else {
            root
        }
    }
}

struct InitialState {
    core_status: String,
    profiles: Vec<ConnectionProfile>,
    known_hosts_file: std::path::PathBuf,
    profile_file: std::path::PathBuf,
}

fn profile_matches_search(profile: &ConnectionProfile, query: &str) -> bool {
    query.is_empty()
        || [
            profile.name.as_str(),
            profile.host.as_str(),
            profile.username.as_str(),
        ]
        .iter()
        .any(|value| value.to_lowercase().contains(query))
}

struct EntryMetadata {
    access: String,
    owner: String,
    group: String,
    size: String,
    modified: String,
}

impl EntryMetadata {
    fn values(&self) -> [String; 5] {
        [
            self.access.clone(),
            self.owner.clone(),
            self.group.clone(),
            self.size.clone(),
            self.modified.clone(),
        ]
    }
}

fn entry_metadata(details: &str) -> EntryMetadata {
    let columns = details.split_whitespace().collect::<Vec<_>>();
    if columns.len() < 8 {
        return EntryMetadata {
            access: details.into(),
            owner: "—".into(),
            group: "—".into(),
            size: "—".into(),
            modified: "—".into(),
        };
    }
    EntryMetadata {
        access: columns[0].into(),
        owner: columns[2].into(),
        group: columns[3].into(),
        size: columns[4].into(),
        modified: format!("{} {} {}", columns[5], columns[6], columns[7]),
    }
}

fn metadata_columns(values: [String; 5]) -> Div {
    div()
        .w(px(520.0))
        .flex()
        .items_center()
        .text_xs()
        .text_color(rgb(MUTED_TEXT))
        .child(div().w(px(100.0)).child(values[0].clone()))
        .child(div().w(px(72.0)).child(values[1].clone()))
        .child(div().w(px(72.0)).child(values[2].clone()))
        .child(div().w(px(76.0)).child(values[3].clone()))
        .child(div().w(px(190.0)).child(values[4].clone()))
}

fn entry_icon(is_directory: bool) -> Div {
    if is_directory {
        div()
            .relative()
            .w(px(18.0))
            .h(px(16.0))
            .flex_shrink_0()
            .child(
                div()
                    .absolute()
                    .left(px(1.0))
                    .top(px(1.0))
                    .w(px(8.0))
                    .h(px(5.0))
                    .rounded_sm()
                    .bg(rgb(0xe7bc66)),
            )
            .child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(4.0))
                    .w(px(18.0))
                    .h(px(11.0))
                    .rounded_sm()
                    .bg(rgb(0xe7bc66)),
            )
    } else {
        div()
            .relative()
            .w(px(18.0))
            .h(px(16.0))
            .flex_shrink_0()
            .child(
                div()
                    .absolute()
                    .left(px(3.0))
                    .top(px(0.0))
                    .w(px(12.0))
                    .h(px(16.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(0xa9bdcc)),
            )
            .child(
                div()
                    .absolute()
                    .left(px(5.0))
                    .top(px(5.0))
                    .w(px(8.0))
                    .h(px(1.0))
                    .bg(rgb(0xa9bdcc)),
            )
            .child(
                div()
                    .absolute()
                    .left(px(5.0))
                    .top(px(9.0))
                    .w(px(8.0))
                    .h(px(1.0))
                    .bg(rgb(0xa9bdcc)),
            )
    }
}

struct LogDisplay {
    text: String,
    highlights: Vec<LogHighlight>,
}

struct LogHighlight {
    range: Range<usize>,
    color: u32,
}

impl LogDisplay {
    fn plain(text: String) -> Self {
        Self {
            text,
            highlights: Vec::new(),
        }
    }
}

fn normalize_newlines(content: String) -> String {
    content.replace("\r\n", "\n").replace('\r', "\n")
}

fn parse_log_content(content: &str) -> LogDisplay {
    let bytes = content.as_bytes();
    let mut text = String::with_capacity(content.len());
    let mut highlights = Vec::new();
    let mut foreground = None;
    let mut dim = false;
    let mut highlight_start = None;
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == 0x1b {
            if let Some((end, command)) = csi_command(bytes, index) {
                if command == b'm' {
                    let next_style = sgr_style(&bytes[index + 2..end], foreground, dim);
                    let next_color =
                        next_style
                            .0
                            .or(if next_style.1 { Some(MUTED_TEXT) } else { None });
                    let current_color = foreground.or(if dim { Some(MUTED_TEXT) } else { None });
                    if next_color != current_color {
                        finish_log_highlight(
                            &mut highlights,
                            &mut highlight_start,
                            text.len(),
                            current_color,
                        );
                        highlight_start = next_color.map(|_| text.len());
                    }
                    foreground = next_style.0;
                    dim = next_style.1;
                }
                index = end + 1;
                continue;
            }

            if bytes.get(index + 1) == Some(&b']') {
                index = skip_osc(bytes, index + 2);
                continue;
            }

            index += 1;
            continue;
        }

        let remaining = &content[index..];
        let character = remaining
            .chars()
            .next()
            .expect("the byte index always points to a character boundary");
        text.push(character);
        index += character.len_utf8();
    }

    let current_color = foreground.or(if dim { Some(MUTED_TEXT) } else { None });
    finish_log_highlight(
        &mut highlights,
        &mut highlight_start,
        text.len(),
        current_color,
    );

    LogDisplay { text, highlights }
}

fn csi_command(bytes: &[u8], start: usize) -> Option<(usize, u8)> {
    (bytes.get(start + 1) == Some(&b'[')).then_some(())?;
    let end = (start + 2..bytes.len()).find(|&index| (b'@'..=b'~').contains(&bytes[index]))?;
    Some((end, bytes[end]))
}

fn skip_osc(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() {
        if bytes[index] == 0x07 {
            return index + 1;
        }
        if bytes[index] == 0x1b && bytes.get(index + 1) == Some(&b'\\') {
            return index + 2;
        }
        index += 1;
    }
    index
}

fn finish_log_highlight(
    highlights: &mut Vec<LogHighlight>,
    start: &mut Option<usize>,
    end: usize,
    color: Option<u32>,
) {
    if let (Some(start), Some(color)) = (start.take(), color)
        && start < end
    {
        highlights.push(LogHighlight {
            range: start..end,
            color,
        });
    }
}

fn sgr_style(parameters: &[u8], mut foreground: Option<u32>, mut dim: bool) -> (Option<u32>, bool) {
    let values = std::str::from_utf8(parameters)
        .unwrap_or_default()
        .split(';')
        .map(|value| value.parse::<u16>().unwrap_or(0))
        .collect::<Vec<_>>();
    let mut index = 0;

    while index < values.len().max(1) {
        let value = values.get(index).copied().unwrap_or(0);
        match value {
            0 => {
                foreground = None;
                dim = false;
            }
            2 => dim = true,
            22 => dim = false,
            30..=37 | 90..=97 => foreground = ansi_standard_color(value),
            39 => foreground = None,
            38 if values.get(index + 1) == Some(&5) => {
                if let Some(color) = values.get(index + 2) {
                    foreground = Some(ansi_256_color(*color));
                    index += 2;
                }
            }
            38 if values.get(index + 1) == Some(&2) => {
                if let (Some(red), Some(green), Some(blue)) = (
                    values.get(index + 2),
                    values.get(index + 3),
                    values.get(index + 4),
                ) {
                    foreground = Some((*red as u32) << 16 | (*green as u32) << 8 | *blue as u32);
                    index += 4;
                }
            }
            _ => {}
        }
        index += 1;
    }

    (foreground, dim)
}

fn ansi_standard_color(value: u16) -> Option<u32> {
    const COLORS: [u32; 16] = [
        0x4e5a65, 0xe06c75, 0x98c379, 0xe5c07b, 0x61afef, 0xc678dd, 0x56b6c2, 0xd8dee9, 0x7f8c98,
        0xff7b86, 0xb7e19d, 0xffd683, 0x83c5ff, 0xe2a5f5, 0x7cdbeb, 0xffffff,
    ];
    let index = match value {
        30..=37 => value - 30,
        90..=97 => value - 90 + 8,
        _ => return None,
    };
    COLORS.get(index as usize).copied()
}

fn ansi_256_color(value: u16) -> u32 {
    if value < 16 {
        return ansi_standard_color(if value < 8 { value + 30 } else { value + 82 })
            .unwrap_or(0xc8d7e5);
    }
    if value >= 232 {
        let shade = 8 + (value as u32 - 232) * 10;
        return shade << 16 | shade << 8 | shade;
    }

    let value = value as u32 - 16;
    let component = |part| if part == 0 { 0 } else { 55 + part * 40 };
    let red = component(value / 36);
    let green = component((value / 6) % 6);
    let blue = component(value % 6);
    red << 16 | green << 8 | blue
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
                    profile_file: paths.profile_file(),
                },
                Err(error) => InitialState {
                    core_status: format!("저장 세션을 읽지 못했습니다: {error}"),
                    profiles: Vec::new(),
                    known_hosts_file: paths.known_hosts_file(),
                    profile_file: paths.profile_file(),
                },
            }
        }
        Err(error) => InitialState {
            core_status: format!("trace-deck-core 초기화 실패: {error}"),
            profiles: Vec::new(),
            known_hosts_file: std::path::PathBuf::new(),
            profile_file: std::path::PathBuf::new(),
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
                let session_form_focus = cx.focus_handle();
                let server_search_focus = cx.focus_handle();
                let file_action_focus = cx.focus_handle();
                let view = cx.new(|_| TraceDeckGpui {
                    core_status: initial_state.core_status.clone(),
                    profiles: initial_state.profiles.clone(),
                    selected_profile_id: None,
                    remote_path: "/".to_owned(),
                    remote_entries: Vec::new(),
                    known_hosts_file: initial_state.known_hosts_file.clone(),
                    profile_file: initial_state.profile_file.clone(),
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
                    sidebar_scroll: ScrollHandle::new(),
                    file_list_scroll: ScrollHandle::new(),
                    log_scroll: ScrollHandle::new(),
                    terminal_scroll: ScrollHandle::new(),
                    terminal_focus,
                    session_form_focus,
                    server_search_focus,
                    server_search: String::new(),
                    session_draft: SessionDraft::default(),
                    active_draft_field: DraftField::Name,
                    active_bottom_panel: BottomPanel::Log,
                    bottom_panel_height: 254.0,
                    show_new_session_dialog: false,
                    file_action: None,
                    file_action_name: String::new(),
                    file_action_focus,
                    context_menu: None,
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

#[cfg(test)]
mod tests {
    use super::{entry_metadata, normalize_newlines, parse_log_content};

    #[test]
    fn preserves_ansi_colors_and_normalizes_newlines() {
        let content = normalize_newlines(
            "\u{1b}[2m2026-11-05\u{1b}[0m\r\n\u{1b}[31merror\u{1b}[0m".to_owned(),
        );
        let display = parse_log_content(&content);

        assert_eq!(display.text, "2026-11-05\nerror");
        assert_eq!(display.highlights.len(), 2);
        assert_eq!(display.highlights[1].range, 11..16);
        assert_eq!(display.highlights[1].color, 0xe06c75);
    }

    #[test]
    fn labels_remote_entry_metadata_in_column_order() {
        assert_eq!(
            entry_metadata("-rw-r--r-- 1 opc staff 5150 Oct 22 23:41 app.log").values(),
            ["-rw-r--r--", "opc", "staff", "5150", "Oct 22 23:41"].map(str::to_owned)
        );
    }
}
