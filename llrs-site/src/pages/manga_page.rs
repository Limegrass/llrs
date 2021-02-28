use crate::agents::page::{Action, PageAgent};
use crate::app::AppRoute;
use llrs_model::Page;
use log::*;
use std::{cmp::min, rc::Rc};
use yew::{prelude::*, Component, ComponentLink};
use yew_router::components::RouterAnchor;

pub struct State {
    pages: Option<Rc<Vec<Page>>>,
    view_format: ViewFormat,
    #[allow(dead_code)]
    chapter_agent: Box<dyn Bridge<PageAgent>>,
}

enum ViewFormat {
    Single,
    Long,
}

impl State {}

pub struct MangaPage {
    state: State,
    props: Props,
}

#[derive(Debug)]
pub enum Msg {
    FetchPagesComplete(Rc<Vec<Page>>),
    LoadPage { page_number: usize },
}

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct Props {
    pub manga_id: i32,
    pub chapter_number: String,
    pub page_number: usize,
}

impl Component for MangaPage {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut page_agent = PageAgent::bridge(link.callback(Msg::FetchPagesComplete));
        page_agent.send(Action::GetPageList {
            manga_id: props.manga_id,
            chapter_number: props.chapter_number.to_owned(),
        });

        let state = State {
            pages: None,
            view_format: ViewFormat::Single,
            chapter_agent: page_agent,
        };

        Self { state, props }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.update(Msg::LoadPage {
            page_number: props.page_number,
        })
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        trace!("{:?}", msg);
        match msg {
            Msg::FetchPagesComplete(data) => {
                self.state.pages = Some(data);
                true
            }
            Msg::LoadPage { page_number } => {
                let last_page = self
                    .state
                    .pages
                    .as_ref()
                    .expect("Should never try render without pages")
                    .len();
                self.props.page_number = min(last_page, page_number);
                page_number <= last_page
            }
        }
    }

    fn view(&self) -> Html {
        trace!("rendered manga_page");
        match &self.state.pages {
            Some(pages) => self.render_view(pages),
            None => html! {"Fetching"},
        }
    }
}

impl MangaPage {
    fn render_view(&self, pages: &[Page]) -> Html {
        let page_index = self.props.page_number - 1;
        let pages = match self.state.view_format {
            // TODO: Progressive loading (first page first)
            ViewFormat::Long => html! {
                for pages.iter().map(|val| self.manga_page(&val))
            },
            ViewFormat::Single => html! {
                self.manga_page(&pages[page_index])
            },
        };
        html! {
            <div classes="container">
                {pages}
            </div>
        }
    }

    fn manga_page(&self, page: &Page) -> Html {
        // TODO: Look into an alternative to format!
        let next_page_number = self.props.page_number + 1;
        type Anchor = RouterAnchor<AppRoute>;
        html! {
            <Anchor route=AppRoute::MangaChapterPage{
                manga_id: self.props.manga_id,
                chapter_number: self.props.chapter_number.to_owned(),
                page_number: next_page_number
            }>
                <img src=&page.url_string
                     alt=format!("Page {} Image", &page.page_number)
                 />
            </Anchor>
        }
    }
}
