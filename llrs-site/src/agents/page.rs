use llrs_model::Page;
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
    FetchPageComplete { pages: Vec<Page> },
    Error,
}

#[derive(Debug)]
pub enum Action {
    GetPageList {
        manga_id: i32,
        chapter_number: String,
    },
    EmitListUpdate,
}

pub struct PageAgent {
    link: AgentLink<PageAgent>,
    fetch_task: Option<FetchTask>,
    pages: Option<Rc<Vec<Page>>>,
    subscribers: HashSet<HandlerId>,
}

impl Agent for PageAgent {
    type Reach = Context<Self>;
    type Message = Msg;
    type Input = Action;
    type Output = Rc<Vec<Page>>;

    fn create(link: AgentLink<Self>) -> Self {
        Self {
            link,
            fetch_task: None,
            pages: None,
            subscribers: HashSet::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) {
        trace!("{:?}", msg);
        match msg {
            Msg::Error => {}
            Msg::FetchPageComplete { pages } => {
                self.pages = Some(Rc::new(pages));
                self.link.send_input(Action::EmitListUpdate);
            }
        }
    }

    fn handle_input(&mut self, action: Self::Input, requester: HandlerId) {
        match action {
            Action::GetPageList {
                manga_id,
                chapter_number,
            } => {
                if let Some(pages) = &self.pages {
                    self.link.respond(requester, Rc::clone(&pages));
                } else if self.fetch_task.is_none() {
                    self.begin_data_fetch(manga_id, &chapter_number)
                }
            }
            Action::EmitListUpdate => {
                if let Some(pages) = &self.pages {
                    for sub in self.subscribers.iter() {
                        self.link.respond(*sub, Rc::clone(&pages));
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

impl PageAgent {
    fn begin_data_fetch(&mut self, manga_id: i32, chapter_number: &str) {
        let task = self
            .get_request_chapter_task(
                PageAgent::parse_chapter_fetch_to_msg,
                manga_id,
                chapter_number,
            )
            .expect("Failed to build request");
        self.fetch_task = Some(task);
    }

    fn parse_chapter_fetch_to_msg(
        response: Response<Json<Result<Vec<Page>, anyhow::Error>>>,
    ) -> Msg {
        let Json(data) = response.into_body();
        match data {
            Ok(pages) => Msg::FetchPageComplete { pages },
            Err(error) => {
                error!("{}", error);
                Msg::Error
            }
        }
    }

    fn get_request_chapter_task(
        &self,
        result_handler: fn(Response<Json<Result<Vec<Page>, anyhow::Error>>>) -> Msg,
        manga_id: i32,
        chapter_number: &str,
    ) -> Result<FetchTask, anyhow::Error> {
        let request = Request::get(format!(
            "http://localhost:42069/manga/{}/{}",
            manga_id, chapter_number
        ))
        .body(Nothing)
        .expect("Could not build request.");
        let callback = self.link.callback(result_handler);
        FetchService::fetch(request, callback)
    }
}
