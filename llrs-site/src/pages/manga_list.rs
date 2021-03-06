use super::progress::progress_bar;
use crate::agents::manga::{Action, MangaAgent, Response};
use crate::route::AppRoute;
use llrs_model::Manga;
use log::*;
use std::{collections::HashMap, rc::Rc};
use yew::{prelude::*, Component, ComponentLink};
use yew_router::components::RouterAnchor;

pub(crate) struct State {
    mangas: Option<Rc<HashMap<i32, Manga>>>,
    #[allow(dead_code)]
    manga_agent: Box<dyn Bridge<MangaAgent>>,
}

impl State {}

pub(crate) struct MangaList {
    state: State,
}

#[derive(Debug)]
pub(crate) enum Msg {
    AgentResponse(Response),
}

impl Component for MangaList {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut manga_agent = MangaAgent::bridge(link.callback(Msg::AgentResponse));
        manga_agent.send(Action::GetMangaList);
        let state = State {
            mangas: None,
            manga_agent,
        };

        Self { state }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        trace!("{:?}", msg);
        match msg {
            Msg::AgentResponse(response) => match response {
                Response::MangaMap { mangas } => self.state.mangas = Some(mangas),
                _ => {}
            },
        }
        true
    }

    fn view(&self) -> Html {
        match &self.state.mangas {
            Some(mangas) => {
                let mangas = mangas
                    .iter()
                    .map(|(_, manga)| manga)
                    .collect::<Vec<&Manga>>();
                html! {
                    {for mangas.chunks(2).map(|chunk| column_spread(chunk))}
                }
            }
            None => progress_bar(),
        }
    }
}

/// Spreads a chunk as a set of columns
fn column_spread(mangas: &[&Manga]) -> Html {
    html! {
        <div class="columns level">
            {for mangas.iter().map(|manga| as_column_level_item(manga_entry(manga)))}
        </div>
    }
}

fn manga_entry(manga: &Manga) -> Html {
    html! {
        <RouterAnchor<AppRoute> route=AppRoute::ChapterList { manga_id: manga.manga_id }>
            <img class="image-link" src=&manga.cover_image_url alt=&manga.manga_name title=&manga.manga_name />
        </RouterAnchor<AppRoute>>
    }
}

fn as_column_level_item(html: Html) -> Html {
    html! {
        <div class="column level-item">
            {html}
        </div>
    }
}
