use llrs_model::Chapter;
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
    FetchChapterComplete { chapters: Vec<Chapter> },
    Error,
}

#[derive(Debug)]
pub enum Action {
    GetChapterList(i32),
    EmitListUpdate,
}

pub struct ChapterAgent {
    link: AgentLink<ChapterAgent>,
    fetch_task: Option<FetchTask>,
    chapters: Option<Rc<Vec<Chapter>>>,
    subscribers: HashSet<HandlerId>,
}

impl Agent for ChapterAgent {
    type Reach = Context<Self>;
    type Message = Msg;
    type Input = Action;
    type Output = Rc<Vec<Chapter>>;

    fn create(link: AgentLink<Self>) -> Self {
        Self {
            link,
            fetch_task: None,
            chapters: None,
            subscribers: HashSet::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) {
        trace!("{:?}", msg);
        match msg {
            Msg::Error => {}
            Msg::FetchChapterComplete { chapters } => {
                self.chapters = Some(Rc::new(chapters));
                self.link.send_input(Action::EmitListUpdate);
            }
        }
    }

    fn handle_input(&mut self, action: Self::Input, requester: HandlerId) {
        match action {
            Action::GetChapterList(manga_id) => {
                if let Some(chapters) = &self.chapters {
                    self.link.respond(requester, Rc::clone(&chapters));
                } else if self.fetch_task.is_none() {
                    self.begin_data_fetch(manga_id)
                }
            }
            Action::EmitListUpdate => {
                if let Some(chapters) = &self.chapters {
                    for sub in self.subscribers.iter() {
                        self.link.respond(*sub, Rc::clone(&chapters));
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

impl ChapterAgent {
    fn begin_data_fetch(&mut self, manga_id: i32) {
        let task = self
            .get_request_chapter_task(ChapterAgent::parse_chapter_fetch_to_msg, manga_id)
            .expect("Failed to build request");
        self.fetch_task = Some(task);
    }

    fn parse_chapter_fetch_to_msg(
        response: Response<Json<Result<Vec<Chapter>, anyhow::Error>>>,
    ) -> Msg {
        let Json(data) = response.into_body();
        match data {
            Ok(chapters) => Msg::FetchChapterComplete { chapters },
            Err(error) => {
                error!("{}", error);
                Msg::Error
            }
        }
    }

    fn get_request_chapter_task(
        &self,
        result_handler: fn(Response<Json<Result<Vec<Chapter>, anyhow::Error>>>) -> Msg,
        manga_id: i32,
    ) -> Result<FetchTask, anyhow::Error> {
        let request = Request::get(format!("http://localhost:42069/manga/{}", manga_id))
            .body(Nothing)
            .expect("Could not build request.");
        let callback = self.link.callback(result_handler);
        FetchService::fetch(request, callback)
    }
}
