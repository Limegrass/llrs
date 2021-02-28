use crate::pages::not_found;
use crate::pages::{ChapterList, Home, MangaPage};
use log::*;
use yew::prelude::*;
use yew_router::{prelude::*, switch::Permissive, Switch};

#[derive(Debug, Switch, PartialEq, Clone)]
pub enum AppRoute {
    #[to = "/manga/{manga_id}/{chapter_number}/{page}"]
    MangaChapter(i32, String, u32),
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
            info!("{:?}", route);
            match route {
                AppRoute::Home => html! {<Home/>},
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
                AppRoute::NotFound(Permissive(None)) => html! { not_found("") },
                AppRoute::NotFound(Permissive(Some(path))) => html! { not_found(&path) },
            }
        });

        info!("rendered!");
        html! {
            <Router<AppRoute, ()> render=render redirect=redirect />
        }
    }
}

impl App {}
