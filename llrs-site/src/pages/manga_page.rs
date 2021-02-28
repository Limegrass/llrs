use crate::agents::page::{Action, PageAgent};
use crate::app::AppRoute;
use llrs_model::Page;
use log::*;
use std::rc::Rc;
use yew::{prelude::*, Component, ComponentLink};
use yew_router::components::RouterAnchor;

pub struct State {
    pages: Option<Rc<Vec<Page>>>,
    current_page_number: usize,
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
    link: ComponentLink<Self>,
    state: State,
}

#[derive(Debug)]
pub enum Msg {
    FetchPagesComplete(Rc<Vec<Page>>),
    LoadPage(usize),
}

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct Props {
    pub manga_id: i32,
    pub chapter_number: String,
    pub page_number: u32,
}

impl Component for MangaPage {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut chapter_agent = PageAgent::bridge(link.callback(Msg::FetchPagesComplete));
        chapter_agent.send(Action::GetPageList {
            manga_id: props.manga_id,
            chapter_number: props.chapter_number.to_owned(),
        });

        let state = State {
            pages: None,
            view_format: ViewFormat::Single,
            current_page_number: 1,
            chapter_agent,
        };

        Self { link, state }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        trace!("{:?}", msg);
        match msg {
            Msg::FetchPagesComplete(data) => self.state.pages = Some(data),
            Msg::LoadPage(page_number) => self.state.current_page_number = page_number,
        }
        true
    }

    fn view(&self) -> Html {
        info!("rendered!");
        match &self.state.pages {
            Some(pages) => self.render_view(pages),
            None => html! {"Fetching"},
        }
    }
}

impl MangaPage {
    fn render_view(&self, pages: &[Page]) -> Html {
        let page_index = self.state.current_page_number - 1;
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
        let next_page_number = 1usize + page.page_number as usize;
        html! {
            <img src=&page.url_string
                 alt=format!("Page {} Image", &page.page_number)
                 onclick=self.link.callback(move |_| Msg::LoadPage(next_page_number))
             />
        }
    }
}
