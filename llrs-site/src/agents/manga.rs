use llrs_model::{Chapter, Manga};
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
    Error(anyhow::Error),
    CleanUpFetchTasks,
    FetchChapterComplete {
        chapters: Vec<Chapter>,
        manga_id: i32,
    },
    EmitFetchComplete {
        action: Action,
    },
}

#[derive(Debug, PartialEq, Hash, Clone)]
pub(crate) enum Action {
    GetChapterList { manga_id: i32 },
    GetMangaList,
}
impl Eq for Action {}

// Use a Rc<RefCell<Option<HashMap<i32, Rc<Manga>>>>>?
// EmitListUpdate just tells subscribers they can refresh if they want
pub(crate) struct MangaAgent {
    chapters: HashMap<i32, Rc<Vec<Chapter>>>,
    link: AgentLink<MangaAgent>,
    fetch_tasks: HashMap<Action, FetchTask>,
    #[allow(dead_code)]
    clean_up_job: Box<dyn Task>,
    manga_map: Option<Rc<HashMap<i32, Manga>>>,
    subscribers_map: HashMap<Action, HashSet<HandlerId>>,
}

#[derive(Debug)]
pub(crate) enum Response {
    MangaMap {
        mangas: Rc<HashMap<i32, Manga>>,
    },
    Chapters {
        manga_id: i32,
        chapters: Rc<Vec<Chapter>>,
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
            Msg::EmitFetchComplete { action } => match action {
                Action::GetMangaList => {
                    if let Some(mangas) = &self.manga_map {
                        if let Some(subscribers) = self.subscribers_map.get_mut(&action) {
                            for sub in subscribers.iter() {
                                let response = Response::MangaMap {
                                    mangas: Rc::clone(mangas),
                                };
                                self.link.respond(*sub, response);
                            }
                            subscribers.clear();
                        }
                    }
                }
                Action::GetChapterList { manga_id } => {
                    if let Some(chapters) = self.chapters.get(&manga_id) {
                        if let Some(subscribers) = self.subscribers_map.get_mut(&action) {
                            for sub in subscribers.iter() {
                                let response = Response::Chapters {
                                    manga_id,
                                    chapters: Rc::clone(chapters),
                                };
                                self.link.respond(*sub, response);
                            }
                            subscribers.clear();
                        }
                    }
                }
            },
            Msg::FetchChapterComplete { chapters, manga_id } => {
                self.chapters.insert(manga_id, Rc::new(chapters));
                self.link.send_message(Msg::EmitFetchComplete {
                    action: Action::GetChapterList { manga_id },
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
                } else if self.fetch_tasks.get(&input).is_none() {
                    match self.fetch_manga_list() {
                        Ok(fetch_task) => {
                            self.fetch_tasks.insert(input.clone(), fetch_task);
                        }
                        Err(error) => error!("{}", error),
                    }
                } // else wait for EmitListUpdate to trigger from existing fetch_task
                if let Some(subscribers) = self.subscribers_map.get_mut(&input) {
                    subscribers.insert(requester);
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
                } else if self.fetch_tasks.get(&input).is_none() {
                    match self.fetch_chapter_list(manga_id) {
                        Ok(fetch_task) => {
                            self.fetch_tasks.insert(input.clone(), fetch_task);
                        }
                        Err(error) => error!("{}", error),
                    }
                };

                let subscribers = self.subscribers_map.entry(input).or_insert(HashSet::new());

                subscribers.insert(requester);
            }
        }
    }
}

impl MangaAgent {
    fn fetch_manga_list(&mut self) -> Result<FetchTask, anyhow::Error> {
        let request = FetchRequest::get(env!("LLRS_MANGA_LIST_ENDPOINT")).body(Nothing)?;
        let callback = self.link.callback(parse_manga_list_response);
        Ok(FetchService::fetch(request, callback)?)
    }

    fn fetch_chapter_list(&mut self, manga_id: i32) -> Result<FetchTask, anyhow::Error> {
        let request = FetchRequest::get(format!(
            "{}/{}",
            env!("LLRS_CHAPTER_LIST_ENDPOINT"),
            manga_id
        ))
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
}

fn parse_manga_list_response(
    response: FetchResponse<Json<Result<Vec<Manga>, anyhow::Error>>>,
) -> Msg {
    let Json(data) = response.into_body();
    match data {
        Ok(mangas) => Msg::FetchMangaComplete { mangas },
        Err(error) => Msg::Error(error),
    }
}
