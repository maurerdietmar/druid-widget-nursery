// Copyright 2022 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::atomic::{AtomicUsize, Ordering};

use druid::theme;
use druid::{AppLauncher, Color, Data, Lens, Widget, WidgetExt, WindowDesc, WindowSizePolicy};
use druid::widget::{Button, Flex, Label, TextBox};

use druid_widget_nursery::StackChildPosition;

use druid_widget_nursery::{SubWindowConfig, SubWindowLauncher, SubWindowManager, SubWindowManagerId, SubWindowProxy};

#[derive(Clone, Default, Data, Lens)]
struct AppState {
    text1: String,
    name: String,
    age: usize,
}

static WINDOW_COUNTER: AtomicUsize = AtomicUsize::new(1);


fn build_navigator_page(launcher: SubWindowLauncher<AppState>, page_level: usize) -> impl Widget<AppState> {
    let page = Flex::column()
        .with_child(
            Flex::row()
                .must_fill_main_axis(true)
                .main_axis_alignment(druid::widget::MainAxisAlignment::Start)
                .with_child(Button::new("Back").on_click({
                    let launcher = launcher.clone();
                    move |ctx, _, _| launcher.close_window(ctx)
                }))
                .with_spacer(10.0)
                .with_child(Label::new(format!("Simple Navigator Example, page level {}", page_level)))
                .with_flex_spacer(1.0)
                .padding(5.0)
                .background(theme::PLACEHOLDER_COLOR)
        )
        .with_flex_child(
            Flex::column()
                .with_child(Label::new(format!("Navigator page level {}", page_level)))
                .with_spacer(10.0)
                .with_child(Button::new("Push new page").on_click({
                    let launcher = launcher.clone();
                    move |ctx, data: &mut AppState, _| {
                        let widget = build_navigator_page(launcher.clone(), page_level + 1);
                        let config = SubWindowConfig::new().position(StackChildPosition::FIT);
                        launcher.add_window(ctx, widget, data, config);
                    }
                }))
                .with_spacer(10.0)
                .with_child(Button::new("Show Dialog").on_click({
                    let launcher = launcher.clone();
                    move |ctx, data: &mut AppState, _| {
                        let widget = Label::new("This is a stupid text without any information.")
                            .padding(10.);
                        let config = SubWindowConfig::new()
                            .modal(true)
                            .title(format!("Infor Dialog on navigator page level {}", page_level));
                        launcher.add_window(ctx, widget, data, config);
                    }
                }))
                .center(),
            1.0,
        );


    page.border(Color::WHITE, 1.0)
}

fn build_toolbar_ui(manager: SubWindowManagerId) -> impl Widget<AppState> {
    SubWindowProxy::new(manager, |launcher| {
        Flex::row()
            .with_child({
                let launcher = launcher.clone();
                Button::new("Add Window").on_click(move |ctx, data: &mut AppState, _| {
                    let widget = Flex::column()
                        .with_child(Label::new("TEST"))
                        .with_spacer(10.0)
                        .with_child(TextBox::new().lens(AppState::text1))
                        .with_spacer(10.0)
                         .with_child(Button::new("Close").on_click({
                            let launcher = launcher.clone();
                            move |ctx, _, _| launcher.close_window(ctx)
                        }))
                        .padding(10.);
                    let count = WINDOW_COUNTER.fetch_add(1, Ordering::SeqCst);
                    let config = SubWindowConfig::new()
                        .position(StackChildPosition::new().top(Some(100.)).left(Some(100.)))
                        .title(format!("Window {}", count));
                    launcher.add_window(ctx, widget, data, config);
                })
            })
            .with_spacer(5.0)
            .with_child({
                let launcher = launcher.clone();
                Button::new("Alert").on_click(move |ctx, data: &mut AppState, _| {
                    let widget = Flex::column()
                        .with_child(Label::new("This is an alert!"))
                        .with_spacer(10.0)
                        .with_child(Button::new("Close").on_click({
                            let launcher = launcher.clone();
                            move |ctx, _, _| launcher.close_window(ctx)
                        }))
                        .padding(10.);
                    let config = SubWindowConfig::new().title("Alert").modal(true);
                    launcher.add_window(ctx, widget, data, config);
                })
            })
            .with_spacer(5.0)
            .with_child({
                let launcher = launcher.clone();
                Button::new("Navigator Example").on_click(move |ctx, data: &mut AppState, _| {
                    let widget = build_navigator_page(launcher.clone(), 0);
                    let config = SubWindowConfig::new().position(StackChildPosition::FIT);
                    launcher.add_window(ctx, widget, data, config);
                })
            })
            .with_flex_spacer(1.0)
            .padding(5.0)
            .border(Color::WHITE, 1.0)
    })
}

fn build_manager_ui() -> impl Widget<AppState> {

    SubWindowManager::build_ui(|manager_id| {
        Flex::column()
            .with_child(build_toolbar_ui(manager_id))
            .with_flex_child(
                Flex::column()
                    .with_child(Label::new("SubWindow Manager root"))
                    .with_spacer(10.0)
                    .with_child(TextBox::new().lens(AppState::text1))
                    .center(),
                1.0,
            )
            .fix_height(500.)
            .fix_width(800.)
    })
}

fn build_ui() -> impl Widget<AppState> {

    Flex::column()
        .with_child(
            Flex::row()
                .with_child(Label::new("Text outside manager:"))
                .with_spacer(2.0)
                .with_child(TextBox::new().lens(AppState::text1))
                .padding(5.0)
        )
        .with_child(build_manager_ui())
 }

pub fn main() {
    let main_window = WindowDesc::new(build_ui().center())
        .window_size_policy(WindowSizePolicy::Content)
        .title("Sub-window manager test");

    let state = AppState::default();

    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(state)
        .expect("launch failed");
}
