use llrs_model::Chapter;
use log::*;
use std::{collections::HashMap, rc::Rc};
use yew::{
    format::{Json, Nothing},
    services::{
        fetch::{FetchTask, Request, Response},
        FetchService,
    },
    worker::*,
};

#[derive(Debug)]
pub(crate) enum Msg {
    FetchChapterComplete {
        manga_id: i32,
        chapters: Vec<Chapter>,
    },
    Error(anyhow::Error),
}

#[derive(Debug)]
pub(crate) enum Action {
    GetChapterList { manga_id: i32 },
    EmitListUpdate { manga_id: i32 },
}

// TODO: Consider a NewType for the i32s
pub(crate) struct ChapterAgent {
    link: AgentLink<ChapterAgent>,
    fetch_tasks: HashMap<i32, FetchTask>,
    chapters: HashMap<i32, Rc<Vec<Chapter>>>,
    subscribers: HashMap<HandlerId, i32>,
}

impl Agent for ChapterAgent {
    type Reach = Context<Self>;
    type Message = Msg;
    type Input = Action;
    type Output = Rc<Vec<Chapter>>;

    fn create(link: AgentLink<Self>) -> Self {
        Self {
            link,
            fetch_tasks: HashMap::new(),
            chapters: HashMap::new(),
            subscribers: HashMap::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) {
        trace!("{:?}", msg);
        match msg {
            Msg::Error(error) => error!("{}", error),
            Msg::FetchChapterComplete { manga_id, chapters } => {
                self.chapters.insert(manga_id, Rc::new(chapters));
                self.link.send_input(Action::EmitListUpdate { manga_id });
            }
        }
    }

    fn handle_input(&mut self, action: Self::Input, requester: HandlerId) {
        match action {
            Action::GetChapterList { manga_id } => {
                if let Some(chapters) = &self.chapters.get(&manga_id) {
                    self.link.respond(requester, Rc::clone(&chapters));
                } else if self.fetch_tasks.get(&manga_id).is_none() {
                    match self.build_fetch_task(manga_id) {
                        Ok(fetch_task) => {
                            self.fetch_tasks.insert(manga_id, fetch_task);
                        }
                        Err(error) => error!("{}", error),
                    }
                }
                self.subscribers.insert(requester, manga_id);
            }
            Action::EmitListUpdate { manga_id } => {
                if let Some(chapters) = &self.chapters.get(&manga_id) {
                    for (sub, subscribed_manga_id) in self.subscribers.iter() {
                        if *subscribed_manga_id == manga_id {
                            self.link.respond(*sub, Rc::clone(&chapters));
                        }
                    }
                }
            }
        }
    }

    fn disconnected(&mut self, id: HandlerId) {
        self.subscribers.remove(&id);
    }
}

impl ChapterAgent {
    fn build_fetch_task(&mut self, manga_id: i32) -> Result<FetchTask, anyhow::Error> {
        let request =
            Request::get(format!("http://localhost:42069/manga/{}", manga_id)).body(Nothing)?;
        let callback = self.link.callback(
            move |response: Response<Json<Result<Vec<Chapter>, anyhow::Error>>>| {
                let Json(data) = response.into_body();
                match data {
                    Ok(chapters) => Msg::FetchChapterComplete { manga_id, chapters },
                    Err(error) => Msg::Error(error),
                }
            },
        );
        FetchService::fetch(request, callback)
    }
}
