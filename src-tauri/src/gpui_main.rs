use gpui::{
    App, Application, Bounds, Context, Div, IntoElement, Render, Window, WindowBounds,
    WindowOptions, div, prelude::*, px, rgb, size,
};
use trace_deck_core::AppPaths;

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 800.0;
const APP_BACKGROUND: u32 = 0x111820;
const PANEL_BACKGROUND: u32 = 0x151e28;
const BORDER: u32 = 0x293846;
const MUTED_TEXT: u32 = 0x8ba1b5;
const ACCENT: u32 = 0x36c79a;

struct TraceDeckGpui {
    core_status: String,
    show_new_session_dialog: bool,
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
            .child(
                div()
                    .mx(px(10.0))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .p(px(14.0))
                    .text_sm()
                    .text_color(rgb(MUTED_TEXT))
                    .child("저장된 세션이 없습니다.\n새 세션을 추가해 연결 정보를 저장하세요."),
            )
            .child(
                div()
                    .mt(px(18.0))
                    .px(px(16.0))
                    .py(px(10.0))
                    .text_xs()
                    .text_color(rgb(0x7fa4c3))
                    .child("OPEN SESSIONS"),
            )
            .child(
                div()
                    .mx(px(10.0))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .p(px(14.0))
                    .text_sm()
                    .text_color(rgb(MUTED_TEXT))
                    .child("열린 세션이 없습니다."),
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

    fn workspace(&self) -> Div {
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
            .child(
                div()
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
            )
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
                    .child("CPU  —")
                    .child("DISK  —"),
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
                    .child(self.workspace()),
            );

        if self.show_new_session_dialog {
            root.child(self.new_session_dialog(cx))
        } else {
            root
        }
    }
}

fn initialize_core() -> String {
    match AppPaths::for_current_user().and_then(|paths| {
        paths.ensure_exists()?;
        Ok(paths)
    }) {
        Ok(_) => "trace-deck-core 데이터 경로를 준비했습니다.".to_owned(),
        Err(error) => format!("trace-deck-core 초기화 실패: {error}"),
    }
}

fn main() {
    let core_status = initialize_core();
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| TraceDeckGpui {
                    core_status: core_status.clone(),
                    show_new_session_dialog: false,
                })
            },
        )
        .expect("TraceDeck GPUI 창을 열 수 없습니다.");
        cx.activate(true);
    });
}
