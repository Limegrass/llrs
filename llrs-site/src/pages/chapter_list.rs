use crate::agents::chapter::{Action, ChapterAgent};
use crate::app::AppRoute;
use llrs_model::Chapter;
use log::*;
use std::rc::Rc;
use yew::{prelude::*, Component, ComponentLink};
use yew_router::components::RouterAnchor;

pub struct State {
    chapters: Option<Rc<Vec<Chapter>>>,
    #[allow(dead_code)]
    chapter_agent: Box<dyn Bridge<ChapterAgent>>,
}

pub struct ChapterList {
    state: State,
}

#[derive(Debug)]
pub enum Msg {
    FetchChaptersComplete(Rc<Vec<Chapter>>),
}

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct Props {
    pub manga_id: i32,
}

impl Component for ChapterList {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut chapter_agent = ChapterAgent::bridge(link.callback(Msg::FetchChaptersComplete));
        chapter_agent.send(Action::GetChapterList {
            manga_id: props.manga_id,
        });
        let state = State {
            chapters: None,
            chapter_agent,
        };

        Self { state }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        trace!("{:?}", msg);
        match msg {
            Msg::FetchChaptersComplete(data) => self.state.chapters = Some(data),
        }
        true
    }

    fn view(&self) -> Html {
        match &self.state.chapters {
            Some(chapters) => html! {
                for chapters.iter().map(|val| self.chapter_entry(&val))
            },
            None => html! {"Fetching"},
        }
    }
}

impl ChapterList {
    fn chapter_entry(&self, chapter: &Chapter) -> Html {
        type Anchor = RouterAnchor<AppRoute>;
        html! {
            <div class="chapter">
                <Anchor route=AppRoute::MangaChapter {
                    manga_id: chapter.manga_id,
                    chapter_number: chapter.chapter_number.to_owned(),
                }>
                    {&chapter.chapter_number}
                </Anchor>
            </div>
        }
    }
}
