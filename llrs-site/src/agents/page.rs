use llrs_model::Page;
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
    FetchPageComplete {
        pages: Vec<Page>,
        manga_id: i32,
        chapter_number: String,
    },
    Error(anyhow::Error),
}

#[derive(Debug)]
pub(crate) enum Action {
    GetPageList {
        manga_id: i32,
        chapter_number: String,
    },
    EmitListUpdate {
        manga_id: i32,
        chapter_number: String,
    },
}

type DataKey = (i32, String);
pub(crate) struct PageAgent {
    link: AgentLink<PageAgent>,
    fetch_tasks: HashMap<DataKey, FetchTask>,
    chapter_pages: HashMap<DataKey, Rc<Vec<Page>>>,
    subscribers: HashMap<HandlerId, DataKey>,
}

impl Agent for PageAgent {
    type Reach = Context<Self>;
    type Message = Msg;
    type Input = Action;
    type Output = Rc<Vec<Page>>;

    fn create(link: AgentLink<Self>) -> Self {
        Self {
            link,
            fetch_tasks: HashMap::new(),
            chapter_pages: HashMap::new(),
            subscribers: HashMap::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) {
        trace!("{:?}", msg);
        match msg {
            Msg::Error(error) => error!("{}", error),
            Msg::FetchPageComplete {
                pages,
                manga_id,
                chapter_number,
            } => {
                let key = (manga_id, chapter_number.to_owned());
                self.chapter_pages.insert(key, Rc::new(pages));
                self.link.send_input(Action::EmitListUpdate {
                    manga_id,
                    chapter_number,
                });
            }
        }
    }

    fn handle_input(&mut self, action: Self::Input, requester: HandlerId) {
        trace!("{:?}", action);
        match action {
            Action::GetPageList {
                manga_id,
                chapter_number,
            } => {
                let key = (manga_id, chapter_number.to_owned());
                if let Some(pages) = &self.chapter_pages.get(&key) {
                    self.link.respond(requester, Rc::clone(&pages));
                } else if self.fetch_tasks.get(&key).is_none() {
                    match self.build_fetch_task(manga_id, chapter_number) {
                        Ok(fetch_task) => {
                            self.fetch_tasks.insert(key.clone(), fetch_task);
                        }
                        Err(error) => error!("{}", error),
                    }
                }
                self.subscribers.insert(requester, key);
            }
            Action::EmitListUpdate {
                manga_id,
                chapter_number,
            } => {
                let key = (manga_id, chapter_number.to_owned());
                if let Some(pages) = &self.chapter_pages.get(&key) {
                    for (sub, subscribed_data_key) in self.subscribers.iter() {
                        if *subscribed_data_key == key {
                            self.link.respond(*sub, Rc::clone(&pages));
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

impl PageAgent {
    fn build_fetch_task(
        &mut self,
        manga_id: i32,
        chapter_number: String,
    ) -> Result<FetchTask, anyhow::Error> {
        let request = Request::get(format!(
            "http://localhost:42069/manga/{}/{}",
            manga_id, chapter_number
        ))
        .body(Nothing)?;
        let callback = self.link.callback(
            move |response: Response<Json<Result<Vec<Page>, anyhow::Error>>>| {
                let Json(data) = response.into_body();
                match data {
                    Ok(pages) => Msg::FetchPageComplete {
                        pages,
                        manga_id,
                        chapter_number: chapter_number.to_owned(),
                    },
                    Err(error) => Msg::Error(error),
                }
            },
        );
        FetchService::fetch(request, callback)
    }
}
