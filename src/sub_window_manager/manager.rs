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

use druid::{
    BoxConstraints, Data, Event, EventCtx, Env, LayoutCtx, LifeCycle, LifeCycleCtx,
    UpdateCtx, Widget, WidgetId, WidgetExt, PaintCtx, Point, SingleUse, Size, UnitPoint,
};
use druid::widget::Label;

use crate::{CommandCtx, Stack, StackChildParams, StackChildPosition};

use super::commands::*;
use super::host::SubWindowHost;
use super::window_config::SubWindowConfig;
use super::window_decoration::SubWindow;

#[derive(Copy, Clone, Debug)]
pub struct SubWindowManagerId(WidgetId);

impl SubWindowManagerId {
    pub(crate) fn widget_id(&self) -> WidgetId {
        self.0
    }
}

pub struct SubWindowManager<T> {
    stack: Stack<()>,
    id: WidgetId,
    root_child: Option<Box<dyn Widget<T>>>,
    root_host_id: WidgetId,
}

pub(crate) fn add_window<W: Widget<U> + 'static, U: Data>(
    ctx: &mut impl CommandCtx,
    manager: SubWindowManagerId,
    proxy_id: WidgetId,
    widget: W,
    data: U,
    config: SubWindowConfig,
) {
    let host_id = WidgetId::next();

    let window = SubWindow::new(widget, &config, manager, host_id);

    let sub_window_root = SubWindowHost::new(manager, host_id, proxy_id, window, data).boxed();

    let command = SWM_ADD_WINDOW
        .with(
            SingleUse::new(
                SwmSubWindowDesc {
                    sub_window_root,
                    position: config.position,
                    modal: config.modal,
                }
            )
        )
        .to(manager.0);

    ctx.submit_command(command);

    let command = SWM_CONNECT_HOST.with(host_id).to(proxy_id);
    ctx.submit_command(command);
}

impl <T: Data> SubWindowManager<T> {
    fn new() -> Self {
        Self {
            stack: Stack::new().align(UnitPoint::CENTER),
            id: WidgetId::next(),
            root_child: Some(Label::new("Sub Window Manager").center().boxed()),
            root_host_id: WidgetId::next(),
        }
    }

    /// Create a new instzance
    pub fn build_ui<W: Widget<T> + 'static>(
        build_ui: impl Fn(SubWindowManagerId) -> W,
    ) -> Self {
        let mut manager = Self::new();
        let child = build_ui(manager.manager_id());
        manager.root_child = Some(Box::new(child));
        manager
    }

    fn manager_id(&self) -> SubWindowManagerId {
        SubWindowManagerId(self.id)
    }
}

impl <T: Data> Widget<T> for SubWindowManager<T> {

    fn id(&self) -> Option<WidgetId> {
        Some(self.id)
    }

    fn event(&mut self, ctx: &mut EventCtx<'_, '_>, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(SWM_ADD_WINDOW) => {
                let payload = cmd.get_unchecked(SWM_ADD_WINDOW);
                if let Some(sub_window_desc) = payload.take() {
                    let params = StackChildParams::from(sub_window_desc.position)
                        .modal( sub_window_desc.modal);
                    self.stack.add_positioned_child(sub_window_desc.sub_window_root, params);
                    ctx.children_changed();
                    ctx.set_handled();
                }
                return;
            }
            Event::Command(cmd) if cmd.is(SWM_DRAG_WINDOW) => {
                let payload = cmd.get_unchecked(SWM_DRAG_WINDOW);
                if let Some((host_id, move_to)) = payload.take() {
                    let origin = ctx.to_window(Point::new(0., 0.));
                    let position = StackChildPosition::new()
                        .left(Some(move_to.x - origin.x))
                        .top(Some(move_to.y - origin.y));
                     self.stack.move_child(ctx, host_id, position);
                    ctx.set_handled();
                }
                return;
            }
            Event::Command(cmd) if cmd.is(SWM_CLOSE_WINDOW) => {
                let payload = cmd.get_unchecked(SWM_CLOSE_WINDOW);
                if let Some(Some(host_id)) = payload.take() {
                    self.stack.remove_child(ctx, host_id);
                    ctx.set_handled();
                }
                return;
            }
            Event::Command(cmd) if cmd.is(SWM_WINDOW_TO_TOP) => {
                let host_id = cmd.get_unchecked(SWM_WINDOW_TO_TOP);
                self.stack.child_to_front(ctx, *host_id);
                ctx.set_handled();
                return;
            }
            Event::Command(cmd) if cmd.is(SWM_HOST_TO_PROXY) => {
                if let Some(update) = cmd
                    .get_unchecked(SWM_HOST_TO_PROXY)
                    .downcast_ref::<T>()
                {
                    *data = (*update).clone();
                }
                ctx.set_handled();
                return;
            }
            _ => {}
        }

        self.stack.event(ctx, event, &mut (), env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx<'_, '_>, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            if let Some(root_child) = self.root_child.take() {
                let sub_window_root = SubWindowHost::new(
                    self.manager_id(),
                    self.root_host_id,
                    self.id, // proxy to ourself
                    root_child,
                    data.clone(),
                );

                self.stack.add_child(sub_window_root);
                ctx.children_changed();
            }
        }
        self.stack.lifecycle(ctx, event, &(), env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_, '_>, old_data: &T, data: &T, env: &Env) {
        // Note: Update with old/new the same! Still required to maintain state.
        self.stack.update(ctx, &(), &(), env);

        // send updates to the root SubWindowHost
        let data_changed = !old_data.same(data);
        if ctx.env_changed() || data_changed {
            submit_host_update(ctx, data, data_changed, env, self.root_host_id);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_, '_>, bc: &BoxConstraints, _data: &T, env: &Env) -> Size {
        self.stack.layout(ctx, bc, &(), env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_, '_, '_>, _data: &T, env: &Env) {
        self.stack.paint(ctx, &(), env);
    }
}
