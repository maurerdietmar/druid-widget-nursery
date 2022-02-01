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

use std::any::Any;

use druid::{Data, Env, Selector, SingleUse, UpdateCtx, Widget, WidgetId, Point};

use crate::StackChildPosition;

pub(crate) struct SwmSubWindowDesc {
    pub(crate) sub_window_root: Box<dyn Widget<()>>,
    pub(crate) position: StackChildPosition,
    pub(crate) modal: bool,
}

pub(crate) struct SwmSubWindowUpdate {
    pub(crate) data: Option<Box<dyn Any>>,
    pub(crate) env: Option<Env>,
}

pub(crate) const SWM_ADD_WINDOW: Selector<SingleUse<SwmSubWindowDesc>> =
    Selector::new("druid-widget-nursery.swm-add-window");

pub(crate) const SWM_DRAG_WINDOW: Selector<SingleUse<(WidgetId, Point)>> =
    Selector::new("druid-widget-nursery.swm-drag-window");

pub(crate) const SWM_CLOSE_WINDOW: Selector<SingleUse<Option<WidgetId>>> =
    Selector::new("druid-widget-nursery.swm-close-window");

pub(crate) const SWM_WINDOW_TO_TOP: Selector<WidgetId> =
    Selector::new("druid-widget-nursery.swm-window-to-top");

pub(crate) const SWM_HOST_TO_PROXY: Selector<Box<dyn Any>> =
    Selector::new("druid-widget-nursery.swm-host-to-proxy");

pub(crate) const SWM_PROXY_TO_HOST: Selector<SwmSubWindowUpdate> =
    Selector::new("druid-widget-nursery.swm-proxy-to-host");

pub(crate) const SWM_CONNECT_HOST: Selector<WidgetId> =
    Selector::new("druid-widget-nursery.swm-connect-host");

pub(crate) const SWM_DISCONNECT_HOST: Selector<WidgetId> =
    Selector::new("druid-widget-nursery.swm-disconnect-host");

// Send SwmSubWindowUpdate to the SubWindowHost
pub(crate) fn submit_host_update<T: Data>(
    ctx: &mut UpdateCtx,
    data: &T,
    data_changed: bool,
    env: &Env,
    host_id: WidgetId,
) {
    let update = SwmSubWindowUpdate {
        data: if data_changed {
            Some(Box::new((*data).clone()))
        } else {
            None
        },
        env: if ctx.env_changed() {
            Some(env.clone())
        } else {
            None
        },
    };
    let command = SWM_PROXY_TO_HOST.with(update).to(host_id);
    ctx.submit_command(command);
}
