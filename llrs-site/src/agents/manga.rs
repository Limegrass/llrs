use llrs_model::Manga;
use log::*;
use std::{collections::HashSet, rc::Rc};
use yew::{
    format::{Json, Nothing},
    services::{
        fetch::{FetchTask, Request, Response},
        FetchService,
    },
    worker::*,
};

#[derive(Debug)]
pub enum Msg {
    FetchMangaComplete { mangas: Vec<Manga> },
    Error,
}

#[derive(Debug)]
pub enum Action {
    GetMangaList,
    EmitListUpdate,
}

pub struct MangaAgent {
    link: AgentLink<MangaAgent>,
    fetch_task: Option<FetchTask>,
    mangas: Option<Rc<Vec<Manga>>>,
    subscribers: HashSet<HandlerId>,
}

impl Agent for MangaAgent {
    type Reach = Context<Self>;
    type Message = Msg;
    type Input = Action;
    type Output = Rc<Vec<Manga>>;

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
                    self.link.respond(requester, Rc::clone(&mangas));
                } else if self.fetch_task.is_none() {
                    self.begin_data_fetch()
                }
            }
            Action::EmitListUpdate => {
                if let Some(mangas) = &self.mangas {
                    for sub in self.subscribers.iter() {
                        self.link.respond(*sub, Rc::clone(&mangas));
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
        response: Response<Json<Result<Vec<Manga>, anyhow::Error>>>,
    ) -> Msg {
        let Json(data) = response.into_body();
        match data {
            Ok(mangas) => Msg::FetchMangaComplete { mangas },
            Err(error) => {
                error!("{}", error);
                Msg::Error
            }
        }
    }

    fn get_request_manga_task(
        &self,
        result_handler: fn(Response<Json<Result<Vec<Manga>, anyhow::Error>>>) -> Msg,
    ) -> Result<FetchTask, anyhow::Error> {
        let request = Request::get("http://localhost:42069")
            .body(Nothing)
            .expect("Could not build request.");
        let callback = self.link.callback(result_handler);
        FetchService::fetch(request, callback)
    }
}
