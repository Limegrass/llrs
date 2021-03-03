use llrs_model::Manga;
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
    FetchMangaComplete { mangas: Vec<Manga> },
    Error(anyhow::Error),
    CleanUpFetchTasks,
}

#[derive(Debug, PartialEq, Hash)]
pub(crate) enum Action {
    GetMangaList,
    EmitMangaList,
}
impl Eq for Action {}

// Use a Rc<RefCell<Option<HashMap<i32, Rc<Manga>>>>>?
// EmitListUpdate just tells subscribers they can refresh if they want
pub(crate) struct MangaAgent {
    link: AgentLink<MangaAgent>,
    fetch_tasks: HashMap<Action, FetchTask>,
    #[allow(dead_code)]
    clean_up_job: Box<dyn Task>,
    manga_map: Option<Rc<HashMap<i32, Manga>>>,
    subscribers_map: HashMap<Action, HashSet<HandlerId>>,
}

#[derive(Debug)]
pub(crate) enum Response {
    MangaMap { mangas: Rc<HashMap<i32, Manga>> },
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
                self.link.send_input(Action::EmitMangaList);
            }
            Msg::CleanUpFetchTasks => self.fetch_tasks.retain(|_, task| task.is_active()),
        }
    }

    fn handle_input(&mut self, action: Self::Input, requester: HandlerId) {
        match action {
            Action::GetMangaList => {
                if let Some(mangas) = &self.manga_map {
                    let response = Response::MangaMap {
                        mangas: Rc::clone(mangas),
                    };
                    self.link.respond(requester, response);
                } else if self.fetch_tasks.get(&action).is_none() {
                    if let Err(error) = self.fetch_manga_list(requester) {
                        error!("{}", error)
                    }
                } // else wait for EmitListUpdate to trigger from existing fetch_task
                if let Some(subscribers) = self.subscribers_map.get_mut(&action) {
                    subscribers.insert(requester);
                }
            }
            Action::EmitMangaList => {
                if let Some(mangas) = &self.manga_map {
                    if let Some(subscribers) = self.subscribers_map.get_mut(&Action::GetMangaList) {
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
        }
    }
}

impl MangaAgent {
    fn fetch_manga_list(&mut self, _: HandlerId) -> Result<(), anyhow::Error> {
        let request = FetchRequest::get("http://localhost:42069").body(Nothing)?;
        let callback = self.link.callback(parse_manga_list_response);
        let task = FetchService::fetch(request, callback)?;
        self.fetch_tasks.insert(Action::GetMangaList, task);
        Ok(())
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
