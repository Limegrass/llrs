use crate::pages::ViewFormat;
use yew::{
    format::Json,
    services::{storage::Area as StorageArea, StorageService},
    worker::{Agent, AgentLink, Context, HandlerId},
};

const READER_PREFERENCE_KEY: &'static str = "llrs.reader.view";

pub(crate) struct UserAgent {
    storage: Option<StorageService>,
    link: AgentLink<Self>,
}

fn get_view_format_response_or_default(agent: &UserAgent) -> Response {
    Response::ViewFormatPreference(
        agent
            .storage
            .as_ref()
            .map_or(ViewFormat::Single, |storage| {
                if let Json(Ok(view_format)) = storage.restore(READER_PREFERENCE_KEY) {
                    view_format
                } else {
                    ViewFormat::Single
                }
            }),
    )
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Action {
    GetViewFormatPreference,
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
        Self { storage, link }
    }

    fn update(&mut self, _: Self::Message) {}

    fn handle_input(&mut self, input: Self::Input, requester: HandlerId) {
        match input {
            Action::GetViewFormatPreference => self
                .link
                .respond(requester, get_view_format_response_or_default(self)),
        }
    }
}
