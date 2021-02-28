use yew::{prelude::*, virtual_dom::VNode};

pub struct Breadcrumb {
    props: Props,
}

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct Props {
    #[prop_or_default]
    pub children: Children,
    #[prop_or_default]
    pub separator: Separator,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum Separator {
    /// / - U+0002F
    ForwardSlash,
    /// → - U+02192
    Arrow,
    /// • - U+02022
    Bullet,
    /// · - U+000B7
    Dot,
    /// ≻ - U+0227B
    Succeeds,
}

impl Default for Separator {
    fn default() -> Self {
        Separator::ForwardSlash
    }
}

impl Separator {
    fn class_name(&self) -> &'static str {
        match self {
            Separator::ForwardSlash => "breadcrumb",
            Separator::Arrow => "breadcrumb has-arrow-separator",
            Separator::Bullet => "breadcrumb has-bullet-separator",
            Separator::Dot => "breadcrumb has-dot-separator",
            Separator::Succeeds => "breadcrumb has-succeeds-separator",
        }
    }
}

impl Component for Breadcrumb {
    type Message = ();

    type Properties = Props;

    fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self { props }
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props = props;
        true
    }

    fn view(&self) -> Html {
        let classes = self.props.separator.class_name();
        html! {
            <nav class=classes aria-label="breadcrumbs">
                <ul>
                    {for self.props.children.iter().map(|child| wrap_as_li(child))}
                </ul>
            </nav>
        }
    }
}

fn wrap_as_li(child: VNode) -> Html {
    html! {
        <li>{child}</li>
    }
}
