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

use std::marker::PhantomData;

use druid::{
    BoxConstraints, Data, Event, EventCtx, Env, LayoutCtx, LifeCycle, LifeCycleCtx,
    UpdateCtx, Widget, WidgetPod, PaintCtx, Point, Size,
};
use druid::{WidgetId, SingleUse, Target};

use crate::CommandCtx;

use super::commands::*;
use super::manager::{add_window, SubWindowManagerId};
use super::window_config::SubWindowConfig;

#[derive(Copy, Clone)]
pub struct SubWindowLauncher<T> {
    manager: SubWindowManagerId,
    proxy_id: WidgetId,
    phantom: PhantomData<T>,
}

impl <T: Data> SubWindowLauncher<T> {

    fn new(manager: SubWindowManagerId, proxy_id: WidgetId) -> Self {
        Self {  manager, proxy_id, phantom: PhantomData }
    }

    pub fn add_window(
        &self,
        ctx: &mut impl CommandCtx,
        widget: impl Widget<T> + 'static,
        data: &T,
        config: SubWindowConfig,
    ) {
        add_window(
            ctx,
            self.manager,
            self.proxy_id,
            widget,
            data.clone(),
            config,
        );
    }

    pub fn close_window(&self, ctx: &mut EventCtx) {
        // We do not know the host_id here, so we send a notification up
        // to the SubWindowHost (who fills in the rest)
        let command = SWM_CLOSE_WINDOW
            .with(SingleUse::new(None))
            .to(Target::Auto);
        ctx.submit_notification(command);
    }
}

pub struct SubWindowProxy<T> {
    child: WidgetPod<T, Box<dyn Widget<T>>>,
    manager: SubWindowManagerId,
    id: WidgetId,
    sub_window_hosts: Vec<WidgetId>,
}

impl <T: Data> SubWindowProxy<T> {

    pub fn new<W: Widget<T> + 'static>(
        manager: SubWindowManagerId,
        build_ui: impl Fn(SubWindowLauncher<T>) -> W,
    ) -> Self {
        let proxy_id = WidgetId::next();
        let launcher = SubWindowLauncher::new(manager, proxy_id);
        let child = build_ui(launcher);
        Self {
            manager,
            child: WidgetPod::new(child).boxed(),
            id: proxy_id,
            sub_window_hosts: Vec::new(),
        }
    }

    pub fn launcher(&self) -> SubWindowLauncher<T> {
        SubWindowLauncher::new(self.manager, self.id)
    }
}

impl <T: Data> Widget<T> for SubWindowProxy<T> {
    fn id(&self) -> Option<WidgetId> {
        Some(self.id)
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(SWM_CONNECT_HOST) => {
                let host_id = cmd.get_unchecked(SWM_CONNECT_HOST);
                self.sub_window_hosts.push(*host_id);
                ctx.set_handled();
                return;
            }
            Event::Command(cmd) if cmd.is(SWM_DISCONNECT_HOST) => {
                let host_id = cmd.get_unchecked(SWM_DISCONNECT_HOST);
                self.sub_window_hosts.retain(|id| id != host_id);
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
        self.child.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.child.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        let data_changed = !old_data.same(data);
        if ctx.env_changed() || data_changed {
            for host_id in &self.sub_window_hosts {
                submit_host_update(ctx, data, data_changed, env, *host_id);
            }
        }
        self.child.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.child.layout(ctx, bc, data, env);
        self.child.set_origin(ctx, data, env, Point::ORIGIN);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint(ctx, data, env);
    }
}
