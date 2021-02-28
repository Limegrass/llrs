use llrs_model::Manga;
use log::*;
use std::{collections::HashSet, rc::Rc};
use yew::{
    format::{Json, Nothing},
    services::{
        fetch::{FetchTask, Request as FetchRequest, Response as FetchResponse},
        FetchService,
    },
    worker::*,
};

#[derive(Debug)]
pub(crate) enum Msg {
    FetchMangaComplete { mangas: Vec<Rc<Manga>> },
    Error,
}

#[derive(Debug)]
pub(crate) enum Action {
    GetMangaList,
    EmitListUpdate,
}

pub(crate) struct MangaAgent {
    link: AgentLink<MangaAgent>,
    fetch_task: Option<FetchTask>,
    mangas: Option<Rc<Vec<Rc<Manga>>>>,
    subscribers: HashSet<HandlerId>,
}

#[derive(Debug)]
pub(crate) enum Response {
    // TODO: Refactor to use a Map at some point
    MangaList { mangas: Rc<Vec<Rc<Manga>>> },
}

impl Agent for MangaAgent {
    type Reach = Context<Self>;
    type Message = Msg;
    type Input = Action;
    type Output = Response;

    fn create(link: AgentLink<Self>) -> Self {
        Self {
            link,
            fetch_task: None,
            mangas: None,
            subscribers: HashSet::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) {
        trace!("{:?}", msg);
        match msg {
            Msg::Error => {}
            Msg::FetchMangaComplete { mangas } => {
                self.mangas = Some(Rc::new(mangas));
                self.link.send_input(Action::EmitListUpdate);
            }
        }
    }

    fn handle_input(&mut self, action: Self::Input, requester: HandlerId) {
        match action {
            Action::GetMangaList => {
                if let Some(mangas) = &self.mangas {
                    let response = Response::MangaList {
                        mangas: Rc::clone(&mangas),
                    };
                    self.link.respond(requester, response);
                } else if self.fetch_task.is_none() {
                    self.begin_data_fetch()
                }
            }
            Action::EmitListUpdate => {
                if let Some(mangas) = &self.mangas {
                    for sub in self.subscribers.iter() {
                        let response = Response::MangaList {
                            mangas: Rc::clone(mangas),
                        };
                        self.link.respond(*sub, response);
                    }
                }
            }
        }
    }

    fn connected(&mut self, subscriber_id: HandlerId) {
        self.subscribers.insert(subscriber_id);
    }

    fn disconnected(&mut self, id: HandlerId) {
        self.subscribers.remove(&id);
    }
}

impl MangaAgent {
    fn begin_data_fetch(&mut self) {
        let task = self
            .get_request_manga_task(MangaAgent::parse_manga_fetch_to_msg)
            .expect("Failed to build request");
        self.fetch_task = Some(task);
    }

    fn parse_manga_fetch_to_msg(
        response: FetchResponse<Json<Result<Vec<Manga>, anyhow::Error>>>,
    ) -> Msg {
        let Json(data) = response.into_body();
        match data {
            Ok(mangas) => {
                let mangas = mangas.into_iter().map(|manga| Rc::new(manga)).collect();
                Msg::FetchMangaComplete { mangas }
            }
            Err(error) => {
                error!("{}", error);
                Msg::Error
            }
        }
    }

    fn get_request_manga_task(
        &self,
        result_handler: fn(FetchResponse<Json<Result<Vec<Manga>, anyhow::Error>>>) -> Msg,
    ) -> Result<FetchTask, anyhow::Error> {
        let request = FetchRequest::get("http://localhost:42069")
            .body(Nothing)
            .expect("Could not build request.");
        let callback = self.link.callback(result_handler);
        FetchService::fetch(request, callback)
    }
}
