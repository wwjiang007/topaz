// Copyright 2018 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#[macro_use]
extern crate failure;
extern crate fidl;
extern crate fidl_fuchsia_ui_gfx as gfx;
extern crate fidl_fuchsia_ui_scenic;
extern crate fidl_fuchsia_ui_viewsv1;
extern crate fidl_fuchsia_ui_policy;
extern crate fidl_fuchsia_ui_viewsv1token;
extern crate fuchsia_app as component;
extern crate fuchsia_async as async;
extern crate fuchsia_scenic as scenic;
extern crate fuchsia_zircon as zx;
extern crate futures;

use component::client::connect_to_service;
use component::server::ServiceFactory;
use failure::{Error, ResultExt};
use fidl::endpoints2::{create_endpoints, ClientEnd, RequestStream, ServerEnd, ServiceMarker};
use fidl_fuchsia_ui_scenic::{ScenicMarker, SessionListenerMarker, SessionListenerRequest,
                             SessionMarker};
use fidl_fuchsia_ui_viewsv1::ViewProviderRequest::CreateView;
use fidl_fuchsia_ui_viewsv1::{ViewListenerMarker, ViewListenerRequest, ViewManagerMarker,
                              ViewManagerProxy, ViewMarker, ViewProviderMarker,
                              ViewProviderRequestStream};
use fidl_fuchsia_ui_viewsv1token::ViewOwnerMarker;
use futures::future::ok as fok;
use futures::{FutureExt, StreamExt,future::empty};
use gfx::ColorRgba;
use scenic::{ImportNode, Material, Rectangle, Session, SessionPtr, ShapeNode};
use std::sync::{Arc, Mutex};

struct ErmineView {
    _view: fidl_fuchsia_ui_viewsv1::ViewProxy,
    session: SessionPtr,
    import_node: ImportNode,
    background_node: ShapeNode,
    width: f32,
    height: f32,
}

type ErmineViewPtr = Arc<Mutex<ErmineView>>;

impl ErmineView {
    pub fn new(
        view_listener_request: ServerEnd<ViewListenerMarker>,
        view: fidl_fuchsia_ui_viewsv1::ViewProxy, mine: zx::EventPair,
        scenic: fidl_fuchsia_ui_scenic::ScenicProxy,
    ) -> Result<ErmineViewPtr, Error> {
        let (session_listener_client, session_listener_server) = zx::Channel::create()?;
        let session_listener = ClientEnd::new(session_listener_client);

        let (session_proxy, session_request) = create_endpoints::<SessionMarker>()?;
        scenic.create_session(session_request, Some(session_listener))?;
        let session = Session::new(session_proxy);

        let view_controller = ErmineView {
            _view: view,
            session: session.clone(),
            import_node: ImportNode::new(session.clone(), mine),
            background_node: ShapeNode::new(session.clone()),
            width: 0.0,
            height: 0.0,
        };

        let view_controller = Arc::new(Mutex::new(view_controller));

        Self::setup_session_listener(&view_controller, session_listener_server);
        Self::setup_view_listener(&view_controller, view_listener_request);

        if let Ok(vc) = view_controller.lock() {
            vc.setup_scene();
            vc.present();
        } else {
            bail!("Inexplicably couldn't lock mutex for view controller.");
        }

        Ok(view_controller)
    }

    fn setup_session_listener(
        view_controller: &ErmineViewPtr, session_listener_server: zx::Channel,
    ) {
        let session_listener_request =
            ServerEnd::<SessionListenerMarker>::new(session_listener_server);
        let view_controller = view_controller.clone();
        async::spawn(
            session_listener_request
                .into_stream()
                .unwrap()
                .for_each(move |request| {
                    match request {
                        SessionListenerRequest::OnEvent { events, .. } => view_controller
                            .lock()
                            .unwrap()
                            .handle_session_events(events),
                        _ => (),
                    }
                    fok(())
                }).map(|_| ())
                .recover(|e| eprintln!("view listener error: {:?}", e)),
        );
    }

    fn setup_view_listener(
        view_controller: &ErmineViewPtr, view_listener_request: ServerEnd<ViewListenerMarker>,
    ) {
        let view_controller = view_controller.clone();
        async::spawn(
            view_listener_request
                .into_stream()
                .unwrap()
                .for_each(
                    move |ViewListenerRequest::OnPropertiesChanged {
                              properties,
                              responder,
                          }| {
                        view_controller
                            .lock()
                            .unwrap()
                            .handle_properies_changed(&properties);
                        responder.send()
                    },
                ).map(|_| ())
                .recover(|e| eprintln!("view listener error: {:?}", e)),
        );
    }

    fn setup_scene(&self) {
        self.import_node
            .resource()
            .set_event_mask(gfx::METRICS_EVENT_MASK);
        self.import_node.add_child(&self.background_node);
        let material = Material::new(self.session.clone());
        material.set_color(ColorRgba {
            red: 0xb7,
            green: 0x41,
            blue: 0x0e,
            alpha: 0xff,
        });
        self.background_node.set_material(&material);
    }

    fn update(&mut self) {
        let center_x = self.width * 0.5;
        let center_y = self.height * 0.5;
        self.background_node.set_shape(&Rectangle::new(
            self.session.clone(),
            self.width,
            self.height,
        ));
        self.background_node
            .set_translation(center_x, center_y, 0.0);
        self.present();
    }

    fn present(&self) {
        async::spawn(
            self.session
                .lock()
                .present(0)
                .map(|_| ())
                .recover(|e| eprintln!("present error: {:?}", e)),
        );
    }

    fn handle_session_events(&mut self, events: Vec<fidl_fuchsia_ui_scenic::Event>) {
        events.iter().for_each(|event| match event {
            fidl_fuchsia_ui_scenic::Event::Gfx(gfx::Event::Metrics(_event)) => {
                self.update();
            }
            _ => (),
        });
    }

    fn handle_properies_changed(&mut self, properties: &fidl_fuchsia_ui_viewsv1::ViewProperties) {
        if let Some(ref view_properties) = properties.view_layout {
            self.width = view_properties.size.width;
            self.height = view_properties.size.height;
            self.update();
        }
    }
}

struct App {
    view_manager: ViewManagerProxy,
    views: Vec<ErmineViewPtr>,
}

type AppPtr = Arc<Mutex<App>>;

impl App {
    pub fn new() -> AppPtr {
        let view_manager = connect_to_service::<ViewManagerMarker>().unwrap();
        Arc::new(Mutex::new(App {
            view_manager,
            views: vec![],
        }))
    }

    pub fn spawn_view_provider_server(app: &AppPtr, chan: async::Channel) {
        let app = app.clone();
        async::spawn(
            ViewProviderRequestStream::from_channel(chan)
                .for_each(move |req| {
                    let CreateView { view_owner, .. } = req;
                    App::app_create_view(app.clone(), view_owner).unwrap();
                    futures::future::ok(())
                }).map(|_| ())
                .recover(|e| eprintln!("error running view_provider server: {:?}", e)),
        )
    }

    pub fn app_create_view(app: AppPtr, req: ServerEnd<ViewOwnerMarker>) -> Result<(), Error> {
        app.lock().unwrap().create_view(req)
    }

    pub fn create_view(&mut self, req: ServerEnd<ViewOwnerMarker>) -> Result<(), Error> {
        let (view, view_server_end) = create_endpoints::<ViewMarker>()?;
        let (view_listener, view_listener_server) = zx::Channel::create()?;
        let view_listener_request = ServerEnd::new(view_listener_server);
        let (mine, theirs) = zx::EventPair::create().unwrap();
        self.view_manager.create_view(
            view_server_end,
            req,
            ClientEnd::new(view_listener),
            theirs,
            None,
        )?;
        let (scenic, scenic_request) = create_endpoints::<ScenicMarker>()?;
        self.view_manager.get_scenic(scenic_request).unwrap();
        let view_ptr = ErmineView::new(view_listener_request, view, mine, scenic).unwrap();
        self.views.push(view_ptr);
        Ok(())
    }
}

struct ViewProvider {
    app: AppPtr,
}

impl ServiceFactory for ViewProvider {
    fn service_name(&self) -> &str {
        ViewProviderMarker::NAME
    }

    fn spawn_service(&mut self, channel: async::Channel) {
        App::spawn_view_provider_server(&self.app, channel);
    }
}

fn main() -> Result<(), Error> {
    let mut executor = async::Executor::new().context("Error creating executor")?;

    let app = App::new();

    let view_provider = ViewProvider { app: app.clone() };

    executor.run_singlethreaded(empty::<(), Error>())?;

    Ok(())
}