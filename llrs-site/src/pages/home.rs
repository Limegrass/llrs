use crate::app::AppRoute;
use llrs_model::Manga;
use log::error;
use yew::{
    format::{Json, Nothing},
    prelude::*,
    services::fetch::{FetchService, FetchTask, Request, Response},
    Component, ComponentLink,
};
use yew_router::components::RouterAnchor;

pub struct State {
    mangas: Option<Vec<Manga>>,
    fetch_task: Option<FetchTask>,
}

impl State {}

pub struct Home {
    state: State,
}

pub enum Msg {
    FetchMangasComplete(Result<Vec<Manga>, anyhow::Error>),
}

impl Component for Home {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let manga_request = Request::get("http://localhost:42069")
            .body(Nothing)
            .expect("Could not build request.");
        let manga_callback = link.callback(
            |response: Response<Json<Result<Vec<Manga>, anyhow::Error>>>| {
                let Json(data) = response.into_body();
                Msg::FetchMangasComplete(data)
            },
        );
        let task =
            FetchService::fetch(manga_request, manga_callback).expect("failed to start request");
        let state = State {
            mangas: None,
            fetch_task: Some(task),
        };

        Self { state }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FetchMangasComplete(data) => match data {
                Ok(mangas) => {
                    self.state.fetch_task = None;
                    self.state.mangas = Some(mangas)
                }
                Err(err) => error!("{}", err),
            },
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
