use crate::pages::ViewFormat;
use std::collections::HashSet;
use yew::{
    format::Json,
    services::{storage::Area as StorageArea, StorageService},
    worker::{Agent, AgentLink, Context, HandlerId},
};

const READER_PREFERENCE_KEY: &'static str = "llrs.reader.view";

pub(crate) struct UserAgent {
    storage: Option<StorageService>,
    link: AgentLink<Self>,
    subscribers: HashSet<HandlerId>,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Action {
    GetViewFormatPreference,
    SetViewFormatPreference(ViewFormat),
}

#[derive(Debug)]
pub(crate) enum Response {
    ViewFormatPreference(ViewFormat),
}

impl Agent for UserAgent {
    type Reach = Context<Self>;
    type Message = ();
    type Input = Action;
    type Output = Response;

    fn create(link: AgentLink<Self>) -> Self {
        // We're fine with not having a local storage of the preferences, use defaults
        let storage = StorageService::new(StorageArea::Local).ok();
        Self {
            storage,
            link,
            subscribers: HashSet::new(),
        }
    }

    fn update(&mut self, _: Self::Message) {}

    fn handle_input(&mut self, input: Self::Input, requester: HandlerId) {
        match input {
            Action::GetViewFormatPreference => self
                .link
                .respond(requester, self.get_view_format_response_or_default()),
            Action::SetViewFormatPreference(view_format) => {
                if let Some(storage) = &mut self.storage {
                    storage.store(READER_PREFERENCE_KEY, Json(&view_format));
                    for sub in &self.subscribers {
                        self.link
                            .respond(*sub, self.get_view_format_response_or_default())
                    }
                }
            }
        }
    }

    fn connected(&mut self, id: HandlerId) {
        self.subscribers.insert(id);
    }

    fn disconnected(&mut self, id: HandlerId) {
        self.subscribers.remove(&id);
    }
}

impl UserAgent {
    fn get_view_format_response_or_default(&self) -> Response {
        Response::ViewFormatPreference(self.storage.as_ref().map_or(
            ViewFormat::Single,
            |storage| {
                if let Json(Ok(view_format)) = storage.restore(READER_PREFERENCE_KEY) {
                    view_format
                } else {
                    ViewFormat::Single
                }
            },
        ))
    }
}
