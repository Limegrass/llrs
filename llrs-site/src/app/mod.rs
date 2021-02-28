mod app_navbar;

use crate::agents::manga::MangaAgent;
use crate::agents::{chapter::ChapterAgent, page::PageAgent};
use crate::pages::{not_found, ChapterList, MangaList, MangaPage};
use crate::route::AppRoute;
use app_navbar::AppNavbar;
use log::trace;
use yew::{agent::Dispatcher, prelude::*};
use yew_router::{prelude::*, switch::Permissive};

// We house the Agents here to persist the data inside of them
// Otherwise the Agents would get destroyed when the last bridge gets destructed.
pub(super) struct App {
    #[allow(dead_code)]
    manga_agent: Dispatcher<MangaAgent>,
    #[allow(dead_code)]
    chapter_agent: Dispatcher<ChapterAgent>,
    #[allow(dead_code)]
    page_agent: Dispatcher<PageAgent>,
}

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self {
            manga_agent: MangaAgent::dispatcher(),
            chapter_agent: ChapterAgent::dispatcher(),
            page_agent: PageAgent::dispatcher(),
        }
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let redirect =
            Router::redirect(|route: Route| AppRoute::NotFound(Permissive(Some(route.route))));
        let render = Router::render(|route: AppRoute| {
            trace!("Route: {:?}", &route);
            let content = render_main_content(&route);
            html! {
                <div class="container">
                    <AppNavbar route=&route />
                    {content}
                </div>
            }
        });

        html! {
            <Router<AppRoute, ()> render=render redirect=redirect />
        }
    }
}

fn render_main_content(route: &AppRoute) -> Html {
    match route {
        AppRoute::MangaList => html! {
            <MangaList />
        },
        AppRoute::ChapterList { manga_id } => html! {
            <ChapterList manga_id=manga_id />
        },
        AppRoute::MangaChapterPage {
            manga_id,
            chapter_number,
            page_number,
        } => html! {
            <MangaPage
                manga_id=manga_id
                chapter_number=chapter_number
                page_number=page_number
            />
        },
        AppRoute::MangaChapter {
            manga_id,
            chapter_number,
        } => html! {
            <MangaPage
                manga_id=manga_id
                chapter_number=chapter_number
                page_number=1
            />
        },
        AppRoute::NotFound(Permissive(unknown_path)) => {
            html! { not_found(unknown_path.as_ref().map_or("", |path| path.as_str())) }
        }
    }
}
