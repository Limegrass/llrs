use super::progress::progress_bar;
use crate::agents::{
    manga::{Action as MangaAction, MangaAgent, Response as MangaAgentResponse},
    user::{Action as UserAgentAction, Response as UserAgentResponse, UserAgent},
};
use crate::route::AppRoute;
use llrs_model::{Chapter, Page};
use log::*;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, cmp::max, collections::VecDeque, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlImageElement, ScrollBehavior, ScrollToOptions, Window};
use yew::{agent::Bridge, prelude::*, Component, ComponentLink};
use yew_router::{
    agent::RouteRequest,
    prelude::{Route, RouteAgentDispatcher},
    switch::Permissive,
};

pub(crate) struct State {
    pages: Option<Rc<Vec<Page>>>,
    chapters: Option<Rc<Vec<Chapter>>>,
    view_format: ViewFormat,
    should_set_to_last_page: bool,
    preload_queue: VecDeque<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum ViewFormat {
    Single,
    Long,
}

pub(crate) struct MangaPage {
    #[allow(dead_code)]
    manga_agent: Box<dyn Bridge<MangaAgent>>,
    #[allow(dead_code)]
    user_agent: Box<dyn Bridge<UserAgent>>,
    route_dispatcher: RouteAgentDispatcher,
    prefetcher: Option<HtmlImageElement>,
    window: Option<Window>,
    state: State,
    props: Props,
    link: ComponentLink<Self>,
}

#[derive(Debug)]
pub(crate) enum Msg {
    PreloadImage { page_index: usize },
    MangaAgentResponse(MangaAgentResponse),
    UserAgentResponse(UserAgentResponse),
    PageBack { current_page_number: usize },
    PageForward { current_page_number: usize },
    ScrollToPage { page_number: usize },
}

#[derive(Debug, Clone, PartialEq, Properties)]
pub(crate) struct Props {
    pub(crate) manga_id: i32,
    pub(crate) chapter_number: String,
    pub(crate) page_number: usize,
}

impl Component for MangaPage {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        trace!("manga_page: {:?}", props);

        let mut manga_agent = MangaAgent::bridge(link.callback(Msg::MangaAgentResponse));
        manga_agent.send(MangaAction::GetChapterList {
            manga_id: props.manga_id,
        });
        manga_agent.send(MangaAction::GetPageList {
            manga_id: props.manga_id,
            chapter_number: props.chapter_number.to_owned(),
        });

        let mut user_agent = UserAgent::bridge(link.callback(Msg::UserAgentResponse));
        user_agent.send(UserAgentAction::GetViewFormatPreference);

        let state = State {
            chapters: None,
            pages: None,
            view_format: ViewFormat::Single,
            should_set_to_last_page: false,
            preload_queue: VecDeque::new(),
        };

        let route_dispatcher = RouteAgentDispatcher::new();
        let window = web_sys::window();

        Self {
            prefetcher: HtmlImageElement::new().ok(),
            route_dispatcher,
            manga_agent,
            state,
            props,
            link,
            window,
            user_agent,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if props.chapter_number != self.props.chapter_number {
            self.manga_agent.send(MangaAction::GetPageList {
                manga_id: props.manga_id,
                chapter_number: props.chapter_number.to_owned(),
            });
            self.props = props;
            // We don't need to rerender yet because
            // we can just wait until we get a response for the new list of pages
            false
        } else {
            self.link.send_message(Msg::ScrollToPage {
                page_number: props.page_number,
            });
            self.props = props;
            true
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        trace!("{:?}", msg);
        match msg {
            Msg::PreloadImage { page_index } => self.preload_image_and_set_next(page_index),
            Msg::MangaAgentResponse(response) => self.handle_manga_response(response),
            Msg::PageBack {
                current_page_number,
            } => {
                self.state.should_set_to_last_page = true;
                self.page_backward(current_page_number);
                false
            }
            Msg::PageForward {
                current_page_number,
            } => {
                self.state.should_set_to_last_page = false;
                self.page_forward(current_page_number);
                false
            }
            Msg::UserAgentResponse(response) => match response {
                UserAgentResponse::ViewFormatPreference(view_format) => {
                    if view_format != self.state.view_format {
                        self.state.view_format = view_format;
                        true
                    } else {
                        false
                    }
                }
            },
            Msg::ScrollToPage { page_number } => {
                self.scroll_to_manga_page_top(page_number);
                false
            }
        }
    }

    fn view(&self) -> Html {
        match &self.state.pages {
            Some(pages) => self.render_view(pages),
            None => progress_bar(),
        }
    }
}

// Check the Chapter Agent to see which chapter is next
impl MangaPage {
    fn scroll_to_manga_page_top(&self, page_number: usize) {
        if let Some(window) = self.window.as_ref() {
            if let Some(doc) = window.document() {
                let mut scroll_to_options = ScrollToOptions::new();
                let element_to_scroll_to_top = match self.state.view_format {
                    ViewFormat::Single => "manga-image".to_owned(),
                    ViewFormat::Long => format!("manga-page-{}", page_number),
                };
                let manga_page_top = doc
                    .get_element_by_id(element_to_scroll_to_top.as_str())
                    .map_or(0.0, |element| element.get_bounding_client_rect().top());
                scroll_to_options.top(manga_page_top);
                scroll_to_options.behavior(ScrollBehavior::Smooth);
                window.scroll_by_with_scroll_to_options(&scroll_to_options);
            }
        }
    }

    fn render_view(&self, pages: &[Page]) -> Html {
        if let Some(page_index) = self.props.page_number.checked_sub(1) {
            let pages = match self.state.view_format {
                // TODO: Progressive loading (first page first)
                ViewFormat::Long => html! {
                    for pages.iter().map(|page| self.manga_page(&page))
                },
                ViewFormat::Single => html! {
                    self.manga_page(&pages[page_index])
                },
            };
            html! {
                <figure class="container image">
                    {pages}
                </figure>
            }
        } else {
            // THIS SHOULD really be a PANIC BUT WASM SIZES :)
            html! {"Somehow had a page_number of 0"}
        }
    }

    fn manga_page(&self, page: &Page) -> Html {
        let current_page_number = page.page_number as usize;

        html! {
            <div id=format!("manga-page-{}", page.page_number) class="container">
                <div class="back-pager"
                    onclick=self.link.callback(move |_| Msg::PageBack { current_page_number }) />
                <div class="forward-pager"
                    onclick=self.link.callback(move |_| Msg::PageForward { current_page_number }) />
                <img id="manga-image"
                     src=&page.url_string
                     alt=format!("Page {} Image", &page.page_number)
                 />
            </div>
        }
    }

    fn preload_image_and_set_next(&mut self, page_index: usize) -> bool {
        match self.state.pages.as_ref() {
            Some(pages) if pages.len() > 0 => {
                if let (Some(page), Some(image_element)) = (pages.get(page_index), &self.prefetcher)
                {
                    let load_next_page_closure =
                        self.state.preload_queue.pop_front().map(|next_page_index| {
                            let link = self.link.clone();
                            Closure::once(Box::new(move || {
                                link.send_message(Msg::PreloadImage {
                                    page_index: next_page_index,
                                });
                            }))
                        });
                    if let Some(closure) = load_next_page_closure {
                        image_element.set_onload(Some(closure.as_ref().unchecked_ref()));
                        // TODO: Research can this still leak memory if the image doesn't load
                        // before the page gets destroyed, not sure given that
                        // the HtmlImageElement should get cleaned up.
                        closure.forget();
                    } else {
                        image_element.set_onload(None);
                    }
                    image_element.set_src(&page.url_string);
                    match self.state.view_format {
                        ViewFormat::Single => false,
                        ViewFormat::Long => true,
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn handle_manga_response(&mut self, response: MangaAgentResponse) -> ShouldRender {
        match response {
            MangaAgentResponse::Chapters { manga_id, chapters } => {
                self.props.manga_id = manga_id;
                self.state.chapters = Some(chapters);
                false
            }
            MangaAgentResponse::Pages {
                manga_id,
                chapter_number,
                pages,
            } => {
                self.props.manga_id = manga_id;
                self.props.chapter_number = chapter_number;
                let route =
                    // also catches people url hacking to a big number
                    if self.state.should_set_to_last_page || pages.len() < self.props.page_number {
                        Some(AppRoute::MangaChapterPage {
                            manga_id: self.props.manga_id,
                            chapter_number: self.props.chapter_number.to_owned(),
                            page_number: pages.len(),
                        })
                    } else if pages.len() == 0 {
                        Some(AppRoute::NotFound(Permissive(Some(format!(
                            "Manga with ID {} and Chapter {} not found",
                            self.props.manga_id, self.props.chapter_number
                        )))))
                    } else if self.props.page_number == 0 {
                        // only to catch people url hacking to 0
                        Some(AppRoute::MangaChapter {
                            manga_id: self.props.manga_id,
                            chapter_number: self.props.chapter_number.to_owned(),
                        })
                    } else {
                        None
                    };

                // Reset queue and load up new preloads
                // from current page to last, then current to first
                self.state.preload_queue.clear();
                let starting_page_index = self.props.page_number.checked_sub(1).unwrap_or(0);
                for page_number in starting_page_index..pages.len() {
                    self.state.preload_queue.push_back(page_number);
                }
                for page_number in (0..starting_page_index).rev() {
                    self.state.preload_queue.push_back(page_number);
                }

                // kick off initial preload
                if let Some(page_index) = self.state.preload_queue.pop_front() {
                    self.link.send_message(Msg::PreloadImage { page_index });
                }

                self.state.pages = Some(pages);
                if let Some(route) = route {
                    self.route_dispatcher
                        .send(RouteRequest::ChangeRoute(Route::from(route)));
                    false
                } else {
                    self.link.send_message(Msg::ScrollToPage {
                        page_number: self.props.page_number,
                    });
                    true
                }
            }
            _ => false,
        }
    }

    fn page_backward(&mut self, current_page_number: usize) {
        let current_chapter_number = self.props.chapter_number.to_owned();
        let previous_page_chapter_number = if current_page_number == 1 {
            self.state
                .chapters
                .as_ref()
                .map_or(current_chapter_number.to_owned(), |chapter_list| {
                    get_previous_chapter_number(chapter_list, current_chapter_number.to_owned())
                })
        } else {
            self.props.chapter_number.to_owned()
        };

        if current_chapter_number != previous_page_chapter_number {
            // We send a message to the agent to fetch the page list
            // because we want to put it on the last page and not the first
            self.manga_agent.send(MangaAction::GetPageList {
                manga_id: self.props.manga_id,
                chapter_number: previous_page_chapter_number.to_owned(),
            });
        } else {
            let previous_page_number = max(
                1,
                current_page_number
                    .checked_sub(1)
                    .unwrap_or(current_page_number),
            );
            let route = AppRoute::MangaChapterPage {
                manga_id: self.props.manga_id,
                chapter_number: self.props.chapter_number.to_owned(),
                page_number: previous_page_number,
            };
            self.route_dispatcher
                .send(RouteRequest::ChangeRoute(Route::from(route)));
        }
    }

    fn page_forward(&mut self, current_page_number: usize) {
        let last_page = self
            .state
            .pages
            .as_ref()
            .map_or(self.props.page_number, |pages| pages.len());
        let current_chapter_number = self.props.chapter_number.to_owned();

        let next_page_chapter_number = if current_page_number == last_page {
            self.state
                .chapters
                .as_ref()
                .map_or(current_chapter_number.to_owned(), |chapter_list| {
                    get_next_chapter_number(chapter_list, current_chapter_number)
                })
        } else {
            self.props.chapter_number.to_owned()
        };

        let route = if next_page_chapter_number == self.props.chapter_number {
            let next_page_number = current_page_number
                .checked_add(1)
                .unwrap_or(self.props.page_number);
            if next_page_number > last_page {
                AppRoute::ChapterList {
                    manga_id: self.props.manga_id,
                }
            } else {
                AppRoute::MangaChapterPage {
                    manga_id: self.props.manga_id,
                    chapter_number: next_page_chapter_number.to_owned(),
                    page_number: next_page_number,
                }
            }
        } else {
            AppRoute::MangaChapterPage {
                manga_id: self.props.manga_id,
                chapter_number: next_page_chapter_number.to_owned(),
                page_number: 1,
            }
        };

        self.route_dispatcher
            .send(RouteRequest::ChangeRoute(Route::from(route)));
    }
}

fn get_next_chapter_number(
    chapter_list: &Rc<Vec<Chapter>>,
    current_chapter_number: String,
) -> String {
    let mut iter = chapter_list.iter();
    let last: RefCell<Option<&Chapter>> = RefCell::new(chapter_list.first());
    while let Some(chapter) = iter.next() {
        if chapter.chapter_number == current_chapter_number {
            return iter.next().map_or(
                last.into_inner()
                    .map_or(current_chapter_number.to_owned(), |ch| {
                        ch.chapter_number.to_owned()
                    }),
                |ch| ch.chapter_number.to_owned(),
            );
        }
        last.replace(Some(chapter));
    }
    last.into_inner()
        .map_or(current_chapter_number.to_owned(), |ch| {
            ch.chapter_number.to_owned()
        })
}

fn get_previous_chapter_number(
    chapter_list: &Rc<Vec<Chapter>>,
    current_chapter_number: String,
) -> String {
    let mut iter = chapter_list.iter();
    let prev: RefCell<Option<&Chapter>> = RefCell::new(chapter_list.first());
    while let Some(chapter) = iter.next() {
        if chapter.chapter_number == current_chapter_number {
            return prev
                .into_inner()
                .map_or(current_chapter_number.to_owned(), |ch| {
                    ch.chapter_number.to_owned()
                });
        }
        prev.replace(Some(chapter));
    }
    prev.into_inner()
        .map_or(current_chapter_number.to_owned(), |ch| {
            ch.chapter_number.to_owned()
        })
}
