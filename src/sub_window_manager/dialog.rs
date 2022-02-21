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
    BoxConstraints, Data, Event, EventCtx, ExtEventSink, Env, LayoutCtx, LifeCycle, LifeCycleCtx,
    UpdateCtx, Widget, PaintCtx, Size,
};
use druid::{WidgetId, SingleUse, Target};

use crate::StackChildPosition;

use super::manager::{add_window, SubWindowManagerId};
use super::window_config::SubWindowConfig;
use super::commands::*;

pub struct Dialog<T> {
    manager: SubWindowManagerId,
    id: WidgetId,
    sub_window_host: Option<WidgetId>,
    sink: Option<ExtEventSink>,
    builder: Box<dyn Fn() -> Box<dyn Widget<T>>>,
    window_config: SubWindowConfig,
}

impl <T: Data> Dialog<T> {

    pub fn new<W: Widget<T> + 'static>(
        manager: SubWindowManagerId,
        builder: impl Fn() -> W + 'static,
    ) -> Self {
        let proxy_id = WidgetId::next();
        Self {
            manager,
            id: proxy_id,
            sub_window_host: None,
            sink: None,
            builder: Box::new(move | | Box::new(builder())),
            window_config: SubWindowConfig::new(),
        }
    }

    pub fn position(mut self,  position: StackChildPosition) -> Self {
        self.window_config.set_position(position);
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.window_config.set_title(title);
        self
    }

    pub fn modal(mut self, modal: bool) -> Self {
        self.window_config.set_modal(modal);
        self
    }
}

impl <T> Drop for Dialog<T> {

    fn drop(&mut self) {
        if let Some(sink) = &self.sink {
            if let Some(host_id) = self.sub_window_host {
                sink.submit_command(
                    SWM_CLOSE_WINDOW,
                    SingleUse::new(Some(host_id)),
                    Target::Widget(self.manager.widget_id()),
                ).unwrap();
            }
        }
    }
}

impl <T: Data> Widget<T> for Dialog<T> {

    fn id(&self) -> Option<WidgetId> {
        Some(self.id)
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, _env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(SWM_DISCONNECT_HOST) => {
                let host_id = cmd.get_unchecked(SWM_DISCONNECT_HOST);
                if Some(*host_id) == self.sub_window_host {
                    self.sub_window_host = None;
                    ctx.set_handled();
                    return;
                }
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
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, _env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.sink = Some(ctx.get_external_handle());
            let widget = (self.builder)();
            self.sub_window_host = Some(add_window(
                ctx,
                self.manager,
                self.id,
                widget,
                data.clone(),
                self.window_config.clone(),
            ));
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if let Some(host_id)  = &self.sub_window_host {
            let data_changed = !old_data.same(data);
            if ctx.env_changed() || data_changed {
                submit_host_update(ctx, data, data_changed, env, *host_id);
            }
        }
     }

    fn layout(&mut self, _ctx: &mut LayoutCtx, _bc: &BoxConstraints, _data: &T, _env: &Env) -> Size {
        Size::ZERO
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _data: &T, _env: &Env) {}
}
