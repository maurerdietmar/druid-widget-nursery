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

use druid::theme;
use druid::{
    BoxConstraints, Data, Event, EventCtx, Env, LayoutCtx, LifeCycle, LifeCycleCtx,
    Widget, WidgetExt, WidgetPod, PaintCtx, Point, RenderContext, Size, UpdateCtx,
};
use druid::{Command, WidgetId, SingleUse, Target};
use druid::widget::{BackgroundBrush, Button, Controller, Flex, Label, Padding};

use super::manager::SubWindowManagerId;
use super::commands::{SWM_CLOSE_WINDOW, SWM_WINDOW_TO_TOP, SWM_DRAG_WINDOW};
use super::window_config::SubWindowConfig;

struct SubWindowTitlebar {
    manager: SubWindowManagerId,
    host_id: WidgetId,
    drag: bool,
    drag_pos: Point,
}

impl SubWindowTitlebar {
    fn new(manager: SubWindowManagerId, host_id: WidgetId) -> Self {
        Self { manager, host_id, drag: false, drag_pos: Point::ZERO }
    }
}

impl <W: Widget<U> + 'static, U: Data> Controller<U, W> for SubWindowTitlebar {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut U,
        env: &Env,
    ) {
        match event {
            Event::MouseDown(ev) => {
                //println!("START DRAG {:?}", ev);
                ctx.set_active(true);
                self.drag = true;
                self.drag_pos = ev.pos;
            }
            Event::MouseUp(_ev) => {
                //println!("END DRAG {:?}", ev);
                ctx.set_active(false);
                self.drag = false;
            }
            Event::MouseMove(ev) => {
                if self.drag {
                    let move_to = ev.window_pos - self.drag_pos;
                    //println!("DRAG TO {:?}", move_to);
                    let command = Command::new(
                        SWM_DRAG_WINDOW,
                        SingleUse::new((self.host_id, Point::new(move_to.x, move_to.y))),
                        Target::Widget(self.manager.widget_id()),
                    );
                    ctx.submit_command(command);
                }
            }
            _ => {}
        }
        child.event(ctx, event, data, env);
    }
}

pub struct SubWindow<U> {
    titlebar: Option<WidgetPod<U, Box<dyn Widget<U>>>>,
    body: WidgetPod<U, Box<dyn Widget<U>>>,
    manager: SubWindowManagerId,
    host_id: WidgetId,
    border_width: f64,

    // Note: Druid has now way to return the minimum layout size, so we use a prototype
    // and layout it with BoxConstraints::UNBOUNDED
    titlebar_prototype: Padding<U, Flex<U>>,
    titlebar_prototype_size: Option<Size>, // min titlebar size

}

impl <U: Data> SubWindow<U> {

    pub fn new(
        body: impl Widget<U> + 'static,
        config: &SubWindowConfig,
        manager: SubWindowManagerId,
        host_id: WidgetId,
    ) -> Self {
        let mut border_width = 0.0;
        let mut titlebar = None;
        let mut titlebar_prototype = Padding::new(0.0, Flex::row());

        if let Some(ref title) = config.title {
            let title_bar = Flex::row()
                .main_axis_alignment(druid::widget::MainAxisAlignment::Start)
                .with_child(Label::new(title.clone()))
                .with_flex_spacer(1.0)
                .with_child(
                    Button::new("x").on_click(|ctx, _, _| {
                        let command = SWM_CLOSE_WINDOW
                            .with(SingleUse::new(None))
                            .to(Target::Auto);
                        ctx.submit_notification(command);
                    })
                )
                .padding(5.0)
                .background(theme::PLACEHOLDER_COLOR).controller(SubWindowTitlebar::new(manager, host_id));

            titlebar = Some(WidgetPod::new(title_bar).boxed());

            titlebar_prototype = Flex::row()
                .main_axis_alignment(druid::widget::MainAxisAlignment::Start)
                .with_child(Label::new(title.clone()))
                .with_child(Button::new("x"))
                .padding(5.0);

            border_width = 1.0;
        }
                                               ;
        Self {
            manager,
            host_id,
            titlebar,
            titlebar_prototype,
            titlebar_prototype_size: None,
            border_width,
            body: WidgetPod::new(body).boxed(),
        }
    }
}

impl <U: Data> Widget<U> for SubWindow<U> {

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut U, env: &Env) {
        if let Some(ref mut titlebar) = self.titlebar {

            if matches!(event, Event::MouseDown(_)) {
                let command = SWM_WINDOW_TO_TOP.with(self.host_id).to(self.manager.widget_id());
                ctx.submit_command(command);
            }

            titlebar.event(ctx, event, data, env);
        }
        self.body.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &U, env: &Env) {
        if let Some(ref mut titlebar) = self.titlebar {
            titlebar.lifecycle(ctx, event, data, env);
            self.titlebar_prototype.lifecycle(ctx, event, data, env);
        }
        self.body.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &U, data: &U, env: &Env) {
        if let Some(ref mut titlebar) = self.titlebar {
            titlebar.update(ctx, data, env);
        }
        self.body.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &U, env: &Env) -> Size {

        let titlebar_prototype_size = match self.titlebar {
            Some(ref mut _titlebar) => {
                match self.titlebar_prototype_size {
                    None => {
                        let titlebar_bc =  BoxConstraints::UNBOUNDED;
                        let size = self.titlebar_prototype.layout(ctx, &titlebar_bc, data, env);
                        //println!("PROTOTYPE SIZE {:?}", size);
                        self.titlebar_prototype_size = Some(size);
                        size
                    }
                    Some(size) => size,
                }
            }
            None => Size::ZERO,
        };

        let body_bc = bc.shrink((
            2.0 * self.border_width,
            2.0 * self.border_width - titlebar_prototype_size.height,
        ));

        let body_size = self.body.layout(ctx, &body_bc, data, env);
        //println!("BODY {:?} {:?}",  body_bc , body_size);

        let content_width = body_size.width.max(titlebar_prototype_size.width);

        let titlebar_size = match self.titlebar {
            Some(ref mut titlebar) => {
                let titlebar_bc = BoxConstraints::tight(Size::new(content_width, titlebar_prototype_size.height));
                let size = titlebar.layout(ctx, &titlebar_bc, data, env);
                //println!("TS {:?} {:?}",  titlebar_bc , size);
                titlebar.set_origin(ctx, data, env, Point::new(self.border_width, self.border_width));
                size
            }
            None => Size::ZERO,
        };

        self.body.set_origin(ctx, data, env, Point::new(self.border_width, self.border_width + titlebar_size.height));

        Size::new(
            2.0 * self.border_width + body_size.width.max(titlebar_prototype_size.width),
            2.0 * self.border_width + body_size.height + titlebar_size.height,
        )
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &U, env: &Env) {

        let bg_color = env.get(theme::WINDOW_BACKGROUND_COLOR);
        let mut brush = BackgroundBrush::Color(bg_color);
        brush.paint(ctx, data, env);

        if self.border_width > 0.0 {
          let border_rect = ctx
                .size()
                .to_rect()
                .inset(self.border_width / -2.0);

            let border_color = env.get(theme::BORDER_LIGHT);
            ctx.stroke(border_rect, &border_color, self.border_width);
        }

        if let Some(ref mut titlebar) = self.titlebar {
            titlebar.paint(ctx, data, env);
        }
        self.body.paint(ctx, data, env);
    }
}
