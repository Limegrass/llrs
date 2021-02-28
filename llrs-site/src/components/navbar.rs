use yew::prelude::*;

pub(crate) struct Navbar {
    link: ComponentLink<Self>,
    props: Props,
    state: State,
}
pub(super) struct State {
    is_menu_opened: bool,
}

#[derive(Debug, Clone, PartialEq, Properties)]
pub(crate) struct Props {
    /// These will not display without clicking the burger in mobile
    /// navbar-item or navbar-dropdown should be applied to the children
    #[prop_or_default]
    pub(crate) children: Children,
    /// These will always display in both mobile and desktop views
    /// navbar-item or navbar-dropdown should be applied to the children
    #[prop_or_default]
    pub(crate) brand_children: Children,
}

#[derive(Debug)]
pub(crate) enum Msg {
    BurgerClick,
}

impl Component for Navbar {
    type Message = Msg;

    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            props,
            link,
            state: State {
                is_menu_opened: false,
            },
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::BurgerClick => self.state.is_menu_opened = !self.state.is_menu_opened,
        }
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props = props;
        true
    }

    fn view(&self) -> Html {
        let (menu_classes, burger_classes) = if self.state.is_menu_opened {
            ("navbar-menu is-active", "navbar-burger is-active")
        } else {
            ("navbar-menu", "navbar-burger")
        };

        let menu_item_count = self.props.children.len();
        let menu_items = self.props.children.iter().collect::<Vec<Html>>();

        let menu_burger = if menu_item_count > 0 {
            html! {
                <a  role="button"
                    class=burger_classes
                    aria-label="menu"
                    aria-expanded="false"
                    onclick=self.link.callback(|_| Msg::BurgerClick)
                >
                    <span aria-hidden="true"></span>
                    <span aria-hidden="true"></span>
                    <span aria-hidden="true"></span>
                </a>
            }
        } else {
            html! {}
        };

        html! {
            <nav class="navbar is-transparent" role="navigation" aria-label="main navigation">
                <div class="navbar-brand">
                    {for self.props.brand_children.iter().map(|child| html!{
                        {child}
                     })}

                    {menu_burger}
                </div>
                <div class=menu_classes>
                    {menu_items}
                </div>
            </nav>
        }
    }
}
