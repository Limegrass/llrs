use llrs_model::{Chapter, Manga, Page};
use log::*;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    time::Duration,
};
use yew::{
    format::{Json, Nothing},
    services::{
        fetch::{FetchTask, Request as FetchRequest, Response as FetchResponse},
        FetchService, IntervalService, Task,
    },
    worker::*,
};

#[derive(Debug)]
pub(crate) enum Msg {
    FetchMangaComplete {
        mangas: Vec<Manga>,
    },
    FetchChapterComplete {
        chapters: Vec<Chapter>,
        manga_id: i32,
    },
    FetchPageComplete {
        pages: Vec<Page>,
        manga_id: i32,
        chapter_number: String,
    },
    Error(anyhow::Error),
    CleanUpFetchTasks,
    EmitFetchComplete {
        action: Action,
    },
}

#[derive(Debug, PartialEq, Hash, Clone)]
pub(crate) enum Action {
    GetChapterList {
        manga_id: i32,
    },
    GetMangaList,
    GetPageList {
        manga_id: i32,
        chapter_number: String,
    },
}
impl Eq for Action {}

type DataKey = (i32, String);
// Use a Rc<RefCell<Option<HashMap<i32, Rc<Manga>>>>>?
// EmitListUpdate just tells subscribers they can refresh if they want
pub(crate) struct MangaAgent {
    chapter_pages: HashMap<DataKey, Rc<Vec<Page>>>,
    chapters: HashMap<i32, Rc<Vec<Chapter>>>,
    link: AgentLink<MangaAgent>,
    fetch_tasks: HashMap<Action, FetchTask>,
    #[allow(dead_code)]
    clean_up_job: Box<dyn Task>,
    manga_map: Option<Rc<HashMap<i32, Manga>>>,
    subscribers_map: HashMap<Action, HashSet<HandlerId>>,
}

#[derive(Debug, Clone)]
pub(crate) enum Response {
    MangaMap {
        mangas: Rc<HashMap<i32, Manga>>,
    },
    Chapters {
        manga_id: i32,
        chapters: Rc<Vec<Chapter>>,
    },
    Pages {
        manga_id: i32,
        chapter_number: String,
        pages: Rc<Vec<Page>>,
    },
}

// Keep what I have, change to a pure pull based data fetching,
// and only implement pub/sub style if needed later (not needed right now).
// Copy karaoke rs pull scheme
// Actually, using use_reducer and use_context seems like a way better option
// along with functional components.
impl Agent for MangaAgent {
    type Reach = Context<Self>;
    type Message = Msg;
    type Input = Action;
    type Output = Response;

    fn create(link: AgentLink<Self>) -> Self {
        let callback = link.callback(|_| Msg::CleanUpFetchTasks);
        let clean_up_job = IntervalService::spawn(Duration::from_millis(1000), callback);

        let subscribers_map: HashMap<Action, HashSet<HandlerId>> =
            vec![(Action::GetMangaList, HashSet::new())]
                .into_iter()
                .collect();
        Self {
            link,
            chapter_pages: HashMap::new(),
            chapters: HashMap::new(),
            fetch_tasks: HashMap::new(),
            manga_map: None,
            clean_up_job: Box::new(clean_up_job),
            subscribers_map,
        }
    }

    fn update(&mut self, msg: Self::Message) {
        trace!("{:?}", msg);
        match msg {
            Msg::Error(error) => error!("{}", error),
            Msg::FetchMangaComplete { mangas } => {
                let manga_map = mangas
                    .into_iter()
                    .map(|manga| (manga.manga_id, manga))
                    .collect::<HashMap<_, _>>();
                self.manga_map = Some(Rc::from(manga_map));
                self.link.send_message(Msg::EmitFetchComplete {
                    action: Action::GetMangaList,
                });
            }
            Msg::CleanUpFetchTasks => self.fetch_tasks.retain(|_, task| task.is_active()),
            Msg::EmitFetchComplete { action } => {
                let response = match action {
                    Action::GetMangaList => {
                        self.manga_map.as_ref().map(|mangas| Response::MangaMap {
                            mangas: Rc::clone(&mangas),
                        })
                    }
                    Action::GetChapterList { manga_id } => {
                        self.chapters
                            .get(&manga_id)
                            .map(|chapters| Response::Chapters {
                                manga_id,
                                chapters: Rc::clone(chapters),
                            })
                    }
                    Action::GetPageList {
                        manga_id,
                        ref chapter_number,
                    } => self
                        .chapter_pages
                        .get(&(manga_id, chapter_number.to_owned()))
                        .map(|pages| Response::Pages {
                            manga_id,
                            chapter_number: chapter_number.to_owned(),
                            pages: Rc::clone(pages),
                        }),
                };
                self.respond_and_remove_subs(&action, response);
            }
            Msg::FetchChapterComplete { chapters, manga_id } => {
                self.chapters.insert(manga_id, Rc::new(chapters));
                self.link.send_message(Msg::EmitFetchComplete {
                    action: Action::GetChapterList { manga_id },
                });
            }
            Msg::FetchPageComplete {
                pages,
                manga_id,
                chapter_number,
            } => {
                let key = (manga_id, chapter_number.to_owned());
                self.chapter_pages.insert(key, Rc::new(pages));
                self.link.send_message(Msg::EmitFetchComplete {
                    action: Action::GetPageList {
                        manga_id,
                        chapter_number,
                    },
                });
            }
        }
    }

    fn handle_input(&mut self, input: Self::Input, requester: HandlerId) {
        match input {
            Action::GetMangaList => {
                if let Some(mangas) = &self.manga_map {
                    let response = Response::MangaMap {
                        mangas: Rc::clone(mangas),
                    };
                    self.link.respond(requester, response);
                } else {
                    if self.fetch_tasks.get(&input).is_none() {
                        match self.fetch_manga_list() {
                            Ok(fetch_task) => {
                                self.fetch_tasks.insert(input.clone(), fetch_task);
                            }
                            Err(error) => self.link.send_message(Msg::Error(error)),
                        }
                    }
                    self.add_subscriber(input, requester);
                }
            }
            Action::GetChapterList { manga_id } => {
                if let Some(chapters) = self.chapters.get(&manga_id) {
                    self.link.respond(
                        requester,
                        Response::Chapters {
                            manga_id,
                            chapters: Rc::clone(chapters),
                        },
                    );
                } else {
                    if self.fetch_tasks.get(&input).is_none() {
                        match self.fetch_chapter_list(manga_id) {
                            Ok(fetch_task) => {
                                self.fetch_tasks.insert(input.clone(), fetch_task);
                            }
                            Err(error) => self.link.send_message(Msg::Error(error)),
                        }
                    }
                    self.add_subscriber(input, requester);
                };
            }
            Action::GetPageList {
                manga_id,
                ref chapter_number,
            } => {
                let key = (manga_id, chapter_number.to_owned());
                if let Some(pages) = &self.chapter_pages.get(&key) {
                    self.link.respond(
                        requester,
                        Response::Pages {
                            manga_id,
                            chapter_number: chapter_number.to_owned(),
                            pages: Rc::clone(&pages),
                        },
                    );
                } else if self.fetch_tasks.get(&input).is_none() {
                    match self.fetch_page_list(manga_id, chapter_number.to_owned()) {
                        Ok(fetch_task) => {
                            self.fetch_tasks.insert(input.clone(), fetch_task);
                        }
                        Err(error) => self.link.send_message(Msg::Error(error)),
                    }
                }
                self.add_subscriber(input, requester);
            }
        }
    }
}

impl MangaAgent {
    fn fetch_manga_list(&mut self) -> Result<FetchTask, anyhow::Error> {
        let request = FetchRequest::get(env!("LLRS_API_ENDPOINT")).body(Nothing)?;
        let callback = self.link.callback(
            |response: FetchResponse<Json<Result<Vec<Manga>, anyhow::Error>>>| {
                let Json(data) = response.into_body();
                match data {
                    Ok(mangas) => Msg::FetchMangaComplete { mangas },
                    Err(error) => Msg::Error(error),
                }
            },
        );

        Ok(FetchService::fetch(request, callback)?)
    }

    fn fetch_chapter_list(&mut self, manga_id: i32) -> Result<FetchTask, anyhow::Error> {
        let request =
            FetchRequest::get(format!("{}/manga/{}", env!("LLRS_API_ENDPOINT"), manga_id))
                .body(Nothing)?;
        let callback = self.link.callback(
            move |response: FetchResponse<Json<Result<Vec<Chapter>, anyhow::Error>>>| {
                let Json(data) = response.into_body();
                match data {
                    Ok(chapters) => Msg::FetchChapterComplete { chapters, manga_id },
                    Err(error) => Msg::Error(error),
                }
            },
        );
        Ok(FetchService::fetch(request, callback)?)
    }

    fn fetch_page_list(
        &mut self,
        manga_id: i32,
        chapter_number: String,
    ) -> Result<FetchTask, anyhow::Error> {
        let request = FetchRequest::get(format!(
            "{}/manga/{}/{}",
            env!("LLRS_API_ENDPOINT"),
            manga_id,
            chapter_number
        ))
        .body(Nothing)?;
        let callback = self.link.callback(
            move |response: FetchResponse<Json<Result<Vec<Page>, anyhow::Error>>>| {
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

    fn respond_and_remove_subs(&mut self, action: &Action, response: Option<Response>) {
        if let Some(response) = response {
            if let Some(subscribers) = self.subscribers_map.get_mut(action) {
                for sub in subscribers.iter() {
                    self.link.respond(*sub, response.clone());
                }
                subscribers.clear();
            }
        }
    }

    fn add_subscriber(&mut self, action: Action, requester: HandlerId) {
        let subscribers = self.subscribers_map.entry(action).or_insert(HashSet::new());
        subscribers.insert(requester);
    }
}
