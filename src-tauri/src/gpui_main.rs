use gpui::{
    App, Application, Bounds, Context, IntoElement, Render, Window, WindowBounds, WindowOptions,
    div, prelude::*, px, rgb, size,
};
use trace_deck_core::AppPaths;

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 800.0;

struct TraceDeckGpui {
    core_status: String,
}

impl Render for TraceDeckGpui {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x101923))
            .text_color(rgb(0xe5edf5))
            .child(
                div()
                    .h(px(52.0))
                    .flex()
                    .items_center()
                    .px(px(20.0))
                    .border_b_1()
                    .border_color(rgb(0x263849))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("TraceDeck"),
                    )
                    .child(
                        div()
                            .ml(px(12.0))
                            .text_xs()
                            .text_color(rgb(0x7fa4c3))
                            .child("GPUI 실험 버전"),
                    ),
            )
            .child(
                div().flex_1().flex().items_center().justify_center().child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .items_center()
                        .child(div().text_xl().child("GPUI 초기화 완료"))
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x8ba1b5))
                                .child(self.core_status.clone()),
                        )
                        .child(
                            div()
                                .mt(px(12.0))
                                .text_sm()
                                .text_color(rgb(0x8ba1b5))
                                .child(
                                    "다음 단계에서 기존 TraceDeck 화면과 세션 기능을 이식합니다.",
                                ),
                        ),
                ),
            )
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
                })
            },
        )
        .expect("TraceDeck GPUI 창을 열 수 없습니다.");
        cx.activate(true);
    });
}
