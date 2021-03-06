use super::progress::progress_bar;
use crate::agents::manga::{Action as MangaAction, MangaAgent, Response as MangaAgentResponse};
use crate::route::AppRoute;
use llrs_model::{Chapter, Page};
use log::*;
use std::{cell::RefCell, cmp::max, rc::Rc};
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
    is_loaded_page: Option<Vec<bool>>,
}

#[allow(dead_code)]
enum ViewFormat {
    Single,
    Long,
}

pub(crate) struct MangaPage {
    #[allow(dead_code)]
    manga_agent: Box<dyn Bridge<MangaAgent>>,
    route_dispatcher: RouteAgentDispatcher,
    prefetcher: Option<HtmlImageElement>,
    window: Option<Window>,
    state: State,
    props: Props,
    link: ComponentLink<Self>,
}

#[derive(Debug)]
pub(crate) enum Msg {
    PreloadNextImage { page_index: usize },
    MangaAgentResponse(MangaAgentResponse),
    PageBack,
    PageForward,
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

        let state = State {
            chapters: None,
            pages: None,
            view_format: ViewFormat::Single,
            should_set_to_last_page: false,
            is_loaded_page: None,
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
            self.props = props;
            true
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        trace!("{:?}", msg);
        match msg {
            Msg::PreloadNextImage { page_index } => {
                self.preload_image_and_set_next(page_index);
                false
            }
            Msg::MangaAgentResponse(response) => self.handle_manga_response(response),
            Msg::PageBack => {
                self.state.should_set_to_last_page = true;
                self.scroll_to_manga_page_top();
                self.page_backwards();
                false
            }
            Msg::PageForward => {
                self.state.should_set_to_last_page = false;
                self.scroll_to_manga_page_top();
                self.page_forwards();
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
    fn scroll_to_manga_page_top(&self) {
        let mut scroll_to_options = ScrollToOptions::new();
        let manga_page_top = self.window.as_ref().map_or(0.0, |window| {
            window.document().map_or(0.0, |doc| {
                doc.get_element_by_id("manga-image")
                    .map_or(0.0, |element| element.get_bounding_client_rect().top())
            })
        });
        scroll_to_options.top(manga_page_top);
        scroll_to_options.behavior(ScrollBehavior::Smooth);
        self.window.as_ref().and_then(|window| {
            window.scroll_by_with_scroll_to_options(&scroll_to_options);
            Some(window)
        });
    }

    fn render_view(&self, pages: &[Page]) -> Html {
        if let Some(page_index) = self.props.page_number.checked_sub(1) {
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
        info!("{:?}", self.state.is_loaded_page);
        if !self
            .state
            .is_loaded_page
            .as_ref()
            .map_or(true, |is_loaded_page| {
                *is_loaded_page
                    .get(page.page_number as usize)
                    .unwrap_or(&true)
            })
        {
            self.link.send_message(Msg::PreloadNextImage {
                page_index: page.page_number as usize, // do current page, which will be cached
            });
        }

        html! {
            <div class="container">
                <div class="back-pager"
                    onclick=self.link.callback(|_| Msg::PageBack) />
                <div class="forward-pager"
                    onclick=self.link.callback(|_| Msg::PageForward) />
                <img id="manga-image"
                     src=&page.url_string
                     alt=format!("Page {} Image", &page.page_number)
                 />
            </div>
        }
    }

    fn get_next_page_index(&self, page_number: usize) -> usize {
        let page_count = self.state.pages.as_ref().map_or(0, |pages| pages.len());

        // Since page_number is 1 indexed, we don't need math
        let next_page_index = page_number as usize;

        if next_page_index < page_count {
            next_page_index
        } else {
            0
        }
    }

    fn preload_image_and_set_next(&mut self, page_index: usize) {
        match self.state.pages.as_ref() {
            Some(pages) if pages.len() > 0 => {
                if let (Some(page), Some(image_element)) = (pages.get(page_index), &self.prefetcher)
                {
                    let link = self.link.clone();
                    let page_count = self.state.pages.as_ref().map_or(0, |pages| pages.len());
                    let next_page_index =
                        self.get_next_page_index(page_index.checked_add(1).unwrap_or(page_index));

                    if !self
                        .state
                        .is_loaded_page
                        .as_ref()
                        .map_or(true, |is_loaded_page| {
                            *is_loaded_page.get(next_page_index).unwrap_or(&true)
                        })
                    {
                        if next_page_index < page_count
                            && !self
                                .state
                                .is_loaded_page
                                .as_ref()
                                .map_or(true, |is_loaded_page| {
                                    *is_loaded_page.get(next_page_index).unwrap_or(&true)
                                })
                        {
                            let load_next_image = Closure::once(Box::new(move || {
                                link.send_message(Msg::PreloadNextImage {
                                    page_index: next_page_index,
                                });
                            }));
                            image_element
                                .set_onload(Some(load_next_image.as_ref().unchecked_ref()));
                            load_next_image.forget();
                        }
                        image_element.set_src(&page.url_string);
                    }

                    if let Some(Some(is_loaded)) = self
                        .state
                        .is_loaded_page
                        .as_mut()
                        .map(|is_loaded_page| is_loaded_page.get_mut(next_page_index))
                    {
                        *is_loaded = true;
                    }
                }
            }
            _ => {}
        };
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
                        Some(AppRoute::MangaChapter {
                            manga_id: self.props.manga_id,
                            chapter_number: self.props.chapter_number.to_owned(),
                        })
                    } else {
                        None
                    };

                self.state.is_loaded_page = Some(vec![false; pages.len()]);
                self.state.pages = Some(pages);
                if let Some(route) = route {
                    self.route_dispatcher
                        .send(RouteRequest::ChangeRoute(Route::from(route)));
                    false
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    fn page_backwards(&mut self) {
        let current_chapter_number = self.props.chapter_number.to_owned();
        let previous_page_chapter_number = if self.props.page_number == 1 {
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
            let route = AppRoute::MangaChapterPage {
                manga_id: self.props.manga_id,
                chapter_number: self.props.chapter_number.to_owned(),
                page_number: max(
                    1,
                    self.props
                        .page_number
                        .checked_sub(1)
                        .unwrap_or(self.props.page_number),
                ),
            };
            self.route_dispatcher
                .send(RouteRequest::ChangeRoute(Route::from(route)));
        }
    }

    fn page_forwards(&mut self) {
        let last_page = self
            .state
            .pages
            .as_ref()
            .map_or(self.props.page_number, |pages| pages.len());
        let current_chapter_number = self.props.chapter_number.to_owned();

        let next_page_chapter_number = if self.props.page_number == last_page {
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
            let next_page_number = self.props.page_number as usize + 1;
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
