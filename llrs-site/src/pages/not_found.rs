use crate::route::AppRoute;
use yew::prelude::*;
use yew_router::components::RouterAnchor;

pub(crate) fn not_found(path: &str) -> Html {
    html! {
        <RouterAnchor<AppRoute> route=AppRoute::MangaList>
            <div class="fixed-container flex-center">
                <div class="not-found">
                    {format!("Path not found {}", path)}
                </div>
            </div>
        </RouterAnchor<AppRoute>>
    }
}
