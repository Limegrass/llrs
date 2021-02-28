use std::rc::Rc;

use crate::agents::manga::{Action, MangaAgent};
use crate::app::AppRoute;
use llrs_model::Manga;
use log::*;
use yew::{prelude::*, Component, ComponentLink};
use yew_router::components::RouterAnchor;

pub struct State {
    mangas: Option<Rc<Vec<Manga>>>,
    #[allow(dead_code)]
    manga_agent: Box<dyn Bridge<MangaAgent>>,
}

impl State {}

pub struct Home {
    state: State,
}

#[derive(Debug)]
pub enum Msg {
    FetchMangasComplete(Rc<Vec<Manga>>),
}

impl Component for Home {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut manga_agent = MangaAgent::bridge(link.callback(Msg::FetchMangasComplete));
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
            Msg::FetchMangasComplete(data) => self.state.mangas = Some(data),
        }
        true
    }

    fn view(&self) -> Html {
        // TODO: On navigating to index,
        // call API (locally running libllrs)and display links to each
        let view = match &self.state.mangas {
            Some(mangas) => html! {
                {for mangas.chunks(2).map(|chunk| column_spread(chunk))}
            },
            None => html! {
                <progress max="100" class="progress is-primary" />
            },
        };
        html! {
            <div class="container">
                {view}
            </div>
        }
    }
}

/// Spreads a chunk as a set of columns
fn column_spread(mangas: &[Manga]) -> Html {
    html! {
        <div class="columns level">
            {for mangas.iter().map(|manga| as_column_level_item(manga_entry(manga)))}
        </div>
    }
}

fn manga_entry(manga: &Manga) -> Html {
    html! {
        <RouterAnchor<AppRoute> route=AppRoute::ChapterList(manga.manga_id)>
            <img src=&manga.cover_image_url alt=&manga.manga_name title=&manga.manga_name />
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
