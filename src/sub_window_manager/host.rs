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

use std::ops::Deref;

use tracing::warn;

use druid::{
    BoxConstraints, Data, Event, EventCtx, Env,
    LayoutCtx, LifeCycle, LifeCycleCtx, UpdateCtx, Widget, WidgetId, WidgetPod,
    PaintCtx, Point, Size, SingleUse,
};

use super::manager::SubWindowManagerId;
use super::commands::*;

pub(crate) struct SubWindowHost<U, W: Widget<U>> {
    manager: SubWindowManagerId,
    id: WidgetId,
    proxy_id: WidgetId,
    child: WidgetPod<U, W>,
    data: U,
    // fixme: env: Env,
}

impl<U, W: Widget<U>> SubWindowHost<U, W> {
    pub(crate) fn new(manager: SubWindowManagerId, id: WidgetId, proxy_id: WidgetId, widget: W, data: U) -> Self {
        SubWindowHost {
            manager,
            id,
            proxy_id,
            data,
            child: WidgetPod::new(widget),
        }
    }
}

impl<U: Data, W: Widget<U>> Widget<()> for SubWindowHost<U, W> {

    fn id(&self) -> Option<WidgetId> {
        Some(self.id)
    }

    fn event(&mut self, ctx: &mut EventCtx<'_, '_>, event: &Event, _data: &mut (), env: &Env) {
        match event {
            Event::Notification(cmd) if cmd.is(SWM_CLOSE_WINDOW) => {
                let mut payload = cmd.get(SWM_CLOSE_WINDOW);
                if let Some(_) = payload.take() {
                    // generate a new command which includes the window ID
                    let command = SWM_CLOSE_WINDOW
                        .with(SingleUse::new(Some(self.id)))
                        .to(self.manager.widget_id());
                    ctx.submit_command(command);

                    let command = SWM_DISCONNECT_HOST
                        .with(self.id)
                        .to(self.proxy_id);
                    ctx.submit_command(command);

                    ctx.set_handled();
                    return;
                }
            }
            Event::Command(cmd) if cmd.is(SWM_PROXY_TO_HOST) => {
                let update = cmd.get_unchecked(SWM_PROXY_TO_HOST);
                 if let Some(data_update) = &update.data {
                    if let Some(dc) = data_update.downcast_ref::<U>() {
                        self.data = dc.deref().clone();
                        ctx.request_update();
                    } else {
                         warn!("Received a SWM_PROXY_TO_HOST command that could not be unwrapped.");
                     }

                }
                if let Some(_env_update) = &update.env {
                    // fixme: self.env = env_update.clone()
                }
                ctx.set_handled();
                return;
           }
            _ => {}
        }

        let old = self.data.clone();
        self.child.event(ctx, event, &mut self.data, env);

        if !old.same(&self.data) {
            ctx.submit_command(
                SWM_HOST_TO_PROXY
                    .with(Box::new(self.data.clone()))
                    .to(self.proxy_id),
            )
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx<'_, '_>, event: &LifeCycle, _data: &(), env: &Env) {
        self.child.lifecycle(ctx, event, &self.data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_, '_>, _old_data: &(), _data: &(), env: &Env) {
        self.child.update(ctx, &self.data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_, '_>, bc: &BoxConstraints, _data: &(), env: &Env) -> Size {
        let size = self.child.layout(ctx, bc, &self.data, env);
        self.child.set_origin(ctx, &self.data, env, Point::ORIGIN);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_, '_, '_>, _data: &(), env: &Env) {
        self.child.paint(ctx, &self.data, env);
    }
}
