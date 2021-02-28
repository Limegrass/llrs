use crate::components::breadcrumb::{Breadcrumb, Separator};
use crate::pages::not_found;
use crate::pages::{ChapterList, Home, MangaPage};
use log::trace;
use yew::prelude::*;
use yew_router::{components::RouterAnchor, prelude::*, switch::Permissive, Switch};

#[derive(Debug, Switch, PartialEq, Clone)]
pub enum AppRoute {
    #[to = "/manga/{manga_id}/{chapter_number}/{page}"]
    MangaChapter(i32, String, u32),
    #[to = "/manga/{manga_id}/{chapter_number}"]
    MangaChapterDefault(i32, String),
    #[to = "/manga/{manga_id}"]
    ChapterList(i32),
    #[to = "/page-not-found"]
    NotFound(Permissive<String>),
    #[to = "/!"]
    Home,
}

pub struct App {
    // link: ComponentLink<Self>,
}

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
        // Self { link }
        Self {}
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        true
    }

    fn view(&self) -> Html {
        let redirect =
            Router::redirect(|route: Route| AppRoute::NotFound(Permissive(Some(route.route))));
        let render = Router::render(|route: AppRoute| {
            trace!("Route: {:?}", route);
            let content = match route {
                AppRoute::Home => html! {
                    <Home/>
                },
                AppRoute::ChapterList(manga_id) => html! {
                    <ChapterList manga_id=manga_id />
                },
                AppRoute::MangaChapter(manga_id, chapter_number, page_number) => html! {
                    <MangaPage
                        manga_id=manga_id
                        chapter_number=chapter_number
                        page_number=page_number
                    />
                },
                AppRoute::MangaChapterDefault(manga_id, chapter_number) => html! {
                    <MangaPage
                        manga_id=manga_id
                        chapter_number=chapter_number
                        page_number=1
                    />
                },
                AppRoute::NotFound(Permissive(None)) => html! { not_found("") },
                AppRoute::NotFound(Permissive(Some(path))) => html! { not_found(&path) },
            };
            html! {
                <div class="container">
                    <Breadcrumb separator=Separator::Succeeds >
                        <a href="/">
                            {"🏠 llrs"}
                        </a>
                        <a href="#">{"prolly use agents for this"}</a>
                    </Breadcrumb>
                    {content}
                </div>
            }
        });

        html! {
            <Router<AppRoute, ()> render=render redirect=redirect />
        }
    }
}
